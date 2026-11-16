//! `snapd-state`: read snapd's state.json, as `inspect image` and
//! `inspect diff` used to with python3.
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
//! Exit status: 0 ok, 2 usage or an unreadable or invalid file.

use serde_json::{Map, Value};
use std::io::Write;

const USAGE: &str =
    "usage: idiot-helper snapd-state [--report snaps|connections|changes] <state.json>";

pub fn run(args: &[String]) -> u8 {
    let (report, path) = match args {
        [p] if !p.starts_with('-') => (None, p),
        [flag, section, p] if flag == "--report" => (Some(section.as_str()), p),
        _ => {
            eprintln!("{USAGE}");
            return 2;
        }
    };
    let state: Value = match std::fs::read(path)
        .map_err(|e| e.to_string())
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("idiot-helper: {path}: {e}");
            return 2;
        }
    };
    let out = match report {
        None => records(&state),
        Some("snaps") => report_snaps(&state, &Colors::from_env()),
        Some("connections") => report_connections(&state, &Colors::from_env()),
        Some("changes") => report_changes(&state, &Colors::from_env()),
        Some(other) => {
            eprintln!("idiot-helper snapd-state: unknown report: {other}");
            return 2;
        }
    };
    let mut stdout = std::io::stdout().lock();
    if stdout.write_all(out.as_bytes()).is_err() {
        return 2;
    }
    0
}

struct Colors {
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
        0 => "Hold",
        1 => "Do",
        2 => "Doing",
        3 => "Done",
        4 => "Abort",
        5 => "Undo",
        6 => "Undoing",
        7 => "Undone",
        8 => "Wait",
        9 => "Error",
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
            let code = ch.get("status").and_then(Value::as_i64).unwrap_or(-1);
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
            3 => &c.green,
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
             "changes":{"10":{"status":3,"ready-time":"2026-11-01T10:05:00.5Z","summary":"Init","task-ids":["1"]},
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
}
