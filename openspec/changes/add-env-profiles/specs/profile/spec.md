# Spec Delta

## Purpose

Named bundles of the four settings idiot manages in a shell (credential,
store, architecture, Launchpad user), so a whole working context can be
saved once and switched to with one command.

## ADDED Requirements

### Requirement: Profile storage
Profiles SHALL be saved one per file in `IDIOT_PROFILES_DIR` (default
`$XDG_DATA_HOME/idiot/profiles`, under the snap `~/snap/idiot/common/profiles`).
A profile SHALL record, for each of the fields `auth`, `store`, `arch` and
`lp`, either nothing (the field is not part of the profile), an empty value
(the setting is cleared), or a value: a saved credential's name, a saved
store's name, an architecture, or a Launchpad username. A profile SHALL
never contain credential contents or a store ID. Profile names SHALL be
made of letters, digits, `-` and `_`; any other name SHALL be rejected
without writing anything.

#### Scenario: References only
- **WHEN** the user saves a profile while credential `work` is active
- **THEN** the profile file holds the name `work` and none of the credential's contents

#### Scenario: Invalid name
- **WHEN** the user runs `idiot profile save ../x`
- **THEN** the command fails with an invalid-name error and nothing is written

### Requirement: Saving a profile
`idiot profile save <name>` SHALL record the current shell's settings:
- `auth`: the name of the active credential (`IDIOT_ACTIVE_CRED`), or empty
  when no credential is active;
- `store`: the name of the saved store whose ID is `UBUNTU_STORE_ID`, or
  empty when `UBUNTU_STORE_ID` is unset;
- `arch`: `UBUNTU_STORE_ARCH`, or empty when it is unset (the host
  default);
- `lp`: the active Launchpad username (as `idiot lp status` resolves it),
  or empty when there is none.
With `--fields <list>` (comma-separated, from `auth`, `store`, `arch`,
`lp`) it SHALL record only the listed fields. A field that cannot be
recorded by name, because the active credential has no name or
`UBUNTU_STORE_ID` matches no saved store, SHALL be left out of the profile
with a warning on stderr saying how to make it recordable. It SHALL refuse
an existing profile name unless `--force` is given, and an unknown field.
On success it SHALL print a confirmation on stderr listing what was
recorded.

#### Scenario: Save everything
- **WHEN** credential `ci` and store `staging` are active, `UBUNTU_STORE_ARCH` is `arm64` and the Launchpad user is `alice`, and the user runs `idiot profile save staging-ci`
- **THEN** profile `staging-ci` records `auth` `ci`, `store` `staging`, `arch` `arm64` and `lp` `alice`

#### Scenario: Cleared settings are recorded
- **WHEN** no credential is active, no store is set and no arch is set, and the user runs `idiot profile save global`
- **THEN** profile `global` records `auth`, `store` and `arch` as empty

#### Scenario: Some fields only
- **WHEN** the user runs `idiot profile save staging --fields auth,store`
- **THEN** the profile records `auth` and `store` and leaves out `arch` and `lp`

#### Scenario: Store ID without a saved name
- **WHEN** `UBUNTU_STORE_ID` matches no saved store and the user runs `idiot profile save x`
- **THEN** the profile leaves out `store`, and a warning suggests `idiot store add`

#### Scenario: Existing profile
- **WHEN** profile `staging-ci` exists and the user runs `idiot profile save staging-ci`
- **THEN** the command fails telling the user to use `--force` or remove it first, and the profile is unchanged

#### Scenario: Overwrite
- **WHEN** profile `staging-ci` exists and the user runs `idiot profile save staging-ci --force`
- **THEN** the profile records the current settings

### Requirement: Activating a profile
`idiot profile use [<name>]` SHALL, for each field the profile records:
- `auth` with a name: set the variables `idiot auth login <name>` sets;
  empty: unset the variables `idiot auth logout` unsets;
- `store` with a name: set `UBUNTU_STORE_ID` as `idiot store use <name>`
  does; empty: unset it;
