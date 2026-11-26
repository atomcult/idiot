use super::Failure;
use crate::RepairArgs;
use idiot_store::store::credential::repair;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Strips base64 padding from a credential file's macaroons, in place.
/// Prints only `ok` (nothing to do), `repaired`, or with --check `padded`;
/// never the credential. The rewrite goes through a private temporary file
/// in the same directory, renamed over the original.
pub fn run(args: RepairArgs) -> Result<(), Failure> {
    let path = &args.file;
    let credential =
        fs::read_to_string(path).map_err(|e| Failure::local(format!("{}: {e}", path.display())))?;
    let status = match repair(&credential)? {
        None => "ok",
        Some(_) if args.check => "padded",
        Some(fixed) => {
            replace(path, &fixed)?;
            "repaired"
        }
    };
    println!("{status}");
    Ok(())
}

fn replace(path: &Path, contents: &str) -> Result<(), Failure> {
    let name = path
        .file_name()
        .ok_or_else(|| Failure::local(format!("{}: not a file", path.display())))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(format!(".repair.{}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    let write = || -> std::io::Result<()> {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    };
    write().map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Failure::local(format!("{}: {e}", path.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::prelude::*;
    use std::os::unix::fs::PermissionsExt;

    fn dir(tag: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("idiot-store-repair-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn cred(r: &str, d: &str) -> String {
        BASE64_STANDARD.encode(format!(
            r#"{{"t":"u1-macaroon","v":{{"r":"{r}","d":"{d}"}}}}"#
        ))
    }

    fn args(file: &Path, check: bool) -> RepairArgs {
        RepairArgs {
            file: file.to_path_buf(),
            check,
        }
    }

    #[test]
    fn repairs_in_place_privately() {
        let d = dir("fix");
        let f = d.join("work");
        fs::write(&f, cred("abc=", "def")).unwrap();
        fs::set_permissions(&f, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(run(args(&f, false)).is_ok());
        assert_eq!(fs::read_to_string(&f).unwrap(), cred("abc", "def"));
        assert_eq!(
            fs::metadata(&f).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::read_dir(&d).unwrap().count(),
            1,
            "no temporary file left"
        );
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn check_and_clean_files_unchanged() {
        let d = dir("check");
        let padded = d.join("padded");
        fs::write(&padded, cred("abc=", "def")).unwrap();
        assert!(run(args(&padded, true)).is_ok());
        assert_eq!(fs::read_to_string(&padded).unwrap(), cred("abc=", "def"));
        let clean = d.join("clean");
        let text = format!("{}\n", cred("abc", "def"));
        fs::write(&clean, &text).unwrap();
        assert!(run(args(&clean, false)).is_ok());
        assert_eq!(fs::read_to_string(&clean).unwrap(), text, "byte-for-byte");
        fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn invalid_left_alone() {
        let d = dir("bad");
        let f = d.join("bad");
        fs::write(&f, "not a credential").unwrap();
        assert!(run(args(&f, false)).is_err());
        assert_eq!(fs::read_to_string(&f).unwrap(), "not a credential");
        assert!(run(args(&d.join("missing"), false)).is_err());
        fs::remove_dir_all(&d).unwrap();
    }
}
