# Tasks

## 1. `idiot-store expiry`

- [ ] 1.1 Confirm the caveat format on a real credential before writing the matcher: with a throwaway Rust test or `examples/` binary run locally (not committed), print only each first-party caveat's predicate *key* (`<location>|<key>`) and, for `expires`, its value, for the root and discharge of one credential made with `idiot auth add probe --expires 2d` (Lauren runs the login); never print signatures, the account caveat's value, or the credential. Record the observed format in design.md's Assumptions and delete `probe` afterwards
- [ ] 1.2 Add the caveat reader next to `credential::decode` and `src/cmd/expiry.rs` (`--file`, else `UBUNTU_STORE_AUTH`, like `whoami`); register `Expiry` in `main.rs` and list it in the crate doc comment; add the unit tests from design.md ("Helper tests"); verify `cargo test --manifest-path tools/idiot-store/Cargo.toml` passes offline (no network namespace: `unshare -rn cargo test …`) and that `idiot-store expiry --file <probe credential>` prints the expected lines and nothing else
- [ ] 1.3 Run `make check` and commit (`feat(idiot-store): read a credential's expiry offline`)

## 2. Expiry record and display

- [ ] 2.1 `share/creds.sh`: `expiry_record_path`, `record_expiry <name>` (helper times → `date -u -d` → smallest Unix time, written 0600 via `ensure_auth_dir`'s umask; falls back to a given time), `read_expiry <name>`, `expiry_phrase <unix> <now>`, and the warning-window parser for `IDIOT_CRED_WARN_DAYS`; verify `expiry_phrase` by hand for 41d, 5h, 30min and −2d, and a record's mode is 0600 under `umask 022`
- [ ] 2.2 `cmd/auth/add` and `cmd/auth/import`: record the expiry after saving (add: fall back to the requested `--expires`; import: dim `expiry unknown` note on failure, still exit 0); `cmd/auth/remove`: delete the record and the warning stamp; verify the "Recorded at creation", "Recorded on import", "Unreadable expiry" and "Remove" scenarios, and `strace -f -e trace=execve` of `auth import` shows no credential in any argv
- [ ] 2.3 `cmd/auth/list` (expiry after each name), `cmd/auth/status` (` (in 41d)` / ` (expired)` after `expires:`; backfill a missing record from the file, else from the whoami report), `cmd/status` (auth row annotation from `IDIOT_CRED_EXPIRES`, else the record); verify the auth "Credential storage" and "Account status" scenarios and status "Credential expiry"/"Expired credential" with records written by hand (past, 3 days out, 41 days out, none)
- [ ] 2.4 README (auth section: expiry shown where, `IDIOT_CRED_WARN_DAYS`), CLAUDE.md (new creds.sh helpers, the `.expires.<name>` record under State paths), auth group help (`help_env IDIOT_CRED_WARN_DAYS`); run `make check` and commit (`feat(auth): record and show credential expiry`)

## 3. Login export and warning

- [ ] 3.1 `cmd/auth/login`: export `IDIOT_CRED_EXPIRES` or unset it, backfill a missing record when `SNAP` is unset, print the expiring/expired notice and write the day's stamp; `cmd/auth/logout`: unset it; verify the "Activating a credential" scenarios through the wrapper in bash and fish, including the snap case with no record (`snapcraft pack`, `snap install --dangerous`)
- [ ] 3.2 `idiot`: call `cred_expiry_warn` where `_increment_count` is called; verify "Expiring soon", "Only once a day" (second shell too), "Expired", "Window configured", "Early warning disabled" and "Expiry unknown" by writing records and stamps by hand; under the snap verify a host command (e.g. `idiot auth status`) warns exactly once although it is dispatched twice
- [ ] 3.3 Verify the warning check adds nothing measurable without an active credential (100 × `idiot store list` with and without the change, timing as in shell-prompt-segment's design) and one `date` call with one; run `make check` and commit (`feat(auth): warn before the active credential expires`)

## 4. Integration

- [ ] 4.1 Walk every scenario in the change's auth and status specs on a source install and the snap, check `ls -la "$IDIOT_AUTH_DIR"` shows only 0600 files and a 0700 directory, then run `make check`
