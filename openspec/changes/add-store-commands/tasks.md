# Tasks

## 1. Response shapes

- [ ] 1.1 With the user's go-ahead, make the four read-only calls once with their credential (`/dev/api/account`, `/api/v2/stores/{ce-team-test}/snaps`, `/v2/snaps/info/ce-dummy` with `Snap-Device-Store`, and the download probe without following redirects). Save the responses as test fixtures under `tools/idiot-store/src/testdata/store/`, with account details (emails, names, IDs other than the stores', keys) replaced by placeholders. Confirm the v2 info `fields` names and the shapes of `stores` and store-snaps items. Verify: the fixtures hold no email, account ID or macaroon (grep for `@`, the user's account ID, `Macaroon`).

## 2. idiot-store: read-only store calls

- [ ] 2.1 `credential::permissions` (offline, from the root's `myapps.developer.ubuntu.com|acl` caveat) and `idiot-store permissions [--file]`. Verify with unit tests on crafted macaroons (a caveat present, absent, malformed) and on the shape seen in 1.1.
- [ ] 2.2 An authorisation header snapd would build (root and bound discharge, both unpadded), used by the new calls. Verify with a unit test that neither part contains `=` and that the discharge is bound to the root.
- [ ] 2.3 `account`, `store-snaps --store <id>`, `info <snap> [--store <id>]` and `probe <snap> --arch --channel [--store <id>]`, printing the tab-separated lines in design.md, with the exit codes there. The probe doesn't follow redirects and reads no body. Verify with unit tests that parse the 1.1 fixtures into the expected lines, plus a test that the probe's client has redirects off.
- [ ] 2.4 Run `cargo test` and `make check`. Commit: `feat(idiot-store): read-only store queries`.

## 3. sh: the store commands

- [ ] 3.1 `share/creds.sh`: resolve the credential for a query (active, or `--cred <name>` mapped to `--file`, failing `credentials '<name>' not found` before any network call) and `creds_with_permission <perm>`. Verify in a scratch data directory with fake credentials carrying crafted `acl` caveats (made by a test helper in idiot-store or by hand).
- [ ] 3.2 `cmd/store/roles` and `cmd/store/channels`: host commands, aligned output on a terminal, `--tsv`, the active store marked, the target arch highlighted, `--arch`. Verify against the user's account (with them): `idiot store roles` shows `ce-team-test` and `dsmr6` with the right roles; `idiot store channels ce-dummy --tsv` has one row per channel and arch.
- [ ] 3.3 `cmd/store/snaps`: an admin with `store_admin` gets the full list, everyone else the partial list with the note, the global store your own snaps; `--tsv`, `--pick` (fzf, chosen name on stdout). Verify with the user: `idiot store snaps --cred <their store_admin credential>` in `ce-team-test` lists the store; without it, the note names that credential.
- [ ] 3.4 `cmd/store/check`: gather the facts, print the report lines and one conclusion in the order design.md gives, exit 0/1. Verify from the source tree: no active credential (anonymous only), a padded fake credential (`✗` padding line, checked offline), an expired fake (`✗`), and with the user `ce-dummy` in `ce-team-test` (downloadable) and a snap in a store they don't belong to (not a member).
- [ ] 3.5 Group help (`cmd/store/help`), each command's `--help`, README (a "Checking a store" section) and the using-idiot skill reference. Add bare-host checks for what works without a store account: usage errors, `--cred` with an unknown name, `store check` with no credential and a padded fake (the offline part, with the network calls failing cleanly in the container). Run `make check`. Commit: `feat(store): check, channels, roles and snaps`.

## 4. Verification

- [ ] 4.1 Run `openspec validate add-store-commands --strict`.
- [ ] 4.2 Pack the snap (`snapcraft clean idiot`, pack into a gitignored directory inside the tree) and run `tests/bare-host` on `local:`; all checks pass.
- [ ] 4.3 With the user, under the snap: `store check`, `channels`, `roles` and `snaps` against `ce-team-test` (admin) and `dsmr6` (viewer) give the expected results, and nothing they print contains a credential.
