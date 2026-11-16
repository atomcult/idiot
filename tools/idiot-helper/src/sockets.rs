//! The socket jobs POSIX sh can't do, for `idiot vm`.
//!
//! qmp <socket> <command> [<json-arguments>]
//!   Send one QMP command to a running qemu (the former share/vm-qmp.py):
//!   read the greeting, negotiate capabilities, send the command, print its
//!   "return" as JSON on stdout. Exit 1 with the error's description on
//!   stderr when qemu answers with an error, 2 when the socket can't be
//!   used. qemu closing the socket after `quit` is success.
//! port-free <port>
//!   Exit 0 if TCP port <port> can be bound on all addresses with
//!   SO_REUSEADDR, as qemu's hostfwd will bind it, else 1.
//! swtpm-shutdown <socket>
//!   Send swtpm's control channel CMD_SHUTDOWN and wait for its answer.
//!   Exit 0 when swtpm answered, 1 otherwise.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

/// `json.dumps(v)`: Python's separators and ASCII-only escaping.
fn dumps(v: &Value) -> String {
    match v {
        Value::Object(m) => format!(
            "{{{}}}",
            m.iter()
                .map(|(k, v)| format!("{}: {}", dumps(&Value::String(k.clone())), dumps(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Array(a) => format!("[{}]", a.iter().map(dumps).collect::<Vec<_>>().join(", ")),
        Value::String(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    '\x08' => out.push_str("\\b"),
                    '\x0c' => out.push_str("\\f"),
                    c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                        let mut buf = [0u16; 2];
                        for u in c.encode_utf16(&mut buf) {
                            out.push_str(&format!("\\u{u:04x}"));
                        }
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        Value::Null => "null".into(),
        Value::Number(n) => n.to_string(),
    }
}

pub fn qmp(args: &[String]) -> u8 {
    let (path, command, arguments) = match args {
        [p, c] => (p, c, None),
        [p, c, a] => (p, c, Some(a)),
        _ => {
            eprintln!("usage: vm-qmp.py <socket> <command> [<json-arguments>]");
            return 2;
        }
    };
    let mut request = json!({ "execute": command });
    if let Some(a) = arguments {
        match serde_json::from_str::<Value>(a) {
            Ok(v) => request["arguments"] = v,
            Err(e) => {
                eprintln!("vm-qmp: {e}");
                return 2;
            }
        }
    }
    let sock = match UnixStream::connect(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("vm-qmp: {path}: {}", crate::py::strerror(&e));
            return 2;
        }
    };
    let timeout = Some(Duration::from_secs(10));
    if sock.set_read_timeout(timeout).is_err() || sock.set_write_timeout(timeout).is_err() {
        eprintln!("vm-qmp: {path}: can't set a timeout");
        return 2;
    }
    let mut writer = match sock.try_clone() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("vm-qmp: {e}");
            return 2;
        }
    };
    let mut reader = BufReader::new(sock);
    let mut line = String::new();
    // skip asynchronous events until the answer to what was sent; None when
    // qemu closed the socket (e.g. after quit)
    let mut reply = |reader: &mut BufReader<UnixStream>| -> Result<Option<Value>, String> {
        loop {
            line.clear();
            if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                return Ok(None);
            }
            let msg: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            if msg.get("return").is_some() || msg.get("error").is_some() {
                return Ok(Some(msg));
            }
        }
    };
    let result = (|| -> Result<u8, String> {
        let mut greeting = String::new();
        reader.read_line(&mut greeting).map_err(|e| e.to_string())?;
        let mut last = Value::Null;
        for req in [json!({ "execute": "qmp_capabilities" }), request] {
            writer
                .write_all(format!("{}\n", dumps(&req)).as_bytes())
                .map_err(|e| e.to_string())?;
            let Some(msg) = reply(&mut reader)? else {
                return Ok(if command == "quit" { 0 } else { 2 });
            };
            if let Some(err) = msg.get("error") {
                let desc = match err.get("desc") {
                    Some(Value::String(d)) => d.clone(),
                    _ => dumps(err),
                };
                eprintln!("vm-qmp: {desc}");
                return Ok(1);
            }
            last = msg;
        }
        println!("{}", dumps(&last["return"]));
        Ok(0)
    })();
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("vm-qmp: {e}");
            2
        }
    }
}

