//! `snapd-state`: read snapd's state.json, as `inspect image`, `inspect
//! diff` and `inspect state` need it.
//!
//! Every mode redacts the state right after parsing it (see [`redact`]),
//! so nothing it prints holds a macaroon, cookie or password.
//!
//! Records (the default), one per line, tab-separated:
//!   preseed <yes|no>
//!   snap    <name> <revision> <channel>
//!   conn    <plug-snap:plug> <slot-snap:slot>
//! snaps and connections sorted by name. Tabs and newlines in a value become
//! spaces (snapd's names never hold them).
//!
//! `--report snaps|connections|changes` prints that section of
//! `inspect image`'s report instead, colored from CYAN, DIM, RESET, GREEN,
//! YELLOW and RED in the environment. The padding counts characters, which
//! the shell can't do portably (mawk and printf count bytes).
//!
//! `--redact <in|-> <out>` writes the redacted state as indented JSON to
//! `<out>` (a 0600 temporary file beside it, then renamed) and prints
//! `redacted <n>`, the number of values replaced. The input must be a
//! JSON object with a `data` object.
//!
//! `--list snaps|connections|changes|tree`, `--list tasks <change-id>` and
//! `--list tree <pointer>` print the explorer's rows (see [`views`]);
//! `--show snap|connection|change|task|path <key>` prints a preview. Both
//! color as `--report` does, plus BOLD.
//!
//! Exit status: 0 ok, 1 a key that isn't in the state, 2 usage or an
//! unreadable or invalid file.

mod redact;
mod views;

use serde_json::{Map, Value};
use std::io::{Read, Write};

const USAGE: &str =
    "usage: idiot-helper snapd-state [--report snaps|connections|changes] <state.json>
       idiot-helper snapd-state --redact <state.json|-> <out>
       idiot-helper snapd-state --list snaps|connections|changes|tree <state.json>
       idiot-helper snapd-state --list tasks <change-id> <state.json>
       idiot-helper snapd-state --list tree <pointer> <state.json>
       idiot-helper snapd-state --show snap|connection|change|task|path <key> <state.json>";

enum Mode<'a> {
    Records,
    Report(&'a str),
    List(&'a str, Option<&'a str>),
    Show(&'a str, &'a str),
}

fn usage() -> u8 {
    eprintln!("{USAGE}");
    2
}

pub fn run(args: &[String]) -> u8 {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let (mode, path) = match a.as_slice() {
        ["--redact", input, out] => return redact_to(input, out),
        [p] if !p.starts_with('-') => (Mode::Records, *p),
        ["--report", section, p] => (Mode::Report(section), *p),
        ["--list", view @ ("snaps" | "connections" | "changes" | "tree"), p] => {
            (Mode::List(view, None), *p)
        }
        ["--list", view @ ("tasks" | "tree"), key, p] => (Mode::List(view, Some(key)), *p),
        ["--list", view, ..] => {
            eprintln!("idiot-helper snapd-state: unknown view: {view}");
            return 2;
        }
        ["--show", view, key, p] => (Mode::Show(view, key), *p),
        _ => return usage(),
    };
    let mut state = match load(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("idiot-helper: {path}: {e}");
            return 2;
        }
    };
    redact::redact(&mut state);
    let c = Colors::from_env();
    let out = match mode {
        Mode::Records => Ok(records(&state)),
        Mode::Report("snaps") => Ok(report_snaps(&state, &c)),
        Mode::Report("connections") => Ok(report_connections(&state, &c)),
        Mode::Report("changes") => Ok(report_changes(&state, &c)),
        Mode::Report(other) => {
            eprintln!("idiot-helper snapd-state: unknown report: {other}");
            return 2;
        }
        Mode::List("snaps", _) => Ok(views::list_snaps(&state, &c)),
        Mode::List("connections", _) => Ok(views::list_connections(&state, &c)),
        Mode::List("changes", _) => Ok(views::list_changes(&state, &c)),
        Mode::List("tasks", Some(id)) => views::list_tasks(&state, id, &c),
        Mode::List(_, key) => views::list_tree(&state, key.unwrap_or(""), &c),
        Mode::Show("snap", k) => views::show_snap(&state, k, &c),
        Mode::Show("connection", k) => views::show_connection(&state, k, &c),
        Mode::Show("change", k) => views::show_change(&state, k, &c),
        Mode::Show("task", k) => views::show_task(&state, k, &c),
        Mode::Show("path", k) => views::show_path(&state, k, &c),
        Mode::Show(other, _) => {
            eprintln!("idiot-helper snapd-state: unknown view: {other}");
            return 2;
        }
    };
    let out = match out {
        Ok(s) => s,
        Err(views::NotFound(msg)) => {
            eprintln!("idiot-helper snapd-state: {msg}");
            return 1;
        }
    };
    let mut stdout = std::io::stdout().lock();
    if stdout.write_all(out.as_bytes()).is_err() {
        return 2;
    }
    0
}

