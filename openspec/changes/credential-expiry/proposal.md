# Proposal

## Why

Every credential idiot creates expires (`idiot auth add --expires`,
default one year), and imported `snapcraft export-login` files do too, but
idiot only says so once, at creation, and in `idiot auth status` as a raw
timestamp that needs a network round trip the first time. The first sign
of an expired credential is a store rejection in the middle of a push or
release. idiot should know each credential's expiry offline, show it where
the user already looks, and warn shortly before it runs out.

## What Changes

- idiot records each saved credential's expiry next to it in
  `IDIOT_AUTH_DIR` (`.expires.<name>`, owner-only), read offline from the
  credential's own macaroon caveats by `idiot-store`, falling back to the
  store's `whoami` answer. `auth add`, `auth import` and `auth status`
  write the record; `auth login` fills a missing one where it can run the
  helper (source install); `auth remove` deletes it.
- New `idiot-store expiry [--file <file>]`: reads the credential from the
  file or `UBUNTU_STORE_AUTH` (never argv) and prints only the expiry
  timestamps found in its caveats, one per line, without touching the
  network. The credential and signatures are never printed.
- `idiot auth login` also exports `IDIOT_CRED_EXPIRES` (Unix time) when the
  expiry is known; `idiot auth logout` unsets it. Login says when the
  chosen credential has expired or expires soon.
- Shown as `expires in 41d` / `expires in 5h` / `expired`:
  - `idiot status`: on the auth row, next to the name and email;
  - `idiot auth list`: next to each name (`expiry unknown` when there is
    no record);
  - `idiot auth status`: after the `expires:` line.
  Within the warning window the phrase is yellow; expired is red.
- Warning: when the active credential expires within
  `IDIOT_CRED_WARN_DAYS` days (default 7; `0` turns off the early warning)
  or has expired, the first idiot command each day says so on stderr, once
  per credential per day across all shells.

stdout: unchanged for every existing command; `idiot-store expiry` (a
helper, not a user command) prints timestamps to stdout for the sh side.
Eval file: `auth login` additionally queues `export IDIOT_CRED_EXPIRES=…`
(or an unset of it when unknown), `auth logout` an unset. Warnings go to
stderr only. No new runtime dependencies: the helper gains a subcommand,
no new crates; GNU `date` (coreutils) is already used by `auth add`. The
snap builds the helper as before; nothing new to stage.

## Capabilities

### New Capabilities

### Modified Capabilities
- `auth`: an offline expiry record per credential, `IDIOT_CRED_EXPIRES`
  on login, expiry in `auth list` and `auth status`, the once-a-day expiry
  warning, and removal of the record with the credential.
- `status`: the auth row shows the active credential's expiry.

## Impact

- `tools/idiot-store`: new `expiry` subcommand (`src/cmd/expiry.rs`,
  caveat reading in `src/store/credential.rs`), unit tests, `main.rs` doc
  comment and exit-status list.
- `share/creds.sh`: record path, read/write helpers, the relative-time
  phrase, the warning check.
- `idiot` (the warning check where counted commands are dispatched),
  `cmd/auth/{add,import,login,logout,list,status,remove,help}`,
  `cmd/status`, README (auth section), CLAUDE.md (creds.sh helpers, state
  paths).
- Coordination: the in-flight `add-env-profiles` change adds a
  `.store.<name>` record in the same directory and also modifies the auth
  `Activating a credential` and `Removing a credential` requirements;
  whichever lands second merges those requirement texts. `add-doctor`
  can use the record for its credential checks.
