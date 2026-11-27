//! The state explorer's views: `--list <view>` rows and `--show <view>
//! <key>` previews, over a redacted state.
//!
//! A row is `<key>\t<display>`: the key is what `--show` and the explorer
//! take back (snap name, connection key, change or task id, JSON Pointer),
//! the display is one field of padded, colored columns. An empty view is
//! one row with an empty key, `(none)`.

use super::{data, obj, py_str, status_name, width, Colors};
use serde_json::Value;

/// A key that isn't in the state.
pub struct NotFound(pub String);

type Out = Result<String, NotFound>;

const NONE_ROW: &str = "\t(none)\n";

/// snapd's change status priority, for a change whose own status is
/// Default (`overlord/state/change.go` statusOrder).
const STATUS_ORDER: &[i64] = &[5, 7, 6, 3, 2, 10, 9, 8, 4, 1];

fn task_status(t: &Value) -> i64 {
    match t.get("status").and_then(Value::as_i64).unwrap_or(0) {
        0 => 2, // Default is Do for a task
        s => s,
    }
}

fn change_tasks(ch: &Value) -> Vec<String> {
    ch.get("task-ids")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(py_str).collect())
        .unwrap_or_default()
}

/// The status snapd reports for a change: its own, else the aggregate of
/// its tasks' (Hold for none; Wait only when nothing can run).
pub fn change_status(state: &Value, ch: &Value) -> i64 {
    let own = ch.get("status").and_then(Value::as_i64).unwrap_or(0);
    if own != 0 {
        return own;
    }
    let tasks = obj(state.get("tasks"));
    let stats: Vec<i64> = change_tasks(ch)
        .iter()
        .filter_map(|id| tasks.and_then(|t| t.get(id)))
        .map(task_status)
        .collect();
    if stats.is_empty() {
        return 1;
    }
    if stats.contains(&10) && !stats.iter().any(|s| [2, 3, 5, 6, 7].contains(s)) {
        return 10;
    }
    STATUS_ORDER
        .iter()
        .copied()
        .find(|s| stats.contains(s))
        .unwrap_or(own)
}

fn status_color(c: &Colors, code: i64) -> &str {
    match code {
        4 => &c.green,
        9 => &c.red,
        _ => &c.yellow,
    }
}

/// A stored time as `YYYY-MM-DD HH:MM:SS`.
fn time(v: Option<&Value>) -> String {
    let s = v
        .filter(|v| super::truthy(v))
        .map(py_str)
        .unwrap_or_default();
    s.chars().take(19).collect::<String>().replace('T', " ")
}

fn one_line(s: &str) -> String {
    s.replace(['\t', '\n'], " ")
}

fn pad(s: &str, w: usize) -> String {
    format!("{s}{}", " ".repeat(w.saturating_sub(width(s))))
}

fn rpad(s: &str, w: usize) -> String {
    format!("{}{s}", " ".repeat(w.saturating_sub(width(s))))
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_default() + "\n"
}

/// ERROR lines of a task log (`<time> ERROR <message>`), as messages.
fn log_errors(t: &Value) -> Vec<String> {
    t.get("log")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            (parts.len() == 3 && parts[1] == "ERROR").then(|| parts[2].to_string())
        })
        .collect()
}

// ---- snaps ----------------------------------------------------------------

fn snap_channel(s: &Value) -> String {
    super::channel(s)
}

fn snap_type(s: &Value) -> String {
    s.get("type").map(py_str).unwrap_or_default()
}

pub fn list_snaps(state: &Value, c: &Colors) -> String {
    let snaps = super::snaps(state);
    if snaps.is_empty() {
        return NONE_ROW.into();
    }
    let rows: Vec<[String; 4]> = snaps
        .iter()
        .map(|(n, s)| {
            [
                one_line(n),
                s.get("current").map(py_str).unwrap_or_default(),
                one_line(&snap_channel(s)),
                snap_type(s),
            ]
        })
        .collect();
    let w: Vec<usize> = (0..3)
        .map(|i| rows.iter().map(|r| width(&r[i])).max().unwrap_or(0))
        .collect();
    let mut out = String::new();
    for (row, (name, _)) in rows.iter().zip(&snaps) {
        out += &format!(
            "{}\t{}{}{}  {}{}{}  {}  {}{}{}\n",
            one_line(name),
            c.cyan,
            pad(&row[0], w[0]),
            c.reset,
            c.dim,
            rpad(&row[1], w[1]),
            c.reset,
            pad(&row[2], w[2]),
            c.dim,
            row[3],
            c.reset
        );
    }
    out
}

