# Proposal

## Why

A credential is an Ubuntu One login to the Snap Store, and the same login
usually works in the global store and in every brand store its account
belongs to. idiot pairs each credential it adds with the store active at the
time, so a personal login added on the global store warns every time a
brand store is active:

```
warning: credentials 'personal' belong to the global store, but store 'dsmr6' is active
         switch: idiot store unset · re-pair: idiot auth bind personal
```

Both suggested fixes are wrong for that credential: it belongs to all of
them. A tester asked for a way to mark such a credential "for all stores".
Unpaired credentials already aren't warned about, but nothing lets a user
choose that, and `auth list` doesn't show which credential belongs where.

## What Changes

- `idiot auth bind --any [<name>]` marks a credential (the active one by
  default) as for any store: it is never warned about, whichever store is
  active. `idiot auth bind [<name>]` without `--any` pairs it with the
  active store again, as now.
- `idiot auth add --any <name>` saves a new credential for any store
  instead of pairing it with the active one. Without `--any`, `auth add`
  pairs as now; the default doesn't change, because a store-specific
  credential (staging, CI) is where the warning catches mistakes.
- The mismatch warning offers the new fix as well:
  `switch: idiot store unset · re-pair: idiot auth bind personal · any store: idiot auth bind --any personal`.
- `idiot auth list` shows each credential's store after its expiry: the
  saved store's name (else its ID), `global`, or `any store` (marked, or
  never paired).
- The marker is recorded in the existing pairing record (`.store.<name>`,
  no credential contents), so `auth remove` already deletes it.

Output: nothing new on stdout and nothing new in the eval file; `auth bind`
and `auth add` keep their messages on stderr, and `auth list` writes to
stderr as now. No new runtime dependencies; nothing for the snap to stage.
Confined as now: `auth bind` and `auth list` only touch idiot's state;
`auth add` stays a host command.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `auth`: credential store pairing gains "any store" (`auth bind --any`,
  `auth add --any`); the store mismatch warning offers it; `auth list`
  shows each credential's store.

## Impact

- `share/creds.sh`: the pairing record learns the any-store value
  (`record_cred_store`, `cred_store_id` or a new reader, a label helper);
  `warn_store_mismatch` skips it and prints the third fix.
- `cmd/auth/bind`, `cmd/auth/add`: `--any`, help text.
- `cmd/auth/list`: the store column.
- `tests/bare-host.d/profile` (pairing checks): `bind --any`, the warning's
  third fix, `auth list`.
- `README.md` (Credentials and stores) and the using-idiot skill's command
  reference.
- An older idiot reading an any-store record would treat it as paired with
  an unknown store and warn; downgrades are rare, and re-running
  `idiot auth bind` there fixes it.
