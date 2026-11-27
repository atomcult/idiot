# assertion-writing Specification

## Purpose

Writing model, system-user and validation-set assertions with `idiot
assertion`: unsigned drafts filled from the environment and the store,
checked and edited, then signed with `snap sign` and the user's key.

## Requirements

### Requirement: Writing an assertion
`idiot assertion new <model|system-user|validation-set> [--set
<header>=<value>]... [--add <header>=<value>]... [--snap
<name>[=<channel>]]... [--arch <arch>] [--cred <name>] [--no-edit] [-o
<file>|-o -]` SHALL write an unsigned assertion of that type as a JSON
object whose scalar values are all strings, with `type` set. A list or map
header holds strings in the same way. `--set` SHALL give a scalar header
and `--add` SHALL append to a list header (`models`, `ssh-keys`,
`serials`, `system-user-authority`, `serial-authority`). On a terminal it
SHALL ask, on stderr, for each header of the type that wasn't given,
showing a default where there is one, so that Enter accepts the default.
Without a terminal it SHALL take only the given values and defaults, and
fail before contacting anything, naming the first required header that is
missing. It SHALL write to `-o <file>`, to stdout with `-o -`, or by
default to a file in the current directory:
- `<model>.json` for a model;
- `<username>.json` for a system-user;
- `<name>-<sequence>.json` for a validation set.

It SHALL refuse to overwrite an existing file and SHALL say on stderr where
it wrote. Cancelling a prompt or picker SHALL write nothing and exit zero.
It SHALL print nothing on stdout except with `-o -`, and SHALL queue
nothing in the eval file. It SHALL be a host command.

#### Scenario: Model without prompts
- **WHEN** the user runs `idiot assertion new model --set model=kiosk --set base=core24 --set grade=signed --snap pc=24/stable --snap pc-kernel=24/stable --snap core24 --snap snapd --no-edit -o -` with stdin not a terminal
- **THEN** stdout holds one JSON object with `"type": "model"`, `"model": "kiosk"`, `"series": "16"`, every scalar a string, and four entries in `snaps`

#### Scenario: Required value missing without a terminal
- **WHEN** the user runs `idiot assertion new validation-set --no-edit` with stdin not a terminal and no `--set name=…`
- **THEN** the command fails naming the `name` header, and contacts nothing

#### Scenario: Default file name
- **WHEN** the user answers `kiosk` for the model name and accepts the other defaults
- **THEN** `./kiosk.json` is written and stderr says so

#### Scenario: Existing file
- **WHEN** `./kiosk.json` exists and the user writes a model named `kiosk` without `-o`
- **THEN** the command fails saying the file exists, and the file is unchanged

### Requirement: Defaults from the environment
The writer SHALL default:
- `authority-id`, `brand-id` (model, system-user) and `account-id`
  (validation set) to the account ID behind the credential (the active
  one, or `--cred <name>`);
- `series` to `16`;
- a model's `architecture` to `--arch`, else `UBUNTU_STORE_ARCH`, else the
  host's;
- a model's `store` to `UBUNTU_STORE_ID`, leaving `store` out when the
  global store is active;
- `timestamp` (model, validation set) and a system-user's `since` to the
  current UTC time, and `until` to one year after `since`.

With no credential it SHALL still work, without account defaults: those
headers SHALL be asked for, or be required with `--set` when there is no
terminal. A credential SHALL be used only to read the account ID and to
see the store as that account does.

#### Scenario: Brand from the credential
- **WHEN** the active credential belongs to account `abcDEF123` and the user runs `idiot assertion new model`
- **THEN** the `authority-id` and `brand-id` prompts both default to `abcDEF123`

#### Scenario: Brand store active
- **WHEN** `UBUNTU_STORE_ID` is `acme-store-id` and the user writes a model accepting the defaults
- **THEN** the model's `store` header is `acme-store-id`

#### Scenario: Global store active
- **WHEN** no store is active and the user writes a model accepting the defaults
- **THEN** the model has no `store` header

### Requirement: Snaps from the store
For each snap in a model or validation set, the writer SHALL take the name
from `--snap`, or on a terminal from a picker. The picker offers the snaps
the credential sees in the active store and accepts a typed name. It SHALL
then take the channel from `--snap <name>=<channel>`, or on a terminal
from a picker of the snap's channels that have a revision for the target
architecture. It SHALL look the snap up in the active store for the target
architecture, as the credential sees it, and fill in:
- its `id`, for both types;
- for a model, its `type` from the store, and `default-channel` as the
  channel, written with a track (`latest/` added to a bare risk);
- for a validation set, its `revision` in that channel for the target
  architecture.

