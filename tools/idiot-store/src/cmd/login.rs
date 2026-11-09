use super::Failure;
use crate::LoginArgs;
use idiot_store::store::login::{Login, Restrictions};
use idiot_store::StoreError;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::OpenOptionsExt;

pub fn run(args: LoginArgs) -> Result<(), Failure> {
    // fail before prompting, not after a successful login
    if args.output.exists() {
        return Err(Failure::local(format!(
            "{}: already exists",
            args.output.display()
        )));
    }

    let restrictions = Restrictions {
        permissions: args.permissions,
        description: args.description,
        expires: args.expires,
        snaps: args.snaps,
        channels: args.channels,
    };

    let email = prompt_line("Email: ")?;
    let password = rpassword::prompt_password("Password: ")?;

    let login = Login::start(&restrictions)?;
    let credential = match login.finish(&email, &password, None) {
        Err(StoreError::TwoFactorRequired) => {
            let otp = rpassword::prompt_password("One-time password: ")?;
            login.finish(&email, &password, Some(otp.trim()))?
        }
        other => other?,
    };

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&args.output)
        .map_err(|e| Failure::local(format!("{}: {e}", args.output.display())))?;
    file.write_all(credential.as_bytes())?;
    Ok(())
}

/// Prompts on the terminal (not stdin/stdout, which may be redirected) and
/// reads one echoed line.
fn prompt_line(prompt: &str) -> Result<String, Failure> {
    let tty = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| Failure::local(format!("no terminal to prompt on: {e}")))?;
    let mut out: &File = &tty;
    out.write_all(prompt.as_bytes())?;
    out.flush()?;
    let mut line = String::new();
    BufReader::new(&tty).read_line(&mut line)?;
    Ok(line.trim().to_string())
}