/// Read a state file, or stdin for `-`.
fn read_input(path: &str) -> Result<Vec<u8>, String> {
    if path == "-" {
        let mut b = Vec::new();
        std::io::stdin()
            .read_to_end(&mut b)
            .map_err(|e| e.to_string())?;
        Ok(b)
    } else {
        std::fs::read(path).map_err(|e| e.to_string())
    }
}

fn load(path: &str) -> Result<Value, String> {
    let bytes = read_input(path)?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

/// How an input is named in messages.
fn label(input: &str) -> &str {
    if input == "-" {
        "stdin"
    } else {
        input
    }
}

/// `--redact`: write the redacted state to `out`, readable by the user
/// only, through a temporary file renamed over it.
fn redact_to(input: &str, out: &str) -> u8 {
    match read_input(input) {
        Ok(b) => redact_bytes(&b, input, out),
        Err(e) => {
            eprintln!("idiot-helper: {}: {e}", label(input));
            2
        }
    }
}

fn redact_bytes(bytes: &[u8], input: &str, out: &str) -> u8 {
    let parsed: Result<Value, String> = serde_json::from_slice(bytes).map_err(|e| e.to_string());
    let mut state = match parsed {
        Ok(v) if v.get("data").is_some_and(Value::is_object) => v,
        Ok(_) => {
            eprintln!(
                "idiot-helper: {}: not snapd state (no data object)",
                label(input)
            );
            return 2;
        }
        Err(e) => {
            eprintln!("idiot-helper: {}: {e}", label(input));
            return 2;
        }
    };
    let n = redact::redact(&mut state);
    let mut json = serde_json::to_string_pretty(&state).unwrap_or_default();
    json.push('\n');
    if let Err(e) = write_private(std::path::Path::new(out), json.as_bytes()) {
        eprintln!("idiot-helper: {out}: {e}");
        return 2;
    }
    println!("redacted {n}");
    0
}

fn write_private(out: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let dir = match out.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => std::path::Path::new("."),
    };
    let base = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "state.json".into());
    let mut i = 0u32;
    let (tmp, mut f) = loop {
        let tmp = dir.join(format!(".{base}.{}.{i}", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)
        {
            Ok(f) => break (tmp, f),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && i < 100 => i += 1,
            Err(e) => return Err(e),
        }
    };
    let res = f
        .write_all(bytes)
        .and_then(|_| f.sync_all())
        .and_then(|_| std::fs::rename(&tmp, out));
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

struct Colors {
    bold: String,
    cyan: String,
    dim: String,
    reset: String,
    green: String,
    yellow: String,
    red: String,
}

impl Colors {
    fn from_env() -> Self {
        let get = |k: &str| std::env::var(k).unwrap_or_default();
        Colors {
            bold: get("BOLD"),
            cyan: get("CYAN"),
            dim: get("DIM"),
            reset: get("RESET"),
            green: get("GREEN"),
            yellow: get("YELLOW"),
            red: get("RED"),
        }
    }
}

/// `str(v)` as Python prints a JSON value.
fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Null => "None".into(),
        other => other.to_string(),
    }
}

/// Python's truthiness of a JSON value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn obj(v: Option<&Value>) -> Option<&Map<String, Value>> {
    v.and_then(Value::as_object)
}

fn data(state: &Value) -> Option<&Map<String, Value>> {
    obj(state.get("data"))
}

fn snaps(state: &Value) -> Vec<(&String, &Value)> {
    let mut v: Vec<_> = obj(data(state).and_then(|d| d.get("snaps")))
        .map(|m| m.iter().collect())
        .unwrap_or_default();
    v.sort_by(|a, b| a.0.cmp(b.0));
    v
}