When the snap has a different revision in that channel for another
architecture, it SHALL say so on stderr, since a validation set's
revision holds for every architecture. A snap the store doesn't show SHALL
be left without an `id` in a `dangerous` model, with a note on stderr. In
any other model or validation set it SHALL be an error naming the snap and
suggesting `idiot store check <snap>`. A channel with no revision for the
target architecture SHALL be an error listing the channels that have one.
A validation-set snap's `presence` SHALL default to `required`, and
`--snap <name>=invalid` SHALL mark it `invalid` with no revision.

#### Scenario: IDs and types filled
- **WHEN** the user adds `pc-kernel` with channel `24/stable` to an amd64 model
- **THEN** its entry has `"name": "pc-kernel"`, `"id": "pYVQrBcKmBa0mZ4CCN7ExT6jH8rY1hza"`, `"type": "kernel"` and `"default-channel": "24/stable"`

#### Scenario: Revision from a channel
- **WHEN** the user runs `idiot assertion new validation-set --snap core24=latest/stable --arch amd64` and `core24`'s amd64 revision in `latest/stable` is 2307
- **THEN** the `core24` entry has `"revision": "2307"` and `"presence": "required"`

#### Scenario: Bare risk
- **WHEN** the user gives `--snap snapd=stable` for a model
- **THEN** the entry's `default-channel` is `latest/stable`

#### Scenario: Local snap in a dangerous model
- **WHEN** the model's grade is `dangerous` and the store doesn't show `my-app`
- **THEN** `my-app`'s entry has no `id`, and stderr says the ID was left out because the store doesn't show the snap

#### Scenario: Unknown snap in a signed model
- **WHEN** the model's grade is `signed` and the store doesn't show `my-app`
- **THEN** the writer reports an error naming `my-app` and suggesting `idiot store check my-app`, and writes nothing until the snap is removed or the grade changed

#### Scenario: Not released for the target
- **WHEN** the user gives `--snap pc=24/edge` and `pc` has no amd64 revision in `24/edge`
- **THEN** the writer fails listing the channels where `pc` has an amd64 revision

### Requirement: Model essentials
For a model, the writer SHALL ask for `model`, `base` (default `core24`),
`grade` (`dangerous`, `signed` or `secured`; default `signed`) and an
optional `display-name`. Before asking for other snaps, it SHALL propose
the essential snaps for that base and architecture:
- `snapd` in `latest/stable`;
- the base in `latest/stable`;
- for amd64, `pc` and `pc-kernel`, and for arm64 and armhf, `pi` and
  `pi-kernel`, each in the base's track (`24/stable` for `core24`).

For any other architecture, it SHALL ask for the gadget and kernel with no
default. The user SHALL be able to replace each essential snap.
Essential snaps SHALL carry no `presence` or `modes`. Other snaps SHALL
default to `presence` `required`. A model SHALL be able to list validation
sets (`account-id`, `name`, optional `sequence`, `mode` `enforce` or
`prefer-enforce`).

#### Scenario: Essentials for core24 on amd64
- **WHEN** the user writes an amd64 model with base `core24` and accepts the proposed essentials
- **THEN** `snaps` holds `pc` and `pc-kernel` with `default-channel` `24/stable`, and `core24` and `snapd` with `latest/stable`, none with `presence`

#### Scenario: Other architecture
- **WHEN** the target architecture is `riscv64`
- **THEN** the writer asks for the gadget and the kernel without proposing one

### Requirement: System users
For a system-user, the writer SHALL ask for `email`, `username` (a
lowercase letter or digit, then lowercase letters, digits, `-`, `.` or
`_`), an optional `name`, the `models` it is for, and the SSH public keys
to include. On a terminal it SHALL offer model names from the saved model
repositories whose `brand-id` matches, and the files `~/.ssh/*.pub`. A
password SHALL be optional. When the user asks for one, it SHALL be read
twice from the terminal without echo, and SHALL be hashed with SHA-512
crypt by the host's `openssl` (failing, if `openssl` is missing, with a
hint to install it or use SSH keys). Only the hash SHALL be written. The
plain password SHALL never be written to a file, passed on a command line,
put in the environment or shown. `--set password=<hash>` SHALL accept only
an existing hash. A system-user file, unsigned or signed, SHALL be created
with mode 0600. The writer SHALL set `format` to `1` when `serials` is
given and to `2` when `user-presence` is, as snapd requires.

#### Scenario: Password hashed
- **WHEN** the user asks for a password and types the same one twice
- **THEN** the file's `password` header starts with `$6$`, the typed password appears nowhere in the file, the terminal or the process list, and the file's mode is 0600

#### Scenario: Plain text refused
- **WHEN** the user runs `idiot assertion new system-user --set password=hunter2`
- **THEN** the command fails saying `password` must be a crypt(3) hash of type 6 or higher

#### Scenario: Serials need format 1
- **WHEN** the user adds `--add serials=SN123` with one model
- **THEN** the written assertion has `"format": "1"`