pub fn show_snap(state: &Value, name: &str, c: &Colors) -> Out {
    let s = obj(data(state).and_then(|d| d.get("snaps")))
        .and_then(|m| m.get(name))
        .ok_or_else(|| NotFound(format!("no snap named {name}")))?;
    let current = s.get("current").map(py_str).unwrap_or_default();
    let active = s.get("active").is_some_and(super::truthy);
    let flags: Vec<&str> = ["classic", "devmode", "jailmode"]
        .into_iter()
        .filter(|f| s.get(*f).is_some_and(super::truthy))
        .collect();
    let mut out = format!(
        "{}{}{}{}  {}{}{}\n",
        c.bold,
        c.cyan,
        name,
        c.reset,
        c.dim,
        snap_type(s),
        c.reset
    );
    out += &format!(
        "  active:    {}\n",
        if active {
            format!("{}yes{}", c.green, c.reset)
        } else {
            format!("{}no{}", c.yellow, c.reset)
        }
    );
    out += &format!(
        "  flags:     {}\n",
        if flags.is_empty() {
            format!("{}none{}", c.dim, c.reset)
        } else {
            flags.join(", ")
        }
    );
    out += &format!("  channel:   {}\n", snap_channel(s));
    out += "  revisions:\n";
    let seq = s.get("sequence").and_then(Value::as_array);
    for si in seq.into_iter().flatten() {
        let rev = si.get("revision").map(py_str).unwrap_or_default();
        out += &if rev == current {
            format!(
                "    {}* {rev}{}  {}current{}\n",
                c.bold, c.reset, c.green, c.reset
            )
        } else {
            format!("      {}{rev}{}\n", c.dim, c.reset)
        };
    }
    out += &format!("\n{}State:{}\n", c.bold, c.reset);
    out += &pretty(s);
    Ok(out)
}

// ---- connections ----------------------------------------------------------

fn how(conn: &Value) -> String {
    let mut tags: Vec<&str> = Vec::new();
    let t = |k: &str| conn.get(k).is_some_and(super::truthy);
    if t("by-gadget") {
        tags.push("by-gadget");
    } else if t("auto") {
        tags.push("auto");
    }
    if t("hotplug-key") {
        tags.push(if t("hotplug-gone") {
            "hotplug (gone)"
        } else {
            "hotplug"
        });
    }
    if t("undesired") {
        tags.push("undesired");
    }
    if tags.is_empty() {
        "manual".into()
    } else {
        tags.join(", ")
    }
}

fn conn_sides(key: &str) -> (&str, &str) {
    key.split_once(' ').unwrap_or((key, ""))
}

pub fn list_connections(state: &Value, c: &Colors) -> String {
    let conns = obj(data(state).and_then(|d| d.get("conns")));
    let keys = super::conns(state);
    if keys.is_empty() {
        return NONE_ROW.into();
    }
    let side = |s: &str, w: usize| {
        let (snap, name) = s.split_once(':').unwrap_or((s, ""));
        format!(
            "{}{snap}{}{}:{name}{}{}",
            c.cyan,
            c.reset,
            c.dim,
            c.reset,
            " ".repeat(w.saturating_sub(width(s)))
        )
    };
    let rows: Vec<(&String, &str, &str, String, String)> = keys
        .iter()
        .map(|k| {
            let (p, s) = conn_sides(k);
            let v = conns.and_then(|m| m.get(*k)).unwrap_or(&Value::Null);
            (
                *k,
                p,
                s,
                one_line(&v.get("interface").map(py_str).unwrap_or_default()),
                how(v),
            )
        })
        .collect();
    let wp = rows.iter().map(|r| width(r.1)).max().unwrap_or(0);
    let ws = rows.iter().map(|r| width(r.2)).max().unwrap_or(0);
    let wi = rows.iter().map(|r| width(&r.3)).max().unwrap_or(0);
    let mut out = String::new();
    for (k, p, s, iface, how) in rows {
        out += &format!(
            "{}\t{}  {}→{}  {}  {}  {}{}{}\n",
            one_line(k),
            side(&one_line(p), wp),
            c.dim,
            c.reset,
            side(&one_line(s), ws),
            pad(&iface, wi),
            c.dim,
            how,
            c.reset
        );
    }
    out
}

