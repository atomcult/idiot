## ADDED Requirements

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

## MODIFIED Requirements

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
