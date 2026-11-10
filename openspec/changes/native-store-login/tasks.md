# Tasks

## 1. Bring in the crate

- [x] 1.1 Copy idiot-v2's `Cargo.toml`, `Cargo.lock` and `src/` (only those; not `creds`, `snapcraft` or `GEMINI.md`) to `tools/idiot-store/`, add `target/` to `.gitignore`; verify `git status` shows no credential file and `cargo build --manifest-path tools/idiot-store/Cargo.toml` succeeds
- [x] 1.2 Commit as-is so the port is reviewable as a diff (`chore(idiot-store): import store client from idiot-v2`)

## 2. Port and harden the helper

- [x] 2.1 Switch to blocking `reqwest` with `rustls-tls` and drop tokio; verify `cargo build` and `ldd target/release/idiot-store | grep -c ssl` prints 0
- [x] 2.2 Implement the `login` interface from design.md (`--output` 0600 `O_EXCL`, `--permissions/--snaps/--channels/--expires/--description`, `/dev/tty` prompts, exit codes) and `whoami` with snapcraft's exact `key: value` lines; remove unused `parse_credential`; add unit tests for credential encode/decode, caveat extraction and SSO error/2FA parsing from recorded JSON; verify `cargo test` passes offline and `cargo clippy -- -D warnings` is clean
- [ ] 2.3 Live check with Lauren's account: `idiot-store login --output /tmp/x --permissions package_access --expires <tomorrow>` then `UBUNTU_STORE_AUTH="$(cat /tmp/x)" idiot-store whoami`; confirm the store accepts the `idiot-store/<version>` user agent (else apply the documented fallback), confirm 2FA prompting, then delete the test credential file
- [ ] 2.4 Commit (`feat(idiot-store): scoped login and snapcraft-compatible whoami`)

## 3. Build and install

- [x] 3.1 Makefile: build with `cargo build --release --locked`, install to `$(LIBDIR)/libexec/idiot-store`, fail clearly without cargo; add `cargo fmt --check` and `cargo clippy` to `make check`; verify `make install PREFIX=$(mktemp -d)` puts an executable at `lib/idiot/libexec/idiot-store` and `make check` passes
- [x] 3.2 Add `store_helper` to `share/creds.sh`; verify it resolves the installed binary, the dev-checkout fallback, and the "make install" error when neither exists
- [x] 3.3 Snap: add the `idiot-store` rust part (`enable-patchelf`, organised into `lib/idiot/libexec/`); verify `snapcraft pack` succeeds, `unsquashfs -l` lists `lib/idiot/libexec/idiot-store`, and `readelf -l` shows core24's interpreter; mark `cmd/auth/add` and `cmd/auth/status` `# idiot: host` if `host-handoff` hasn't
- [x] 3.4 Run `make check` and commit (`build: build and ship idiot-store`)

## 4. Use it from `idiot auth`

- [ ] 4.1 `cmd/auth/add`: new options, validation before prompting (name, permissions, expiry incl. `Nd`), `--permissions` fzf multi-select, helper call into a temp file in the auth dir and atomic rename; update its help and `cmd/auth/help`; verify every scenario in the change's auth spec "Creating a credential" (live, with Lauren), including that `snapcraft` absent from PATH doesn't matter
- [ ] 4.2 `cmd/auth/status`: use `store_helper whoami`; verify cached/refresh/not-logged-in/expired scenarios and that the `idiot status` email annotation still appears
- [ ] 4.3 Compatibility: verify `snapcraft whoami` accepts an idiot-made credential and `idiot auth status` accepts a snapcraft-exported one
- [ ] 4.4 Update README and CLAUDE.md (helper layout, `make install` needs cargo, no snapcraft needed for auth); run `make check` and commit (`feat(auth)!: log in without snapcraft`)