- `arch` with a value: set `UBUNTU_STORE_ARCH`; empty: unset it;
- `lp` with a value: set `IDIOT_LP_USER`; empty: unset it;
and leave the settings of fields it does not record unchanged. It SHALL set
`IDIOT_ACTIVE_PROFILE` to the profile's name. It SHALL NOT write or delete
the saved Launchpad username (`idiot lp set`'s file): a profile changes the
current shell only. Before changing anything it SHALL check every recorded
reference and value; if a credential or store it names doesn't exist, an
arch is not one `idiot arch set` accepts, or the file can't be read as a
profile, it SHALL fail naming the profile and the problem, and change no
variable. Without a name it SHALL offer an fzf picker of saved profiles
whose preview shows the profile's fields; cancelling SHALL change nothing.

#### Scenario: Use
- **WHEN** profile `staging-ci` records `ci`, `staging`, `arm64` and `alice`, and the user runs `idiot profile use staging-ci`
- **THEN** the current shell has credential `ci` active, `UBUNTU_STORE_ID` set to `staging`'s ID, `UBUNTU_STORE_ARCH` `arm64`, `IDIOT_LP_USER` `alice` and `IDIOT_ACTIVE_PROFILE` `staging-ci`

#### Scenario: Cleared fields
- **WHEN** profile `global` records `auth`, `store` and `arch` as empty, and the user runs `idiot profile use global`
- **THEN** no credential is active, `UBUNTU_STORE_ID` and `UBUNTU_STORE_ARCH` are unset, and `IDIOT_LP_USER` is unchanged

#### Scenario: Fields left out
- **WHEN** profile `staging` records only `auth` and `store`, `UBUNTU_STORE_ARCH` is `riscv64`, and the user runs `idiot profile use staging`
- **THEN** `UBUNTU_STORE_ARCH` is still `riscv64`

#### Scenario: Missing credential
- **WHEN** profile `staging-ci` names credential `ci`, which has been removed, and the user runs `idiot profile use staging-ci`
- **THEN** the command fails naming profile `staging-ci` and credential `ci`, and no variable changes, including the store

#### Scenario: Unknown profile
- **WHEN** the user runs `idiot profile use nope`
- **THEN** the command fails with `profile 'nope' not found` and no variable changes

#### Scenario: Picker cancelled
- **WHEN** the user runs `idiot profile use` and cancels the picker
- **THEN** the command exits zero and no variable changes

#### Scenario: Saved Launchpad user untouched
- **WHEN** `alice` is saved with `idiot lp set` and the user uses a profile recording `lp` `bob`
- **THEN** `IDIOT_LP_USER` is `bob` in the current shell, and a new shell still resolves `alice`

### Requirement: Listing and removing profiles
`idiot profile list` SHALL print on stderr each saved profile's name and
recorded fields, marking the active one, or a hint to use
`idiot profile save` when none are saved. `idiot profile remove [<name>]`
SHALL delete a saved profile, offering an fzf picker when no name is given,
and SHALL fail with `profile '<name>' not found` for an unknown name.
Removing a profile SHALL NOT change the current shell's settings or any
credential or store.

#### Scenario: Empty list
- **WHEN** no profiles are saved and the user runs `idiot profile list`
- **THEN** a hint to use `idiot profile save` is printed on stderr and nothing on stdout

#### Scenario: Remove
- **WHEN** the user runs `idiot profile remove staging-ci`
- **THEN** the profile is deleted, and credential `ci` and store `staging` still exist

### Requirement: Active profile status
`idiot profile status` SHALL print on stderr the name of the active profile
(`IDIOT_ACTIVE_PROFILE`) and, for each field the profile records whose
current setting differs from it, the field with the recorded and current
values; or `no profile active` when none is. A profile that has since been
removed SHALL be reported as removed.

#### Scenario: Unchanged
- **WHEN** the user has run `idiot profile use staging-ci` and changed nothing since
- **THEN** `idiot profile status` prints `staging-ci` and no changed fields

#### Scenario: Drifted
- **WHEN** after `idiot profile use staging-ci` the user runs `idiot arch set amd64`
- **THEN** `idiot profile status` prints `staging-ci` and reports `arch` as `arm64` in the profile, `amd64` now

#### Scenario: None active
- **WHEN** no profile has been used in this shell
- **THEN** `idiot profile status` prints `no profile active` and exits zero

### Requirement: Profiles run confined
Every `idiot profile` command SHALL use nothing but idiot's own state and
fzf, and so SHALL run confined under the snap rather than being handed to
the host.

#### Scenario: Under the snap
- **WHEN** idiot is installed as a snap and the user runs `idiot profile use staging-ci`
- **THEN** the switch happens without a host handoff, and works without any of the snap's optional components installed
