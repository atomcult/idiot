# Proposal

## Why

`idiot assertion edit` only reads the unsigned JSON an assertion was
signed from, and refuses a signed one. That JSON is often gone, or was
never on this machine: the published model is in the store, on a device
(`snap model --assertion`), or in someone else's repo. Two jobs are then
done by hand:
- making the next revision of a model or validation set that is already
  signed;
- starting a new one from someone else's, for example Canonical's
  `ubuntu-core-24-amd64` model, for its snaps and their channels.

A signed assertion holds every header the draft needs. Only
`sign-key-sha3-384` and the signature have to go. idiot can already parse
signed assertions (`idiot inspect assertion`) and fetch them
(`idiot store assertion`).

## What Changes

- **`idiot assertion edit <file|->` accepts a signed model, system-user or
  validation-set** as well as a JSON draft. A stream from `--chain` or
  `snap known` also works: the one assertion of those types in it is used,
  and account and account-key assertions are skipped.
  - It turns the assertion into a draft: every header kept, as strings,
    with `sign-key-sha3-384`, the signature and `body-length` dropped.
  - It then goes through the usual steps: ask for headers, offer the
    revision or sequence bump, update the timestamp, apply `--refresh`
    and `--snap`, open the editor and check.
  - It never writes back to a signed file. It writes the draft under
    `new`'s default name (`<model>.json`, `<username>.json`,
    `<name>-<sequence>.json`), or to `-o`, and refuses to overwrite.
  - **This replaces the current refusal:** "a signed assertion, not a JSON
    draft" is no longer an error.
- **`idiot assertion edit model <brand-id>/<model>` and
  `idiot assertion edit validation-set <account-id>/<name>[=<sequence>]`**
  fetch the assertion from the store (as `idiot store assertion` does: the
  active store and credential, `--cred`), then continue as for a signed
  file.
- **`idiot assertion new <type> --from <file|->`** and **`idiot assertion
  new <model|validation-set> --from-store <key>`** take a signed
  assertion or a draft as a template for a different assertion:
  - The template's snaps, base, grade, other headers and fields idiot
    doesn't model become the defaults.
  - Its identity is not carried over. `authority-id` and `brand-id` (or
    `account-id`) default to the credential's account, as in `new`. The
    model name or validation-set name, and a system-user's username,
    email, name and password, are asked afresh. `revision` is left out,
    and `timestamp`, `since` and `until` start from now.
  - A template of another type fails.
- **Messages:** stderr says where the starting point came from
  (`<brand>/<model> revision <n>`, signed or a draft, fetched or from a
  file). The signature is not checked: a template is only a set of
  defaults, and signing makes a new signature anyway.
- **stdout and the eval file are unchanged.** Nothing is printed on stdout
  except with `-o -`, nothing is queued in the eval file, and all of these
  remain host commands.
- **Dependencies:** none new. The parser comes from idiot-helper's
  assertion decoder, and the fetch from idiot-store's `assertion`
  subcommand, which the snap already ships.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `assertion-writing`:
  - "Editing an assertion" gains signed input (a file, stdin, or a stream
    holding one), the output naming for it, and loses the signed-input
    refusal.
  - New "Editing an assertion from the store" covers fetching by key.
  - New "Starting from another assertion" covers the `--from` and
    `--from-store` templates.

## Impact

- **`tools/idiot-helper/src/draft.rs`:** `draft read` accepts signed input
  through `assertion::decode_stream`. It emits a new record saying the
  input was signed, with the type, key headers and revision, which `draft
  write` ignores. It picks the one model, system-user or validation-set
  assertion in a stream. The `signed_and_foreign_input` test is replaced.
  No network code is added.
- **`cmd/assertion/edit`, `cmd/assertion/new`, `share/assertion.sh`:**
  - source arguments (positional `<type> <key>`, `-`, `--from`,
    `--from-store`);
  - the fetch, via `cmd/store/assertion --raw`;
  - template mode (which headers are dropped and asked again);
  - output naming for signed sources.
- **Help, docs and tests:**
  - help for `new` and `edit`, and the help group text;
  - `README.md` ("Writing assertions");
  - `skills/using-idiot/references/commands.md`;
  - `tests/bare-host.d/assertion-writer` (offline, from the bundled
    Canonical model, and, when the store is reachable, one anonymous
    fetch of `canonical/ubuntu-core-24-amd64`).
