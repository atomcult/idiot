# status Specification

## Purpose

A one-screen summary of the environment idiot manages in the current shell:
credential, store, architecture, Launchpad identity, and usage level.

## Requirements

### Requirement: Environment summary
`idiot status` SHALL print on stderr one row each for auth, store, arch and
lp, without contacting the network:
- auth: `logged in` when `UBUNTU_STORE_AUTH` or `SNAPCRAFT_STORE_CREDENTIALS`
  is set, annotated with the active credential name and, if cached, its
  account email; otherwise `not logged in`.
- store: the saved store name whose ID matches `UBUNTU_STORE_ID` (with the
  ID), the bare ID if no saved store matches, or `global`.
- arch: `UBUNTU_STORE_ARCH`, or the host architecture marked `host default`.
- lp: the active Launchpad username, or `not set`.
When `IDIOT_ACTIVE_PROFILE` is set it SHALL print a `profile` row first,
with the profile's name, annotated `changed` when any setting the profile
records now differs from it, or `removed` when the profile no longer
exists. Below the rows it SHALL print the auth capability's store mismatch
warning when the active credential is paired with a different store than
the active one.

#### Scenario: Fresh shell
- **WHEN** nothing has been activated
- **THEN** the rows read `not logged in`, `global`, the host arch marked `host default`, and `not set` (unless a username was saved), with no profile row and no warning

#### Scenario: Named store
- **WHEN** `UBUNTU_STORE_ID` matches the saved store `staging`
- **THEN** the store row shows `staging` and its ID

#### Scenario: Unknown store ID
- **WHEN** `UBUNTU_STORE_ID` matches no saved store
- **THEN** the store row shows the raw ID

#### Scenario: Active profile
- **WHEN** the user has run `idiot profile use staging-ci` and changed nothing since
- **THEN** a `profile` row shows `staging-ci` without annotation

#### Scenario: Changed profile
- **WHEN** after `idiot profile use staging-ci` the user switches store
- **THEN** the `profile` row shows `staging-ci` annotated `changed`

#### Scenario: Credential and store mismatch
- **WHEN** credential `ci` is active and paired with store `staging`, and `UBUNTU_STORE_ID` belongs to store `prod`
- **THEN** the rows are printed as usual, followed by a warning on stderr that `ci` belongs to `staging` while `prod` is active, and the exit status is zero

### Requirement: Usage level
Every dispatched command (excluding help, version, status and init) SHALL increment
an invocation counter in `IDIOT_CACHE_DIR`. The count SHALL select the logo
shown in help and version, and `idiot status` SHALL show an XP bar with the
progress from the current level to the next (or `max level`).

#### Scenario: Status does not count
- **WHEN** the user runs `idiot status` twice
- **THEN** the invocation count is unchanged

#### Scenario: Level up
- **WHEN** the count reaches 10
- **THEN** the logo changes from 💀 to 🥚