pub fn port_free(args: &[String]) -> u8 {
    let [port] = args else {
        eprintln!("usage: idiot-helper port-free <port>");
        return 2;
    };
    let Ok(port) = port.parse::<u16>() else {
        eprintln!("idiot-helper port-free: not a port: {port}");
        return 2;
    };
    // std sets SO_REUSEADDR on Unix before binding
    match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

pub fn swtpm_shutdown(args: &[String]) -> u8 {
    let [path] = args else {
        eprintln!("usage: idiot-helper swtpm-shutdown <socket>");
        return 2;
    };
    let run = || -> std::io::Result<()> {
        let mut s = UnixStream::connect(path)?;
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        s.set_write_timeout(Some(Duration::from_secs(5)))?;
        s.write_all(&3u32.to_be_bytes())?; // CMD_SHUTDOWN
        let mut answer = [0u8; 4];
        let _ = s.read(&mut answer)?;
        Ok(())
    };
    match run() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    fn fake_qemu(
        answers: &'static [&'static str],
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let dir = std::env::temp_dir().join(format!(
            "idiot-helper-qmp-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("qmp.sock");
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let handle = std::thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            let mut w = s.try_clone().unwrap();
            let mut r = BufReader::new(s);
            w.write_all(b"{\"QMP\": {\"version\": {}}}\n").unwrap();
            let mut got = Vec::new();
            for a in answers {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap() == 0 {
                    break;
                }
                got.push(line.trim_end().to_string());
                w.write_all(a.as_bytes()).unwrap();
            }
            got
        });
        (path.to_string_lossy().into_owned(), handle)
    }

    #[test]
    fn qmp_negotiates_then_sends() {
        let (path, h) = fake_qemu(&[
            "{\"return\": {}}\n",
            "{\"event\": \"POWERDOWN\"}\n{\"return\": {}}\n",
        ]);
        assert_eq!(qmp(&[path, "system_powerdown".into()]), 0);
        assert_eq!(
            h.join().unwrap(),
            [
                "{\"execute\": \"qmp_capabilities\"}",
                "{\"execute\": \"system_powerdown\"}"
            ]
        );
    }

    #[test]
    fn qmp_error_is_1() {
        let (path, h) = fake_qemu(&[
            "{\"return\": {}}\n",
            "{\"error\": {\"class\": \"GenericError\", \"desc\": \"nope\"}}\n",
        ]);
        assert_eq!(qmp(&[path, "bogus".into()]), 1);
        h.join().unwrap();
    }

    #[test]
    fn qmp_quit_closing_is_success() {
        // qemu reads quit, then closes the socket without answering
        let (path, h) = fake_qemu(&["{\"return\": {}}\n", ""]);
        assert_eq!(qmp(&[path, "quit".into()]), 0);
        h.join().unwrap();
    }

    #[test]
    fn qmp_no_socket_is_2() {
        assert_eq!(qmp(&["/nonexistent/qmp.sock".into(), "quit".into()]), 2);
    }

    #[test]
    fn port_taken_and_free() {
        let held = std::net::TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let port = held.local_addr().unwrap().port().to_string();
        assert_eq!(port_free(std::slice::from_ref(&port)), 1);
        drop(held);
        assert_eq!(port_free(&[port]), 0);
    }

    #[test]
    fn dumps_like_python() {
        assert_eq!(
            dumps(&json!({"a": [1, "é\n"], "b": null})),
            "{\"a\": [1, \"\\u00e9\\n\"], \"b\": null}"
        );
    }
}