pub fn show_connection(state: &Value, key: &str, c: &Colors) -> Out {
    let v = obj(data(state).and_then(|d| d.get("conns")))
        .and_then(|m| m.get(key))
        .ok_or_else(|| NotFound(format!("no connection {key}")))?;
    let (p, s) = conn_sides(key);
    let mut out = format!(
        "{}{p}{}  {}→{}  {}{s}{}\n",
        c.cyan, c.reset, c.dim, c.reset, c.cyan, c.reset
    );
    out += &format!(
        "  interface: {}\n",
        v.get("interface").map(py_str).unwrap_or_default()
    );
    out += &format!("  made:      {}\n", how(v));
    out += &format!("\n{}State:{}\n", c.bold, c.reset);
    out += &pretty(v);
    Ok(out)
}

// ---- changes and tasks ----------------------------------------------------

fn changes(state: &Value) -> Vec<(&String, &Value)> {
    let mut v: Vec<_> = obj(state.get("changes"))
        .map(|m| m.iter().collect())
        .unwrap_or_default();
    v.sort_by_key(|(id, _)| (id.parse::<i64>().unwrap_or(i64::MAX), (*id).clone()));
    v
}

fn task<'a>(state: &'a Value, id: &str) -> Option<&'a Value> {
    obj(state.get("tasks")).and_then(|t| t.get(id))
}

pub fn list_changes(state: &Value, c: &Colors) -> String {
    let rows: Vec<(String, i64, String, String, String)> = changes(state)
        .into_iter()
        .map(|(id, ch)| {
            let code = change_status(state, ch);
            let t = time(ch.get("ready-time")).to_string();
            let t = if t.is_empty() {
                time(ch.get("spawn-time"))
            } else {
                t
            };
            (
                one_line(id),
                code,
                status_name(code),
                t,
                one_line(&ch.get("summary").map(py_str).unwrap_or_default()),
            )
        })
        .collect();
    if rows.is_empty() {
        return NONE_ROW.into();
    }
    let wi = rows.iter().map(|r| width(&r.0)).max().unwrap_or(0);
    let ws = rows.iter().map(|r| width(&r.2)).max().unwrap_or(0);
    let wt = rows.iter().map(|r| width(&r.3)).max().unwrap_or(0);
    let mut out = String::new();
    for (id, code, st, t, summary) in rows {
        out += &format!(
            "{id}\t{}{}{}  {}{}{}  {}{}{}  {summary}\n",
            c.dim,
            rpad(&id, wi),
            c.reset,
            status_color(c, code),
            pad(&st, ws),
            c.reset,
            c.dim,
            pad(&t, wt),
            c.reset
        );
    }
    out
}

fn task_rows(state: &Value, ids: &[String], c: &Colors, errors: bool, indent: &str) -> String {
    let rows: Vec<(String, i64, String, String, String, Vec<String>)> = ids
        .iter()
        .map(|id| {
            let t = task(state, id).unwrap_or(&Value::Null);
            let code = task_status(t);
            (
                one_line(id),
                code,
                status_name(code),
                one_line(&t.get("kind").map(py_str).unwrap_or_default()),
                one_line(&t.get("summary").map(py_str).unwrap_or_default()),
                if errors { log_errors(t) } else { Vec::new() },
            )
        })
        .collect();
    let wi = rows.iter().map(|r| width(&r.0)).max().unwrap_or(0);
    let ws = rows.iter().map(|r| width(&r.2)).max().unwrap_or(0);
    let wk = rows.iter().map(|r| width(&r.3)).max().unwrap_or(0);
    let mut out = String::new();
    for (id, code, st, kind, summary, errs) in rows {
        if indent.is_empty() {
            out += &format!("{id}\t");
        } else {
            out += indent;
        }
        out += &format!(
            "{}{}{}  {}{}{}  {}  {summary}\n",
            c.dim,
            rpad(&id, wi),
            c.reset,
            status_color(c, code),
            pad(&st, ws),
            c.reset,
            pad(&kind, wk)
        );
        for e in errs {
            out += &format!(
                "{indent}{}  {}↳ {}{}\n",
                " ".repeat(wi),
                c.red,
                one_line(&e),
                c.reset
            );
        }
    }
    out
}

