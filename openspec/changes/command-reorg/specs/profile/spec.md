## MODIFIED Requirements

### Requirement: Activating a profile
`idiot profile use [<name>]` SHALL, for each field the profile records:
- `auth` with a name: set the variables `idiot auth use <name>` sets;
  empty: unset the variables `idiot auth unset` unsets;
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

## ADDED Requirements

### Requirement: Clearing the active profile
`idiot profile unset` SHALL clear what the active profile
(`IDIOT_ACTIVE_PROFILE`) set: for each field the profile records, the
variables that field sets (as `idiot auth unset`, `idiot store unset`,
`idiot arch unset` and clearing `IDIOT_LP_USER`), and then
`IDIOT_ACTIVE_PROFILE`. Fields the profile doesn't record SHALL be left
as they are. It SHALL NOT write or delete any saved state (credentials,
stores, the saved Launchpad username or the profile). With no profile
active it SHALL say so and exit zero; when the active profile has since
been removed it SHALL unset only `IDIOT_ACTIVE_PROFILE`, saying why. It
SHALL run confined, like every `idiot profile` command.

#### Scenario: Unset
- **WHEN** the user has run `idiot profile use staging-ci` (recording `ci`, `staging`, `arm64` and `alice`) and runs `idiot profile unset`
- **THEN** no credential is active, `UBUNTU_STORE_ID`, `UBUNTU_STORE_ARCH`, `IDIOT_LP_USER` and `IDIOT_ACTIVE_PROFILE` are unset, and credential `ci` and store `staging` still exist

#### Scenario: Fields left out
- **WHEN** profile `staging` records only `auth` and `store`, `UBUNTU_STORE_ARCH` is `riscv64`, and the user runs `idiot profile use staging` then `idiot profile unset`
- **THEN** `UBUNTU_STORE_ARCH` is still `riscv64`

#### Scenario: None active
- **WHEN** no profile has been used in this shell and the user runs `idiot profile unset`
- **THEN** stderr says no profile is active, nothing changes, and the exit status is zero

#### Scenario: Removed profile
- **WHEN** the active profile `old` has since been removed and the user runs `idiot profile unset`
- **THEN** only `IDIOT_ACTIVE_PROFILE` is unset and stderr says `old` no longer exists
