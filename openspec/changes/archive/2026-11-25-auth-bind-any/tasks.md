# Tasks

## 1. Any-store pairing and the warning

- [x] 1.1 In `share/creds.sh`, have `record_cred_store` accept `*` (any store), and make `warn_store_mismatch` return silently when the record is `*`. Verify in a scratch `IDIOT_DATA_HOME` from the source tree (`eval "$(./idiot init bash)"`): with `.store.personal` set to `store: *` and a brand store active, `idiot auth login personal` and `idiot status` print no warning.
- [x] 1.2 Add the third fix to the warning's hint line (`any store: idiot auth bind --any <name>`). Verify: with `.store.ci` set to `store: abc123` and another store active, `idiot auth login ci` prints all three fixes.
- [x] 1.3 `cmd/auth/bind`: parse `--any` (before or after the name), record `*`, say `Credentials '<name>' are for any store`, keep the "no credential" usage failure, and document `--any` in its help. Verify: `idiot auth bind --any personal` writes `store: *`; plain `idiot auth bind personal` re-pairs with the active store; `idiot auth bind --any` with nothing active fails and writes nothing.
- [x] 1.4 `cmd/auth/add`: add `--any` (help and parsing); record `*` and say `for any store` in the confirmation. Verify the parsing and recording without a login by checking `idiot auth add --any x --expires 0d` fails on the expiry before prompting, then read the code path that writes the record; the full login is checked in 3.2.
- [x] 1.5 Extend `tests/bare-host.d/profile` after the existing `auth bind` check: `auth bind --any`, no warning on `store use`/`auth login`, the warning's third fix for a paired credential, and plain `auth bind` re-pairing. Run `make check`.
- [x] 1.6 Update the README's Credentials and stores section (the warning sample and `--any`) and the using-idiot skill's `references/commands.md` (`auth bind --any`, `auth add --any`). Commit: `feat(auth): mark a credential for any store`.

## 2. Stores in auth list

- [x] 2.1 Add a short store label helper to `share/creds.sh` (the saved store's name, else the ID, `global`, `any store` for `*` or no record) and show it, dimmed, after the expiry in `cmd/auth/list` (which then includes `stores.sh`). Verify in a scratch data directory with one credential of each kind (paired with a saved store, an unsaved ID, global, `*`, no record): each line shows the expected label and the expiry column is unchanged; `idiot auth list 2>/dev/null` prints nothing on stdout.
- [x] 2.2 Add an `auth list` check to `tests/bare-host.d/profile` (the store names shown for `ci` and an any-store credential), update the README's `auth list` mention if any and the skill reference, then run `make check`. Commit: `feat(auth): show each credential's store in auth list`.

## 3. Verification

- [x] 3.1 Run `openspec validate auth-bind-any --strict`.
- [x] 3.2 Pack the snap (`snapcraft clean idiot && snapcraft pack`) and run `tests/bare-host idiot_*_amd64.snap` on `local:`; the profile checks pass. Ask the user to run one real `idiot auth add --any <name>` login, since a login needs their Ubuntu One account, and confirm `idiot auth list` shows `any store` for it.