fn get_change<'a>(state: &'a Value, id: &str) -> Result<&'a Value, NotFound> {
    obj(state.get("changes"))
        .and_then(|m| m.get(id))
        .ok_or_else(|| NotFound(format!("no change {id}")))
}

pub fn list_tasks(state: &Value, change: &str, c: &Colors) -> Out {
    let ids = change_tasks(get_change(state, change)?);
    if ids.is_empty() {
        return Ok(NONE_ROW.into());
    }
    Ok(task_rows(state, &ids, c, false, ""))
}

pub fn show_change(state: &Value, id: &str, c: &Colors) -> Out {
    let ch = get_change(state, id)?;
    let code = change_status(state, ch);
    let mut out = format!(
        "{}{id}{}  {}{}{}  {}{}{}\n",
        c.bold,
        c.reset,
        status_color(c, code),
        status_name(code),
        c.reset,
        c.dim,
        ch.get("kind").map(py_str).unwrap_or_default(),
        c.reset
    );
    out += &format!("{}\n", ch.get("summary").map(py_str).unwrap_or_default());
    out += &format!("  spawned: {}\n", time(ch.get("spawn-time")));
    out += &format!("  ready:   {}\n", time(ch.get("ready-time")));
    out += &format!("\n{}Tasks:{}\n", c.bold, c.reset);
    let ids = change_tasks(ch);
    if ids.is_empty() {
        out += "  (none)\n";
    } else {
        out += &task_rows(state, &ids, c, true, "  ");
    }
    Ok(out)
}