fn conns(state: &Value) -> Vec<&String> {
    let mut v: Vec<_> = obj(data(state).and_then(|d| d.get("conns")))
        .map(|m| m.keys().collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// tracking-channel, else channel, else "" (Python's `a or b`).
fn channel(s: &Value) -> String {
    match s.get("tracking-channel") {
        Some(t) if truthy(t) => py_str(t),
        _ => s.get("channel").map(py_str).unwrap_or_default(),
    }
}

fn field(s: &str) -> String {
    s.replace(['\t', '\n'], " ")
}

fn records(state: &Value) -> String {
    let preseed = data(state)
        .and_then(|d| d.get("preseed"))
        .is_some_and(truthy);
    let mut out = format!("preseed\t{}\n", if preseed { "yes" } else { "no" });
    for (name, s) in snaps(state) {
        let rev = s.get("current").map(py_str).unwrap_or_default();
        out += &format!(
            "snap\t{}\t{}\t{}\n",
            field(name),
            field(&rev),
            field(&channel(s))
        );
    }
    for key in conns(state) {
        let (plug, slot) = key.split_once(' ').unwrap_or((key, ""));
        out += &format!("conn\t{}\t{}\n", field(plug), field(slot));
    }
    out
}

fn width(s: &str) -> usize {
    s.chars().count()
}

fn report_snaps(state: &Value, c: &Colors) -> String {
    let snaps = snaps(state);
    if snaps.is_empty() {
        return "  (none)\n".into();
    }
    let max_name = snaps.iter().map(|(n, _)| width(n)).max().unwrap_or(0);
    let max_rev = snaps
        .iter()
        .map(|(_, s)| width(&s.get("current").map(py_str).unwrap_or_default()))
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (name, s) in snaps {
        let rev = s.get("current").map(py_str).unwrap_or_else(|| "?".into());
        let chan = channel(s);
        let suffix = if chan.is_empty() {
            String::new()
        } else {
            format!("  {chan}")
        };
        out += &format!(
            "  {}{:<max_name$}{}  {}{:>max_rev$}{}{}\n",
            c.cyan, name, c.reset, c.dim, rev, suffix, c.reset
        );
    }
    out
}

fn report_connections(state: &Value, c: &Colors) -> String {
    let keys = conns(state);
    if keys.is_empty() {
        return "  (none)\n".into();
    }
    let pairs: Vec<(&str, &str)> = keys
        .iter()
        .map(|k| k.split_once(' ').unwrap_or((k.as_str(), "")))
        .collect();
    let side = |s: &str| {
        let (snap, iface) = s.split_once(':').unwrap_or((s, ""));
        format!("{}{snap}{}{}:{iface}{}", c.cyan, c.reset, c.dim, c.reset)
    };
    let max_plug = pairs.iter().map(|(p, _)| width(p)).max().unwrap_or(0);
    let mut out = String::new();
    for (plug, slot) in pairs {
        let pad = " ".repeat(max_plug - width(plug));
        out += &format!(
            "  {}{pad}  {}→{}  {}\n",
            side(plug),
            c.dim,
            c.reset,
            side(slot)
        );
    }
    out
}

fn status_name(code: i64) -> String {
    match code {
        0 => "Default",
        1 => "Hold",
        2 => "Do",
        3 => "Doing",
        4 => "Done",
        5 => "Abort",
        6 => "Undo",
        7 => "Undoing",
        8 => "Undone",
        9 => "Error",
        10 => "Wait",
        _ => return code.to_string(),
    }
    .into()
}

fn report_changes(state: &Value, c: &Colors) -> String {
    let empty = Map::new();
    let changes = obj(state.get("changes")).unwrap_or(&empty);
    let tasks = obj(state.get("tasks")).unwrap_or(&empty);
    if changes.is_empty() {
        return "  (none)\n".into();
    }
    let mut ids: Vec<&String> = changes.keys().collect();
    ids.sort_by_key(|id| id.parse::<i64>().unwrap_or(i64::MAX));
    struct Row<'a> {
        id: &'a str,
        code: i64,
        status: String,
        time: String,
        summary: String,
        tasks: Vec<String>,
    }
    let rows: Vec<Row> = ids
        .iter()
        .map(|id| {
            let ch = &changes[*id];
            let code = views::change_status(state, ch);
            let time = ["ready-time", "spawn-time"]
                .iter()
                .filter_map(|k| ch.get(*k))
                .find(|v| truthy(v))
                .map(py_str)
                .unwrap_or_default();
            let time: String = time.chars().take(19).collect::<String>().replace('T', " ");
            Row {
                id,
                code,
                status: status_name(code),
                time,
                summary: ch.get("summary").map(py_str).unwrap_or_default(),
                tasks: ch
                    .get("task-ids")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().map(py_str).collect())
                    .unwrap_or_default(),
            }
        })
        .collect();
    let max_id = rows.iter().map(|r| width(r.id)).max().unwrap_or(0);
    let max_st = rows.iter().map(|r| width(&r.status)).max().unwrap_or(0);
    let max_ts = rows.iter().map(|r| width(&r.time)).max().unwrap_or(0);
    let mut out = String::new();
    for r in &rows {
        let sc = match r.code {
            4 => &c.green,
            9 => &c.red,
            _ => &c.yellow,
        };
        out += &format!(
            "  {}{:>max_id$}{}  {sc}{:<max_st$}{}  {}{:<max_ts$}{}  {}\n",
            c.dim, r.id, c.reset, r.status, c.reset, c.dim, r.time, c.reset, r.summary
        );
        if r.code != 9 {
            continue;
        }
        for tid in &r.tasks {
            let log = tasks
                .get(tid)
                .and_then(|t| t.get("log"))
                .and_then(Value::as_array);
            for line in log.into_iter().flatten().filter_map(Value::as_str) {
                let parts: Vec<&str> = line.splitn(3, ' ').collect();
                if parts.len() == 3 && parts[1] == "ERROR" {
                    out += &format!(
                        "       {}  {}  {}  {}↳ {}{}\n",
                        " ".repeat(max_id),
                        " ".repeat(max_st),
                        " ".repeat(max_ts),
                        c.red,
                        parts[2],
                        c.reset
                    );
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Colors {
        Colors {
            bold: String::new(),
            cyan: String::new(),
            dim: String::new(),
            reset: String::new(),
            green: String::new(),
            yellow: String::new(),
            red: String::new(),
        }
    }

    fn sample() -> Value {
        serde_json::from_str(
            r#"{"data":{"preseed":true,
              "snaps":{"pc":{"current":"x1","channel":"24/stable"},
                       "snapd":{"current":"21759","tracking-channel":"latest/edge","channel":"x"},
                       "Ünï":{"current":12}},
              "conns":{"pc:p core24:s":{},"a\tb:n c:n":{}}},
             "changes":{"10":{"status":4,"ready-time":"2026-11-01T10:05:00.5Z","summary":"Init","task-ids":["1"]},
                        "2":{"status":9,"spawn-time":"2026-11-02T11:00:00Z","summary":"Install","task-ids":["2"]}},
             "tasks":{"2":{"log":["t INFO x","t ERROR it broke","bad"]}}}"#,
        )
        .unwrap()
    }

    #[test]
    fn records_sorted_and_cleaned() {
        assert_eq!(
            records(&sample()),
            "preseed\tyes\n\
             snap\tpc\tx1\t24/stable\n\
             snap\tsnapd\t21759\tlatest/edge\n\
             snap\tÜnï\t12\t\n\
             conn\ta b:n\tc:n\n\
             conn\tpc:p\tcore24:s\n"
        );
    }

    #[test]
    fn no_preseed_or_data() {
        assert_eq!(records(&serde_json::json!({})), "preseed\tno\n");
        assert_eq!(report_snaps(&serde_json::json!({}), &plain()), "  (none)\n");
    }

    #[test]
    fn snaps_pad_by_characters() {
        assert_eq!(
            report_snaps(&sample(), &plain()),
            "  pc        x1  24/stable\n  snapd  21759  latest/edge\n  Ünï       12\n"
        );
    }

    #[test]
    fn changes_numeric_order_and_errors() {
        let r = report_changes(&sample(), &plain());
        let lines: Vec<&str> = r.lines().collect();
        assert_eq!(lines[0], "   2  Error  2026-11-02 11:00:00  Install");
        assert_eq!(
            lines[1],
            "                                       ↳ it broke"
        );
        assert_eq!(lines[2], "  10  Done   2026-11-01 10:05:00  Init");
    }

    // ---- the synthetic fixture: one of each secret, as a marker ----------

    const FIXTURE: &str = include_str!("testdata/state.json");
    const MACAROON_MARKER: &str = "MDAxMKR-MACAROON-SHAPED-0001";

    fn fixture() -> Value {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        redact::redact(&mut v);
        v
    }

    fn colors() -> Colors {
        let mut c = plain();
        c.red = "<R>".into();
        c.reset = "</>".into();
        c
    }

    fn no_marker(what: &str, out: &str) {
        assert!(!out.contains("MKR-"), "{what} leaks a marker:\n{out}");
        assert!(!out.contains(MACAROON_MARKER), "{what} leaks the macaroon");
    }

    /// Every output mode, over every key the views offer.
    fn every_output(state: &Value) -> Vec<(String, String)> {
        let c = colors();
        let mut outs = vec![
            ("records".into(), records(state)),
            ("report snaps".into(), report_snaps(state, &c)),
            ("report connections".into(), report_connections(state, &c)),
            ("report changes".into(), report_changes(state, &c)),
            ("json".into(), serde_json::to_string_pretty(state).unwrap()),
        ];
        let lists = [
            ("snaps", views::list_snaps(state, &c)),
            ("connections", views::list_connections(state, &c)),
            ("changes", views::list_changes(state, &c)),
            ("tree", views::list_tree(state, "", &c).ok().unwrap()),
        ];
        for (view, rows) in lists {
            for key in rows.lines().map(|l| l.split('\t').next().unwrap()) {
                let shown = match view {
                    "snaps" => views::show_snap(state, key, &c),
                    "connections" => views::show_connection(state, key, &c),
                    "changes" => {
                        let t = views::list_tasks(state, key, &c).ok().unwrap();
                        outs.push((format!("tasks {key}"), t.clone()));
                        for tk in t.lines().map(|l| l.split('\t').next().unwrap()) {
                            if !tk.is_empty() {
                                let s = views::show_task(state, tk, &c).ok().unwrap();
                                outs.push((format!("task {tk}"), s));
                            }
                        }
                        views::show_change(state, key, &c)
                    }
                    _ => views::show_path(state, key, &c),
                };
                outs.push((format!("show {view} {key}"), shown.ok().unwrap()));
            }
            outs.push((format!("list {view}"), rows));
        }
        outs
    }

    #[test]
    fn fixture_holds_the_markers() {
        // guards the test itself: the raw fixture has every secret
        assert_eq!(FIXTURE.matches("MKR-").count(), 16);
        assert!(FIXTURE.contains(MACAROON_MARKER));
    }

    #[test]
    fn no_secret_in_any_output() {
        let state = fixture();
        for (what, out) in every_output(&state) {
            no_marker(&what, &out);
        }
    }

    #[test]
    fn redaction_count_and_kept_values() {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(redact::redact(&mut v), 16);
        let s = serde_json::to_string(&v).unwrap();
        for kept in [
            "dev@example.com",
            "SERIAL-0001",
            "KEYID-0001",
            "HOTPLUG-KEY-0001",
            "\"instance-key\":\"two\"",
            "db.local",
            "@proxy:3128",
        ] {
            assert!(s.contains(kept), "{kept} was redacted");
        }
        let users = &v["data"]["auth"]["users"][0];
        assert_eq!(users["macaroon"], "<redacted>");
        assert_eq!(
            users["discharges"],
            serde_json::json!(["<redacted>", "<redacted>"])
        );
        assert_eq!(
            v["data"]["auth"]["device"]["session-macaroon"],
            "<redacted>"
        );
        assert_eq!(
            v["data"]["snap-cookies"],
            serde_json::json!({"<redacted-1>": "<redacted>", "<redacted-2>": "<redacted>"})
        );
        assert_eq!(
            v["data"]["config"]["core"]["proxy"]["http"],
            "http://<redacted>@proxy:3128"
        );
        assert_eq!(
            v["data"]["config"]["my-app"]["db"]["password"],
            "<redacted>"
        );
        assert_eq!(v["data"]["config"]["my-app"]["colour"], "blue");
    }

    #[test]
    fn records_unchanged_by_redaction() {
        let raw: Value = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(records(&raw), records(&fixture()));
        let mut s = sample();
        let before = records(&s);
        redact::redact(&mut s);
        assert_eq!(records(&s), before);
    }

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("idiot-helper-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir(&d).unwrap();
        d
    }

    #[test]
    fn redact_writes_private_files() {
        use std::os::unix::fs::PermissionsExt;
        let d = tmpdir("redact");
        let input = d.join("in.json");
        std::fs::write(&input, FIXTURE).unwrap();
        let out = d.join("out.json");
        std::fs::write(&out, "old").unwrap();
        std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(redact_to(input.to_str().unwrap(), out.to_str().unwrap()), 0);
        let meta = std::fs::metadata(&out).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        let written = std::fs::read_to_string(&out).unwrap();
        no_marker("--redact", &written);
        let v: Value = serde_json::from_str(&written).unwrap();
        assert_eq!(v["data"]["auth"]["users"][0]["email"], "dev@example.com");
        // nothing left beside it
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 2);
        // not state
        std::fs::write(&input, "[1]").unwrap();
        assert_eq!(redact_to(input.to_str().unwrap(), out.to_str().unwrap()), 2);
        std::fs::write(&input, "{\"data\": 1}").unwrap();
        assert_eq!(redact_to(input.to_str().unwrap(), out.to_str().unwrap()), 2);
        std::fs::write(&input, "not json").unwrap();
        assert_eq!(redact_to(input.to_str().unwrap(), out.to_str().unwrap()), 2);
        assert_eq!(
            redact_to(d.join("missing").to_str().unwrap(), out.to_str().unwrap()),
            2
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn redact_from_bytes() {
        let d = tmpdir("bytes");
        let out = d.join("s.json");
        assert_eq!(
            redact_bytes(FIXTURE.as_bytes(), "-", out.to_str().unwrap()),
            0
        );
        no_marker("bytes", &std::fs::read_to_string(&out).unwrap());
        let _ = std::fs::remove_dir_all(&d);
    }

    // ---- views -----------------------------------------------------------

    fn rows(s: &str) -> Vec<(&str, &str)> {
        s.lines().map(|l| l.split_once('\t').unwrap()).collect()
    }

    #[test]
    fn snaps_view() {
        let out = views::list_snaps(&fixture(), &plain());
        let r = rows(&out);
        let keys: Vec<&str> = r.iter().map(|r| r.0).collect();
        assert_eq!(
            keys,
            ["core24", "my-app", "my-app_two", "pc", "pc-kernel", "snapd"]
        );
        assert_eq!(r[4].1, "pc-kernel    1900  24/stable      kernel");
        assert_eq!(r[1].1, "my-app         x1                 app");
    }

    #[test]
    fn kept_revisions() {
        let s = views::show_snap(&fixture(), "pc-kernel", &plain())
            .ok()
            .unwrap();
        assert!(
            s.contains("      1801\n      1855\n    * 1900  current\n"),
            "{s}"
        );
        assert!(s.contains("  active:    yes\n"));
        let s = views::show_snap(&fixture(), "my-app", &plain())
            .ok()
            .unwrap();
        assert!(s.contains("  flags:     devmode\n"), "{s}");
        assert!(views::show_snap(&fixture(), "nope", &plain()).is_err());
    }

    #[test]
    fn connections_view() {
        let out = views::list_connections(&fixture(), &plain());
        let r = rows(&out);
        let got: Vec<(&str, &str)> = r
            .iter()
            .map(|(k, d)| (*k, d.rsplit("  ").next().unwrap()))
            .collect();
        assert_eq!(
            got,
            [
                ("my-app:camera core:camera", "manual"),
                ("my-app:gpio pc:gpio-1", "by-gadget"),
                ("my-app:home core:home", "auto, undesired"),
                ("my-app:network core:network", "auto"),
                ("my-app:serial core:usb-serial", "hotplug"),
            ]
        );
        assert!(
            r[1].1
                .starts_with("my-app:gpio     →  pc:gpio-1        gpio"),
            "{}",
            r[1].1
        );
    }

    #[test]
    fn changes_view() {
        let out = views::list_changes(&fixture(), &plain());
        let r = rows(&out);
        assert_eq!(r.iter().map(|r| r.0).collect::<Vec<_>>(), ["1", "7", "10"]);
        assert_eq!(
            r[0].1,
            " 1  Done   2026-11-20 09:01:30  Initialize system state"
        );
        assert_eq!(
            r[1].1,
            " 7  Error  2026-11-21 10:00:09  Install \"my-app\" snap from file"
        );
        assert_eq!(
            r[2].1,
            "10  Done   2026-11-22 10:00:00  Refresh \"pc-kernel\" snap"
        );
        let empty = serde_json::json!({"data": {}});
        assert_eq!(views::list_changes(&empty, &plain()), "\t(none)\n");
        assert_eq!(views::list_snaps(&empty, &plain()), "\t(none)\n");
        assert_eq!(views::list_connections(&empty, &plain()), "\t(none)\n");
    }

    #[test]
    fn tasks_of_a_change() {
        let out = views::list_tasks(&fixture(), "7", &plain()).ok().unwrap();
        assert_eq!(
            rows(&out),
            [
                (
                    "3",
                    "3  Done   prerequisites  Ensure prerequisites for \"my-app\" are available"
                ),
                (
                    "4",
                    "4  Error  run-hook       Run install hook of \"my-app\" snap if present"
                ),
                (
                    "5",
                    "5  Hold   link-snap      Make snap \"my-app\" available to the system"
                ),
            ]
        );
        assert_eq!(
            views::list_tasks(&fixture(), "10", &plain()).ok().unwrap(),
            "\t(none)\n"
        );
        assert!(views::list_tasks(&fixture(), "99", &plain()).is_err());
        let ch = views::show_change(&fixture(), "7", &colors()).ok().unwrap();
        assert!(
            ch.contains("<R>↳ run hook \"install\": exit status 1</>"),
            "{ch}"
        );
    }

    #[test]
    fn failed_task_preview() {
        let t = views::show_task(&fixture(), "4", &colors()).ok().unwrap();
        assert!(t.contains("  waits for: 3 (prerequisites)\n"), "{t}");
        assert!(
            t.contains("  data:      hook-setup, opaque, patch\n"),
            "{t}"
        );
        assert!(t.contains("  spawned:   2026-11-21 10:00:00\n"));
        assert!(t.contains("  2026-11-21T10:00:05Z INFO running install hook\n"));
        assert!(t.contains("<R>2026-11-21T10:00:08Z ERROR run hook \"install\": exit status 1</>"));
        assert!(views::show_task(&fixture(), "99", &plain()).is_err());
    }

    #[test]
    fn tree_pointers_and_display() {
        let v = serde_json::json!({"data": {"conns": {"a:b c:d": {}}, "x/y~z": [1, {"k.v": 2}]}});
        let out = views::list_tree(&v, "", &plain()).ok().unwrap();
        assert_eq!(
            rows(&out),
            [
                ("/data", "data  {2}"),
                ("/data/conns", "data.conns  {1}"),
                ("/data/conns/a:b c:d", "data.conns[\"a:b c:d\"]  {0}"),
                ("/data/x~1y~0z", "data.x/y~z  [2]"),
                ("/data/x~1y~0z/0", "data.x/y~z[0]  1"),
                ("/data/x~1y~0z/1", "data.x/y~z[1]  {1}"),
                ("/data/x~1y~0z/1/k.v", "data.x/y~z[1][\"k.v\"]  2"),
            ]
        );
        let sub = views::list_tree(&v, "/data/x~1y~0z/1", &plain())
            .ok()
            .unwrap();
        assert_eq!(
            rows(&sub),
            [
                ("/data/x~1y~0z/1", "data.x/y~z[1]  {1}"),
                ("/data/x~1y~0z/1/k.v", "data.x/y~z[1][\"k.v\"]  2"),
            ]
        );
        assert!(views::list_tree(&v, "/nope", &plain()).is_err());
        let p = views::show_path(&v, "/data/x~1y~0z", &plain())
            .ok()
            .unwrap();
        assert_eq!(p, "data.x/y~z\n[\n  1,\n  {\n    \"k.v\": 2\n  }\n]\n");
    }

    #[test]
    fn tree_shows_cookies_redacted() {
        let out = views::list_tree(&fixture(), "/data/snap-cookies", &plain())
            .ok()
            .unwrap();
        let keys: Vec<&str> = rows(&out).iter().map(|r| r.0).collect();
        assert_eq!(
            keys,
            [
                "/data/snap-cookies",
                "/data/snap-cookies/<redacted-1>",
                "/data/snap-cookies/<redacted-2>"
            ]
        );
    }

    #[test]
    fn sequence_paths_in_tree() {
        let out = views::list_tree(&fixture(), "", &plain()).ok().unwrap();
        assert!(out.contains("/data/snaps/pc/sequence/1\tdata.snaps.pc.sequence[1]  {4}\n"));
    }
}
