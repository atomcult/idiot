//! idiot-helper - the host-side work idiot can't do in POSIX sh.
//!
//! One subcommand per job, each a port of what python3 used to do:
//! parsing snapd's state and assertions, reading UEFI variable stores,
//! and talking to qemu's and swtpm's sockets. Data goes to stdout, errors
//! to stderr. It never handles credentials and has no network code.
//!
//! Exit status: 0 ok, 1 a negative answer or invalid input, 2 usage or
//! unreadable input (each subcommand documents its own meaning).

use std::process::ExitCode;

const USAGE: &str = "usage: idiot-helper <command> [args...]

commands:
  assertion       summarise and verify a stream of snapd assertions
  uefi-vars       list the variables in an OVMF/AAVMF variable store
  snapd-state     print snapd state.json records as tab-separated lines
  qmp             send one QMP command to a running qemu
  port-free       succeed if a TCP port can be bound
  swtpm-shutdown  ask swtpm to shut down over its control socket";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let code = match cmd.as_str() {
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            0
        }
        "-V" | "--version" => {
            println!("idiot-helper {}", env!("CARGO_PKG_VERSION"));
            0
        }
        _ => {
            eprintln!("idiot-helper: {cmd}: not implemented");
            let _ = rest;
            2
        }
    };
    ExitCode::from(code)
}
