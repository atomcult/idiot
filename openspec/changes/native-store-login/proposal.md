# Proposal

## Why

`idiot auth add` and `idiot auth status` shell out to `snapcraft
export-login` and `snapcraft whoami`. That makes snapcraft a hard dependency
for something idiot should do itself, and it means waiting on snapcraft's
start-up just to read account details. snapcraft's interactive prompts also
decide which permissions, snaps, channels and expiry a credential gets.
`~/work/idiot-v2` already implements the store's macaroon login
(root macaroon from the dashboard, discharge from login.ubuntu.com, 2FA) and
whoami in Rust. Bringing it into idiot removes the snapcraft dependency and
lets idiot scope credentials properly.

## What Changes

- New Rust helper `idiot-store`, ported from idiot-v2, shipped with idiot
  (built by `make install` and by the snap), with two subcommands:
  - `login`: prompts on the terminal for email, password and (when the
    store asks) a one-time password; requests a credential with the given
    permissions, snaps, channels and expiry; writes it to a private file.
  - `whoami`: reports the account and restrictions of a credential, in the
    same `key: value` format `snapcraft whoami` prints.
- `idiot auth add <name>` uses `idiot-store login` and gains options:
  `--permissions [list]` (no value: fzf multi-select of store permissions),
  `--snaps <list>`, `--channels <list>`, `--expires <date|Nd>`. The defaults
  match `snapcraft export-login`: all `package_*` permissions, no snap or
  channel restriction, one year.
- `idiot auth status` uses `idiot-store whoami`; output and caching are
  unchanged.
- Credentials keep snapcraft's format, so idiot-made credentials work with
  snapcraft and existing credentials keep working with idiot.
- snapcraft is no longer needed for any `idiot auth` command.
- stdout: no command's eval output changes. `idiot-store whoami` writes its
  report to stdout, and `auth status` reads it from there.
- New dependencies:
  - Build time: a Rust toolchain (cargo) for `make install`, and the snap's
    `rust` plugin.
  - Run time: none beyond the built binary, which uses rustls rather than the
    system OpenSSL. The snap must include the binary.

## Capabilities

### New Capabilities

### Modified Capabilities
- `auth`: creating a credential no longer uses snapcraft and accepts permission/snap/channel/expiry options; account status comes from idiot's own store client; credential format compatibility becomes an explicit requirement.

## Impact

- New `tools/idiot-store/` crate (from idiot-v2's `src/`, without its
  untracked `creds`, `snapcraft` symlink and `GEMINI.md`).
- `cmd/auth/add`, `cmd/auth/status`, `cmd/auth/help`, `share/creds.sh`
  (helper lookup), `Makefile` (build + install the helper, `make check` runs
  `cargo fmt --check`/`clippy`), `snap/snapcraft.yaml` (rust part),
  `.gitignore` (`target/`), README/CLAUDE.md.
- Depends on `fix-review-bugs` (credential name validation and 0600 files)
  and `fix-snap-packaging` (classic snap layout). Apply after both.
- idiot-v2 is left as is (archived by Lauren when she's ready).