### Requirement: Validation-set sequences
For a validation set, the writer SHALL ask for `name` (lowercase letters
and digits, with single `-` between them) and `sequence`. The sequence
SHALL default to one more than the latest sequence the store has for that
account and name, and to `1` when the store has none or can't be asked. It
SHALL fill `timestamp` and leave `revision` out of a new sequence.

#### Scenario: Next sequence
- **WHEN** the store's latest `abcDEF123/base-set` is sequence 3 and the user writes a validation set named `base-set`
- **THEN** the sequence prompt defaults to `4`

#### Scenario: New name
- **WHEN** the store has no validation set `abcDEF123/new-set`
- **THEN** the sequence defaults to `1`

### Requirement: Editing an assertion
`idiot assertion edit <file.json> [--refresh] [--snap <name>[=<channel>]]...
[--set …] [--add …] [--no-edit] [-o <file>|-o -]` SHALL read an unsigned
assertion of one of the three types and ask for its headers as `new` does,
with the file's values as defaults. It SHALL offer to raise a model's
`revision` by one. For a validation set, it SHALL offer a new revision of
the same sequence (`revision` one more than the file's, or than the
store's for that sequence when that is higher) or a new sequence (as
above, `revision` left out). It SHALL update `timestamp` to the current
time. With `--refresh`, it SHALL look up every snap's `id` (and a model
snap's `type`) again, and a validation set's revisions from the channel
`--snap` names for each, asking for the channel on a terminal and keeping
the file's revision without one. It SHALL write back to the file it read
unless `-o` is given, and fail on a signed assertion, saying to edit the
JSON it was signed from.

#### Scenario: Model revision bumped
- **WHEN** the user runs `idiot assertion edit kiosk.json` on revision 2 and accepts the bump
- **THEN** `kiosk.json` has `"revision": "3"` and a new `timestamp`

#### Scenario: Revisions refreshed
- **WHEN** the user runs `idiot assertion edit base-set-4.json --refresh --snap core24=latest/stable --no-edit` and `core24`'s revision there is now 2322
- **THEN** the file's `core24` revision is `2322`

#### Scenario: Signed input
- **WHEN** the user runs `idiot assertion edit kiosk.model`
- **THEN** the command fails saying it is a signed assertion, and the file is unchanged

### Requirement: Checking an assertion
`idiot assertion check <file.json|->` SHALL check an unsigned model,
system-user or validation-set assertion against the rules snapd applies
when it signs, and print each problem on stderr, naming the header (and
the snap, for a snap entry). Errors SHALL make it exit 1, and warnings
alone SHALL leave it at 0. It SHALL make no network request. The rules
SHALL include:
- the type's required headers;
- `authority-id` equal to `brand-id` (model) or `account-id` (validation
  set);
- the name rules for `model`, a validation set's `name`, snap names and
  `username`;
- `grade` and `storage-safety` values;
- for a model with a `base`, the extended `snaps` header, with exactly one
  gadget and one kernel, no repeated snap name or ID, essential snaps
  without `presence` or `modes`, an `id` on every snap unless the grade is
  `dangerous`, and `default-channel` with a track;
- snap `type` and `presence` values;
- a validation set's revisions and presences;
- a system-user's email form, password hash form, `until` after `since`,
  at most 365 days when `models` is empty, `serials` only with exactly one
  model, and the `format` its headers need;
- scalars that aren't strings.

Unless `--no-edit` is given, `new` and `edit` SHALL open the draft in the
user's editor (`VISUAL`, `EDITOR`, then `editor`, then `vi`), check it on
save, and reopen it with the problems listed until it passes or the user
declines. Nothing SHALL be written to the destination while the draft has
errors.

#### Scenario: Missing track
- **WHEN** a model's `pc` entry has `"default-channel": "stable"` and the user runs `idiot assertion check kiosk.json`
- **THEN** stderr names `pc` and says `default-channel` must give a track, and the command exits 1

#### Scenario: Missing ID in a signed model
- **WHEN** a `signed` model lists a snap without `id`
- **THEN** `check` reports that `id` is mandatory for a `signed` model, naming the snap, and exits 1

#### Scenario: Global system-user too long
- **WHEN** a system-user with no `models` has `until` 400 days after `since`
- **THEN** `check` reports that `until` can be at most 365 days after `since` when no models are given

#### Scenario: Fixing in the editor
- **WHEN** the user saves a draft with an error and chooses to edit again
- **THEN** the editor reopens with the problem listed, and the destination is written only once the draft passes

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
stderr, as `idiot inspect assertion --remote` determines it. It SHALL exit
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
- **WHEN** the user runs `idiot assertion sign kiosk.json -k brand-key -o - | idiot inspect assertion -`
- **THEN** stdout carries only the signed assertion, and the inspection summarises it
