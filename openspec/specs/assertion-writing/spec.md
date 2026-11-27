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
optional `display-name`. For an Ubuntu Core model (see "Classic models"),
before asking for other snaps, it SHALL propose the essential snaps for
that base and architecture:
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

#### Scenario: An Ubuntu Core model is still the default
- **WHEN** the user runs `idiot assertion new model --no-edit -o -` without a terminal, with no `--classic` and no `classic` header given
- **THEN** the draft has no `classic` or `distribution` header, and lists a gadget and a kernel

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
- for a model with a `base`, the extended `snaps` header, with no repeated
  snap name or ID, essential snaps without `presence` or `modes`, an `id`
  on every snap unless the grade is `dangerous`, and `default-channel`
  with a track;
- an Ubuntu Core model's exactly one gadget and one kernel; a classic
  model's at most one of each, a kernel only with a gadget;
- `distribution` in a classic model with `snaps`, matching an os-release
  ID (lowercase letters, digits, `.`, `_` and `-`), and in no other model;
- no `kernel` or `base` header in a classic model without `snaps`;
- `classic: true` only for app snaps, and only in a classic model;
- snap `type` and `presence` values;
- a validation set's revisions and presences, and its snaps' components:
  no revision on an `invalid` component, and a revision on every other
  component exactly when its snap has one;
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

#### Scenario: Classic model without a distribution
- **WHEN** a model with `classic` `true` and a `snaps` list has no `distribution` and the user runs `idiot assertion check desk.json`
- **THEN** stderr says `distribution` is required in a classic model, and the command exits 1

#### Scenario: Distribution in an Ubuntu Core model
- **WHEN** a model without `classic` has `"distribution": "ubuntu"`
- **THEN** `check` reports that `distribution` is only for classic models, and exits 1

#### Scenario: Classic kernel without a gadget
- **WHEN** a classic model's `snaps` lists `pc-kernel` and no gadget
- **THEN** `check` reports that a classic model can list a kernel only with a gadget, and exits 1

#### Scenario: Classic model without kernel or gadget
- **WHEN** a classic model's `snaps` lists `snapd`, its base and app snaps, with `distribution` `ubuntu`
- **THEN** `check` reports no error

#### Scenario: Old-style classic model with a kernel
- **WHEN** a model with `classic` `true` and no `snaps` list has a `kernel` header
- **THEN** `check` reports that `kernel` can't be given in a classic model without `snaps`, and exits 1

#### Scenario: Component revision in a validation set
- **WHEN** a validation set's snap `app` has `revision` `12` and its component `app+extra` has no revision and presence `required`
- **THEN** `check` names `app+extra` and says it needs a revision since its snap has one, and exits 1

#### Scenario: Revision on an invalid component
- **WHEN** a component has `presence` `invalid` and a `revision`
- **THEN** `check` names the component and says an invalid component can't have a revision, and exits 1

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
- a model's `model` and `display-name`, a validation set's `name` and `sequence`, and a
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

### Requirement: Classic models
`idiot assertion new model` SHALL write an Ubuntu Core model or a classic
model. On a terminal it SHALL ask which, defaulting to Ubuntu Core, unless
the kind is already given: by `--classic`, by `--set classic=true` or
`--set classic=false`, or by the template of `--from` or `--from-store`.
Without a terminal and without any of those, it SHALL write an Ubuntu
Core model. `idiot assertion edit` SHALL keep the kind the draft has and
not ask.

For a classic model the writer SHALL:
- set `classic` to `true`;
- ask for `distribution` (default `ubuntu`), refusing a value that isn't
  an os-release ID (lowercase letters, digits, `.`, `_` and `-`);
- propose `snapd` and the base in `latest/stable`, as for an Ubuntu Core
  model;
- ask for a gadget with no default, where an empty answer means none;
- ask for a kernel only when a gadget was given, where an empty answer
  again means none;
- without a terminal, add a gadget or kernel only when `--snap` names one;
- give `classic: true` to every app snap the store releases with classic
  confinement in the chosen channel.

For an Ubuntu Core model, a snap the store releases with classic
confinement SHALL be added with a warning on stderr that an Ubuntu Core
device can't install it.

#### Scenario: Classic without a terminal
- **WHEN** the user runs `idiot assertion new model --classic --set model=desk --no-edit -o -` without a terminal
- **THEN** the draft has `classic` `true`, `distribution` `ubuntu`, and `snaps` holding `snapd` and `core24` but no gadget or kernel, and `idiot assertion check` passes it

#### Scenario: Asked on a terminal
- **WHEN** the user runs `idiot assertion new model` on a terminal and answers `classic`
- **THEN** the writer asks for `distribution` and then a gadget, and asks for a kernel only if a gadget was given

#### Scenario: Classic-confined app snap
- **WHEN** a classic model adds a snap the store releases with classic confinement in `latest/stable`
- **THEN** its entry has `classic` `true`

#### Scenario: Classic-confined snap in an Ubuntu Core model
- **WHEN** an Ubuntu Core model adds a snap the store releases with classic confinement
- **THEN** the snap is added without `classic`, and stderr warns that an Ubuntu Core device can't install it

#### Scenario: Editing keeps the kind
- **WHEN** the user runs `idiot assertion edit desk.json` on a classic model
- **THEN** the writer doesn't ask which kind of model it is, keeps `classic` and `distribution`, and doesn't propose a gadget or kernel

#### Scenario: Invalid distribution
- **WHEN** the user runs `idiot assertion new model --classic --set distribution=Ubuntu`
- **THEN** the command fails saying `Ubuntu` isn't an os-release ID, and writes nothing
