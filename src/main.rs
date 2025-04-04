use std::collections::HashMap;

use anyhow::Result;
use clap::{Arg, Command};

mod cmd;
use cmd::SubCmd;

const HELP_TEMPLATE: &'static str = "\
{before-help}{name} ({about}) {version}
{author-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
";

fn main() -> Result<()> {
    let cmdmap: HashMap<String, SubCmd> =
        HashMap::from_iter([("login".into(), cmd::login::init())]);

    let mut app = cli();
    for subcmd in cmdmap.values() {
        app = app.subcommand((subcmd.cmd)());
    }

    let matches = &app.get_matches_mut();
    match matches.subcommand() {
        Some((name, args)) => {
            if let Some(cmd) = cmdmap.get(name) {
                (cmd.run)(args)?;
            } else {
                eprintln!("err: subcommand not found: {}\n", name);
                app.print_help()?;
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn cli() -> Command {
    Command::new("idiot-store")
        .about("Interactive Devtools for IoT")
        .author("Lauren Brock <lauren.brock@canonical.com>")
        .help_template(HELP_TEMPLATE)
        .version("alpha")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .args_conflicts_with_subcommands(true)
        .allow_external_subcommands(true)
    // .subcommand(
    //     // The idea here is that we manage the state for a shell, spawn the
    //     // shell, and, upon exit, provide a menu to the user that allows
    //     // them to tweak the environment. Once configuration is complete, a
    //     // new shell is spawned with the appropriate environment. Of course,
    //     // there is also an option to exit. Additionally, there are flags
    //     // that can be used to specify the starting state of the shell,
    //     // thereby skipping the initial configuration.
    //     Command::new("shell"), // .arg(Arg::new("login"))
    //                            // .arg(Arg::new("store"))
    //                            // .arg(Arg::new("arch"))
    //                            // .arg(Arg::new("shell")),
    // )
}
