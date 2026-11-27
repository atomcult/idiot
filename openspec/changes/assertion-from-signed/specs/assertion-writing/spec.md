## MODIFIED Requirements

### Requirement: Editing an assertion
`idiot assertion edit <file|-> [--refresh] [--snap <name>[=<channel>]]...
[--set …] [--add …] [--no-edit] [-o <file>|-o -]` SHALL read a model,
system-user or validation-set assertion, either an unsigned JSON draft or
a signed assertion as snapd writes it, and ask for its headers as `new`
does, with the assertion's values as defaults. It SHALL offer to raise a
model's `revision` by one. For a validation set, it SHALL offer a new
revision of the same sequence (`revision` one more than the file's, or
than the store's for that sequence when that is higher) or a new sequence
(as above, `revision` left out). It SHALL update `timestamp` to the
current time. With `--refresh`, it SHALL look up every snap's `id` (and a
model snap's `type`) again, and a validation set's revisions from the
channel `--snap` names for each, asking for the channel on a terminal and
keeping the file's revision without one. It SHALL write a draft back to
the file it read unless `-o` is given.

A signed input SHALL become a draft holding every header of the assertion
as it was signed, as strings (lists and maps alike), without
`sign-key-sha3-384`, `body-length` or the signature. A stream of several
assertions (as `--chain` or `snap known` give) SHALL be accepted when
exactly one of them is a model, system-user or validation-set; the others
SHALL be ignored. A stream with none, or with more than one, SHALL fail
naming what it holds. A signed input SHALL never be written to: without
`-o`, the draft SHALL go to the file `new` would name in the current
directory (`<model>.json`, `<username>.json`, `<name>-<sequence>.json`,
from the edited values), refusing to overwrite an existing file. stderr
SHALL say which assertion it started from (type, primary keys and
revision) and that it was signed. Its signature SHALL NOT be checked.

#### Scenario: Model revision bumped
- **WHEN** the user runs `idiot assertion edit kiosk.json` on revision 2 and accepts the bump
- **THEN** `kiosk.json` has `"revision": "3"` and a new `timestamp`

#### Scenario: Revisions refreshed
- **WHEN** the user runs `idiot assertion edit base-set-4.json --refresh --snap core24=latest/stable --no-edit` and `core24`'s revision there is now 2322
- **THEN** the file's `core24` revision is `2322`

#### Scenario: Signed input
- **WHEN** the user runs `idiot assertion edit kiosk.model --no-edit` on a signed model `kiosk` at revision 2, with stdin not a terminal
- **THEN** `./kiosk.json` is written with `"revision": "3"`, the model's snaps and headers, and no `sign-key-sha3-384`, and `kiosk.model` is unchanged

#### Scenario: Draft beside it already exists
- **WHEN** the user edits the signed `kiosk.model` without `-o` and `./kiosk.json` exists
- **THEN** the command fails saying the file exists, and neither file changes

#### Scenario: A chain on stdin
- **WHEN** the user runs `idiot store assertion --chain --raw model my-brand/kiosk | idiot assertion edit - --no-edit -o -`
- **THEN** stdout holds the model's draft, and the account and account-key assertions in the stream are ignored

#### Scenario: Not one assertion of a writable type
- **WHEN** the input is a stream holding only an account and an account-key, or two models
- **THEN** the command fails saying what the stream holds, and writes nothing

## ADDED Requirements

### Requirement: Editing an assertion from the store
`idiot assertion edit model <brand-id>/<model>` and `idiot assertion edit
validation-set <account-id>/<name>[=<sequence>]` SHALL fetch the
assertion from the active store as `idiot store assertion` does (the
active or `--cred` credential, the latest sequence of a validation set
when none is given) and then behave as editing a signed input, including
the default output name. When the store doesn't serve it, the command
SHALL fail with the same message `idiot store assertion` gives, before
asking anything. Those options and the `-o`, `--set`, `--add`, `--snap`,
`--refresh` and `--no-edit` options SHALL work as for a file.

#### Scenario: Next revision of a published model
- **WHEN** the user runs `idiot assertion edit model my-brand/kiosk` and the store serves revision 5 of it
- **THEN** stderr says it started from `model my-brand/kiosk` revision 5 from the store, the bump offered is to 6, and `./kiosk.json` is written

#### Scenario: Unknown model
- **WHEN** the user runs `idiot assertion edit model my-brand/nope` and the store has no such model
- **THEN** the command fails with the not-found message, asks nothing and writes nothing

### Requirement: Starting from another assertion
`idiot assertion new <type> --from <file|->` and `idiot assertion new
<model|validation-set> --from-store <key>` (`<brand-id>/<model>` or
`<account-id>/<name>[=<sequence>]`, fetched as above) SHALL take a signed
assertion or a draft of the same type as a template. The new assertion
SHALL take its defaults from the template: its snaps (or validation-set
entries), its other headers and the fields idiot doesn't model. It SHALL
NOT take the template's identity:
- `authority-id` and `brand-id` (a validation set's `account-id`) SHALL
  default as `new`'s do, to the credential's account;
- a model's `model`, a validation set's `name` and `sequence`, and a
  system-user's `username`, `email`, `name` and password SHALL be asked
  (or given with `--set`) with no default from the template;
- `revision` SHALL be left out, and `timestamp`, `since` and `until`
  SHALL start from now as in `new`.

`--set`, `--add` and `--snap` SHALL win over the template. A template of
another type SHALL fail naming both types, and `--from` with `--from-store`
SHALL be refused. Everything else (prompts, editor, check, output file)
SHALL be as for `new`.

#### Scenario: A model from Canonical's
- **WHEN** the user runs `idiot assertion new model --from-store canonical/ubuntu-core-24-amd64 --set model=kiosk --no-edit` with stdin not a terminal and the credential's account `my-brand`
- **THEN** `./kiosk.json` has `"brand-id": "my-brand"`, `"authority-id": "my-brand"`, `"model": "kiosk"`, no `revision`, and the template's snaps with their IDs and channels

#### Scenario: Template of another type
- **WHEN** the user runs `idiot assertion new validation-set --from kiosk.model`
- **THEN** the command fails saying the template is a model, not a validation set

#### Scenario: Identity still required without a terminal
- **WHEN** the user runs `idiot assertion new model --from pc.model --no-edit` with stdin not a terminal and no `--set model=…`
- **THEN** the command fails naming the `model` header, and writes nothing