pub fn show_task(state: &Value, id: &str, c: &Colors) -> Out {
    let t = task(state, id).ok_or_else(|| NotFound(format!("no task {id}")))?;
    let code = task_status(t);
    let mut out = format!(
        "{}{id}{}  {}{}{}  {}{}{}\n",
        c.bold,
        c.reset,
        status_color(c, code),
        status_name(code),
        c.reset,
        c.cyan,
        t.get("kind").map(py_str).unwrap_or_default(),
        c.reset
    );
    out += &format!("{}\n", t.get("summary").map(py_str).unwrap_or_default());
    out += &format!(
        "  change:    {}\n",
        t.get("change").map(py_str).unwrap_or_default()
    );
    out += &format!("  spawned:   {}\n", time(t.get("spawn-time")));
    out += &format!("  ready:     {}\n", time(t.get("ready-time")));
    let waits: Vec<String> = t
        .get("wait-tasks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|w| {
            let w = py_str(w);
            match task(state, &w).and_then(|t| t.get("kind")) {
                Some(k) => format!("{w} ({})", py_str(k)),
                None => w,
            }
        })
        .collect();
    out += &format!(
        "  waits for: {}\n",
        if waits.is_empty() {
            format!("{}nothing{}", c.dim, c.reset)
        } else {
            waits.join(", ")
        }
    );
    let names: Vec<&String> = obj(t.get("data"))
        .map(|m| m.keys().collect())
        .unwrap_or_default();
    out += &format!(
        "  data:      {}\n",
        if names.is_empty() {
            format!("{}none{}", c.dim, c.reset)
        } else {
            names
                .iter()
                .map(|n| n.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    out += &format!("\n{}Log:{}\n", c.bold, c.reset);
    let log = t.get("log").and_then(Value::as_array);
    let mut any = false;
    for line in log.into_iter().flatten().map(py_str) {
        any = true;
        let error = line.split(' ').nth(1) == Some("ERROR");
        out += &if error {
            format!("  {}{}{}\n", c.red, line, c.reset)
        } else {
            format!("  {line}\n")
        };
    }
    if !any {
        out += "  (none)\n";
    }
    Ok(out)
}

// ---- tree -----------------------------------------------------------------

/// One RFC 6901 reference token.
pub fn pointer_token(s: &str) -> String {
    s.replace('~', "~0").replace('/', "~1")
}

/// A member name as the dotted display shows it.
fn display_member(name: &str, first: bool) -> String {
    let plain = !name.is_empty()
        && !name
            .chars()
            .any(|ch| matches!(ch, ' ' | ':' | '.' | '[' | ']' | '"' | '\\') || ch.is_control());
    if plain {
        if first {
            name.into()
        } else {
            format!(".{name}")
        }
    } else {
        format!("[{}]", Value::String(name.into()))
    }
}

/// The dotted display of a JSON Pointer.
fn display_path(state: &Value, pointer: &str) -> String {
    let mut out = String::new();
    let mut node = state;
    if pointer.is_empty() {
        return "(root)".into();
    }
    for tok in pointer[1..].split('/') {
        let tok = tok.replace("~1", "/").replace("~0", "~");
        match node {
            Value::Array(a) => {
                out += &format!("[{tok}]");
                node = tok
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| a.get(i))
                    .unwrap_or(&Value::Null);
            }
            _ => {
                out += &display_member(&tok, out.is_empty());
                node = node.get(&tok).unwrap_or(&Value::Null);
            }
        }
    }
    out
}

fn summary(v: &Value) -> String {
    match v {
        Value::Object(o) => format!("{{{}}}", o.len()),
        Value::Array(a) => format!("[{}]", a.len()),
        other => {
            let s = other.to_string();
            if width(&s) > 60 {
                s.chars().take(59).collect::<String>() + "…"
            } else {
                s
            }
        }
    }
}

fn tree_rows(v: &Value, pointer: &mut String, display: &mut String, c: &Colors, out: &mut String) {
    let (p_len, d_len) = (pointer.len(), display.len());
    let child = |pointer: &mut String, display: &mut String, out: &mut String, item: &Value| {
        out.push_str(&format!(
            "{pointer}\t{display}  {}{}{}\n",
            c.dim,
            summary(item),
            c.reset
        ));
        tree_rows(item, pointer, display, c, out);
        pointer.truncate(p_len);
        display.truncate(d_len);
    };
    match v {
        Value::Object(o) => {
            for (k, item) in o {
                pointer.push('/');
                pointer.push_str(&pointer_token(k));
                let first = display.is_empty();
                display.push_str(&display_member(k, first));
                child(pointer, display, out, item);
            }
        }
        Value::Array(a) => {
            for (i, item) in a.iter().enumerate() {
                pointer.push_str(&format!("/{i}"));
                display.push_str(&format!("[{i}]"));
                child(pointer, display, out, item);
            }
        }
        _ => {}
    }
}

pub fn list_tree(state: &Value, pointer: &str, c: &Colors) -> Out {
    let node = state
        .pointer(pointer)
        .ok_or_else(|| NotFound(format!("no path {pointer}")))?;
    let mut out = String::new();
    let mut p = pointer.to_string();
    let mut d = if pointer.is_empty() {
        String::new()
    } else {
        let d = display_path(state, pointer);
        out += &format!("{pointer}\t{d}  {}{}{}\n", c.dim, summary(node), c.reset);
        d
    };
    tree_rows(node, &mut p, &mut d, c, &mut out);
    if out.is_empty() {
        return Ok(NONE_ROW.into());
    }
    Ok(out)
}

pub fn show_path(state: &Value, pointer: &str, c: &Colors) -> Out {
    let node = state
        .pointer(pointer)
        .ok_or_else(|| NotFound(format!("no path {pointer}")))?;
    Ok(format!(
        "{}{}{}\n{}",
        c.bold,
        display_path(state, pointer),
        c.reset,
        pretty(node)
    ))
}
