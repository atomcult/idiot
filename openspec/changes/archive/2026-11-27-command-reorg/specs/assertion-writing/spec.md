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
- **WHEN** the user runs `idiot assertion fetch --chain --raw model my-brand/kiosk | idiot assertion edit - --no-edit -o -`
- **THEN** stdout holds the model's draft, and the account and account-key assertions in the stream are ignored

#### Scenario: Not one assertion of a writable type
- **WHEN** the input is a stream holding only an account and an account-key, or two models
- **THEN** the command fails saying what the stream holds, and writes nothing

### Requirement: Signing an assertion
`idiot assertion sign <file.json> [-k <key>] [--chain] [-o <file>|-o -]`
SHALL run `check` and then sign the assertion with `snap sign`, passing the
key's name with `-k` and `--chain` when given. Without `-k` it SHALL pick
the key on a terminal from the keys `snap keys` lists, preselecting a key
registered to the assertion's `authority-id`. Without a terminal it SHALL
use snap's `default` key. With no keys, it SHALL fail saying how to create
one (`snap create-key <name>`) and register it (`snapcraft register-key
<name>`). Before signing, it SHALL look up the key's account-key in the
store. It SHALL warn on stderr, and still sign, when the key isn't
registered, is registered to another account than `authority-id`, or the
store can't be reached. It SHALL write the signed assertion:
- to `-o <file>`, or to stdout with `-o -`;
- by default beside the input, as `<name>.model` for a model and
  `<name>.assert` otherwise;

refusing to overwrite an existing file. A system-user's file SHALL be mode
0600. It SHALL then report the signed assertion's signature status on
stderr, as `idiot assertion show --remote` determines it. It SHALL exit
non-zero, writing nothing, when `check` finds an error or `snap sign`
fails. It SHALL never read, copy, export or print a private key or
passphrase, and SHALL leave the passphrase prompt to `snap sign`. It SHALL
be a host command.

#### Scenario: Signed and verified
- **WHEN** the user runs `idiot assertion sign kiosk.json -k brand-key` and `brand-key` is registered to the model's `authority-id`
- **THEN** `kiosk.model` holds the signed model (starting `type: model`), and stderr reports it `verified`

#### Scenario: Key for another account
- **WHEN** the chosen key is registered to account `xyz789` and the model's `authority-id` is `abcDEF123`
- **THEN** stderr warns that the key belongs to `xyz789`, not `abcDEF123`, before `snap sign` runs

#### Scenario: Invalid draft
- **WHEN** the user signs a draft that `check` reports an error for
- **THEN** the errors are printed, `snap sign` isn't run, and nothing is written

#### Scenario: No keys
- **WHEN** `snap keys` lists no keys and the user runs `idiot assertion sign kiosk.json`
- **THEN** the command fails explaining `snap create-key` and `snapcraft register-key`

#### Scenario: To stdout
- **WHEN** the user runs `idiot assertion sign kiosk.json -k brand-key -o - | idiot assertion show -`
- **THEN** stdout carries only the signed assertion, and the inspection summarises it

### Requirement: Editing an assertion from the store
`idiot assertion edit model <brand-id>/<model>` and `idiot assertion edit
validation-set <account-id>/<name>[=<sequence>]` SHALL fetch the
assertion from the active store as `idiot assertion fetch` does (the
active or `--cred` credential, the latest sequence of a validation set
when none is given) and then behave as editing a signed input, including
the default output name. When the store doesn't serve it, the command
SHALL fail with the same message `idiot assertion fetch` gives, before
asking anything. Those options and the `-o`, `--set`, `--add`, `--snap`,
`--refresh` and `--no-edit` options SHALL work as for a file.

#### Scenario: Next revision of a published model
- **WHEN** the user runs `idiot assertion edit model my-brand/kiosk` and the store serves revision 5 of it
- **THEN** stderr says it started from `model my-brand/kiosk` revision 5 from the store, the bump offered is to 6, and `./kiosk.json` is written

#### Scenario: Unknown model
- **WHEN** the user runs `idiot assertion edit model my-brand/nope` and the store has no such model
- **THEN** the command fails with the not-found message, asks nothing and writes nothing
