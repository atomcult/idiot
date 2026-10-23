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

#### Scenario: Fresh shell
- **WHEN** nothing has been activated
- **THEN** the rows read `not logged in`, `global`, the host arch marked `host default`, and `not set` (unless a username was saved)

#### Scenario: Named store
- **WHEN** `UBUNTU_STORE_ID` matches the saved store `staging`
- **THEN** the store row shows `staging` and its ID

#### Scenario: Unknown store ID
- **WHEN** `UBUNTU_STORE_ID` matches no saved store
- **THEN** the store row shows the raw ID

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
