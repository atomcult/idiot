# Proposal

## Why

idiot manages four settings per shell (credential, store, arch, Launchpad
user), and a typical context switch ("work on the staging brand store as
the CI account, for arm64") is four separate commands that are easy to get
half right. The commonest half-right state is the most expensive one: a
credential for one store's account left active while `UBUNTU_STORE_ID`
points at another, which only shows up as a confusing store error several
commands later. Nothing in idiot today knows which store a credential goes
with: the credential (a store macaroon) and `idiot-store whoami` report the
account, permissions, snaps, channels and expiry, but no store.

## What Changes

- New command group `idiot profile`:
  - `profile save <name> [--fields <list>] [--force]` records the active
    credential name, store name, arch and Launchpad user (references by
    name, never credential contents) in `IDIOT_PROFILES_DIR`.
  - `profile use [<name>]` switches all recorded settings in one go,
    validating every reference before queueing anything, so a profile with
    a missing credential or store changes nothing. fzf picker without a
    name, as `auth login` and `store use`.
  - `profile list`, `profile remove [<name>]`, `profile status` (the active
    profile and which of its settings have since changed).
  - A field a profile doesn't record is left as is on `use`; a field
    recorded empty is unset (logged out, global store, host arch, no
    Launchpad user in the environment).
- Credential/store pairing: idiot records, per credential, the store ID it
  belongs to, next to the credential (`.store.<name>` in `IDIOT_AUTH_DIR`):
  - `auth add` records the store active when the credential is created;
  - new `auth bind [<name>]` (re)pairs a credential with the active store;
  - `auth remove` deletes the record. Credentials without a record
    (imported, or saved before this change) are never warned about.
- Mismatch guard: `idiot status`, `store use`, `store unset`, `auth login`
  and `profile use`/`save` warn on stderr when the active credential is
  paired with a different store than the active one. Warnings never change
  exit status or what is exported.
- `idiot status` gains a `profile` row while a profile is active.
- The emit logic of `auth login`/`logout`, `store use`/`unset` and the arch
  list of `arch set` move into share/ helpers that both those commands and
  `profile use` call, so they can't drift. Their behaviour is unchanged.
- stdout: unchanged for every command; `profile list` and `profile status`
  print on stderr like `store list`/`store status`. Eval file: `profile use`
  queues the same statements `auth login`/`logout`, `store use`/`unset`,
  `arch set`/`unset` and an `IDIOT_LP_USER` export/unset would, plus an
  export of the new `IDIOT_ACTIVE_PROFILE`. No other command's queued
  statements change.
- Runtime dependencies: none new (fzf is already bundled). Every profile
  command and `auth bind` touches only idiot's state, so all run confined
  under the snap; nothing new to stage.

## Capabilities

### New Capabilities
- `profile`: saving, listing, removing and activating named bundles of the
  credential, store, arch and Launchpad settings, and reporting the active
  one.

### Modified Capabilities
- `auth`: credentials gain a recorded store pairing (set by `auth add` and
  `auth bind`, removed with the credential); `auth login` warns on a
  mismatch with the active store.
- `store`: `store use` and `store unset` warn when the active credential is
  paired with a different store.
- `status`: a `profile` row, and a warning when the active credential and
  store don't match.

## Impact

- New: `cmd/profile/{help,save,use,list,remove,status}`, `cmd/auth/bind`,
  `share/profiles.sh`.
- Changed: `share/creds.sh` (pairing record, login/logout emitters,
  mismatch warning), `share/stores.sh` (name/ID lookup, use/unset
  emitters), `share/arch.sh` (the valid arch list), `cmd/auth/{login,logout,
  add,remove,help}`, `cmd/store/{use,unset}`, `cmd/arch/set`, `cmd/status`,
  `idiot` (top-level help), README, CLAUDE.md (layout, state paths).
- `tools/idiot-store` is unchanged: the store can't tell us a credential's
  store, so the pairing is idiot's own record.
- Independent of the in-flight `multi-arch-snap` and `vm-named-instances`
  changes.
