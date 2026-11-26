# Tasks

## 1. idiot-store: no padding, and repair

- [ ] 1.1 In `store/login.rs`, keep the ACL response's `macaroon` string and SSO's `discharge_macaroon` string, and change `credential::encode` to take the two strings, stripping any trailing `=` from each. Verify with unit tests: a credential built from padded inputs has no `=` in `r` or `d` and decodes to the same macaroons; one built from unpadded inputs keeps them byte-for-byte.
- [ ] 1.2 Add `credential::repair` (strip trailing `=` from `r` and `d`; report whether anything changed; reject anything that isn't a `u1-macaroon` credential) and `idiot-store repair [--check] <file>` (`cmd/repair.rs`, `main.rs`). It rewrites atomically through a 0600 temporary file in the same directory, prints only `ok`, `repaired` or `padded`, and with `--check` writes nothing. Verify with unit tests for the repair and a test of the command in a directory under `std::env::temp_dir()` (no new dependency): mode 0600 afterwards, content unchanged for an unpadded file, an invalid file left untouched with a non-zero exit.
- [ ] 1.3 Run `cargo test` and `make check`. Commit: `fix(idiot-store): keep the store's macaroons unpadded, as snapd needs`.

## 2. Repair on use

- [ ] 2.1 In `share/creds.sh`, have `emit_cred_login` run `repair` on the credential's file before reading it, print `repaired credentials '<name>' (snapd couldn't use them)` on `repaired`, and warn and carry on when idiot-store is missing. Verify from the source tree in a scratch data directory with a hand-made padded fake credential (base64 JSON with `r` ending in `=`): `idiot auth login` repairs the file and exports the repaired value; a second login prints nothing; `profile use` of a profile naming it repairs it too.
- [ ] 2.2 In `cmd/auth/export`, repair first, then export. Verify: exporting the padded fake to a file leaves both files unpadded.
- [ ] 2.3 Extend `tests/bare-host.d/` (the confined profile/auth checks): a padded fake credential is repaired by a confined `auth login`, with the eval file holding the unpadded value, and the file still has mode 0600. Update the README's Credentials and stores section (a short note) and the using-idiot skill reference. Run `make check`. Commit: `fix(auth): repair padded credentials when they're used`.

## 3. Doctor

- [ ] 3.1 In `cmd/doctor`, run `repair --check` for each saved credential when idiot-store is available, and report `!` naming the credential, that snapd can't use it, and `idiot auth login <name>`. Verify in a scratch data directory: a padded fake gives the `!` line and stays unchanged; after `auth login` the line is gone; with the shell still holding the old value, the existing "differs from the saved one" line appears.
- [ ] 3.2 Add the padded-credential line to `tests/bare-host.d/doctor` (before a login repairs it) and update the README's doctor sample if it lists credential lines. Run `make check`. Commit: `feat(doctor): flag credentials snapd can't use`.

## 4. Verification

- [ ] 4.1 Run `openspec validate fix-macaroon-padding --strict`.
- [ ] 4.2 Pack the snap (`snapcraft clean idiot`, then pack into a gitignored directory inside the tree) and run `tests/bare-host` on `local:`; all checks pass.
- [ ] 4.3 With the user: `idiot auth login` on their padded `lauren-brock` credential reports the repair, then with `ce-team-test` active `snap download ce-dummy` succeeds; and a fresh `idiot auth add` credential has no padding (checked without printing it).
