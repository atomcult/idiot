# Tasks

Verification below runs from a source checkout through the shell wrapper
(`eval "$(./idiot init bash)"`) with `IDIOT_DATA_HOME` and `IDIOT_CACHE_DIR`
pointed at a fresh `mktemp -d`, and fake credentials made with
`auth import` (no store contact needed except where noted).

## 1. Shared emitters (no behaviour change)

- [x] 1.1 `share/creds.sh`: add `emit_cred_login <name>` (require_cred + the three exports) and `emit_cred_logout`; rewire `cmd/auth/login` and `cmd/auth/logout`; verify the auth spec's "Activating a credential" scenarios (login by name, unknown credential leaves variables unchanged, picker cancel, logout) still hold
- [x] 1.2 `share/stores.sh`: add `store_id <name>`, `store_name_for_id <id>`, `emit_store_use <name>`, `emit_store_unset`; rewire `cmd/store/use`, `cmd/store/unset` and the store row of `cmd/status`; verify the store spec's "Activating a store" scenarios and the status spec's named/unknown store scenarios
- [x] 1.3 `share/arch.sh`: add `STORE_ARCHES` and `is_store_arch`; rewire `cmd/arch/set` (picker list and error message from it); verify `idiot arch set arm64`, `idiot arch set x86` (lists valid arches, nothing changes) and the picker
- [x] 1.4 Run `make check`; commit (`refactor: share auth, store and arch emitters`)

## 2. Credential store pairing and mismatch warning

- [x] 2.1 `share/creds.sh`: `cred_store_path`, `record_cred_store <name> <id>`, `cred_store_id <name>` (fails when unpaired) and `warn_store_mismatch <cred> <store-id>` (never dies, returns 0); verify by hand that a `.store.<name>` file is 0600, holds only `store: <id>`, and is not listed by `idiot auth list` or offered by `idiot auth login`'s picker
- [x] 2.2 `cmd/auth/add`: record the active store after the rename and name it in the confirmation; `cmd/auth/remove`: delete the pairing too; verify live (with Lauren) that `auth add` under `UBUNTU_STORE_ID=abc123` pairs with `abc123`, and that `auth remove` deletes credential, whoami cache and pairing (verified with a stub idiot-store helper; a live Ubuntu One login is still to be tried)
- [x] 2.3 New `cmd/auth/bind [<name>]` (confined, `require_cred`, defaults to `IDIOT_ACTIVE_CRED`, usage error with neither); add it to `cmd/auth/help`; verify the auth spec's "Re-pairing" and "Nothing to bind" scenarios, and that `auth import` leaves no pairing
- [x] 2.4 Call `warn_store_mismatch` from `cmd/auth/login`, `cmd/store/use`, `cmd/store/unset` (after emitting, with the post-change values) and from `cmd/status` below the rows; verify every scenario of the auth spec's "Store mismatch warning", the store spec's "Warning on a store switch" and the status spec's "Credential and store mismatch", checking each time that the exit status is 0, stdout is empty and the eval file holds the same statements as without the warning
- [x] 2.5 Update README (pairing, `auth bind`, what the warning means) and run `make check`; commit (`feat(auth): pair credentials with a store and warn on mismatch`)

## 3. Profiles

- [ ] 3.1 New `share/profiles.sh`: `IDIOT_PROFILES_DIR`, `validate_profile_name`, `require_profile`/`require_no_profile`, and a parser that reads a profile into per-field "absent / empty / value" variables without sourcing it, failing on unknown keys or malformed lines; verify against hand-written files (absent key, empty value, `stor: x`, a line without `:`, `auth: ../../x`)
- [ ] 3.2 `cmd/profile/save <name> [--fields <list>] [--force]` and `cmd/profile/help`; verify the profile spec's "Profile storage" and "Saving a profile" scenarios (incl. unnamed credential and unmatched store ID warnings, `--force`), and that the file holds no credential contents (`grep` the credential's text in `IDIOT_PROFILES_DIR` finds nothing)
- [ ] 3.3 `cmd/profile/use [<name>]`: validate every field, then emit via the group 1 emitters plus `IDIOT_LP_USER`/`IDIOT_ACTIVE_PROFILE`, then `warn_store_mismatch` with the resulting pair; fzf picker with `file_preview_cmd`; call the warning from `profile save` too; verify every "Activating a profile" scenario, in particular that a profile naming a removed credential leaves the eval file empty and that the `lp` file is untouched
- [ ] 3.4 `cmd/profile/list`, `cmd/profile/remove [<name>]`, `cmd/profile/status`; verify the "Listing and removing profiles" and "Active profile status" scenarios (empty list hint on stderr and nothing on stdout, drift after `arch set`, removed profile, none active)
- [ ] 3.5 `cmd/status`: `profile` row (name, `changed`, `removed`); add `profile` to `idiot_help` in `idiot`; verify the status spec's "Active profile" and "Changed profile" scenarios and that `idiot status` in a fresh shell has no profile row
- [ ] 3.6 Update README (profiles section, absent vs. empty fields, `lp` file untouched) and CLAUDE.md (layout: `share/profiles.sh`; state paths: `IDIOT_PROFILES_DIR`); run `make check`; commit (`feat(profile): save and switch environment profiles`)

## 4. Integration under the snap

- [ ] 4.1 Build the snap and install it without optional components; in a shell set up with the snap's `idiot init`, verify `profile save`/`use`/`list`/`status`/`remove` and `auth bind` run without a host handoff (no `/snap/.../lib/idiot/idiot` line in the eval file; `grep -L '# idiot: host' cmd/profile/* cmd/auth/bind` lists them all), that profiles land in `~/snap/idiot/common/profiles`, and that a pairing written by the host-side `auth add` is seen by the confined `auth login` warning
- [ ] 4.2 Run `openspec validate add-env-profiles` and `make check` once more; commit any fixes (`fix(profile): ...`) or nothing if clean
