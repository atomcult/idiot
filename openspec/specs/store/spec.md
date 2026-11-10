# store Specification

## Purpose

Saving Dedicated Snap Store IDs under readable names and switching the
current shell between them via `UBUNTU_STORE_ID`.

## Requirements

### Requirement: Saved stores
Stores SHALL be saved one per file in `IDIOT_STORES_DIR` (default
`$XDG_DATA_HOME/idiot/stores`), the file holding the store ID.
`idiot store add <name> <store-id>` SHALL accept only names made of letters,
digits, `-` and `_`, and refuse an existing name. `idiot store remove <name>`
SHALL delete a saved store. `idiot store list` SHALL print names and IDs on
stderr, or a hint when none are saved.

#### Scenario: Add
- **WHEN** the user runs `idiot store add staging abc123`
- **THEN** a store `staging` with ID `abc123` is saved

#### Scenario: Invalid name
- **WHEN** the user runs `idiot store add ../x abc123`
- **THEN** the command fails and nothing is written

#### Scenario: Duplicate
- **WHEN** `staging` exists and the user runs `idiot store add staging def456`
- **THEN** the command fails and the saved ID is unchanged

#### Scenario: Empty list
- **WHEN** no stores are saved and the user runs `idiot store list`
- **THEN** a hint to use `idiot store add` is printed

### Requirement: Activating a store
`idiot store use [<name>]` SHALL emit an export of `UBUNTU_STORE_ID` set to
the saved ID, offering an fzf picker when no name is given (cancelling
changes nothing). `idiot store unset` SHALL emit an unset of
`UBUNTU_STORE_ID`. `idiot store status` SHALL print the active ID, or
`global` when none is set.

#### Scenario: Use
- **WHEN** the user runs `idiot store use staging`
- **THEN** `UBUNTU_STORE_ID` is `abc123` in the current shell

#### Scenario: Unknown store
- **WHEN** the user runs `idiot store use nope`
- **THEN** the command fails with `store 'nope' not found`

#### Scenario: Back to the global store
- **WHEN** the user runs `idiot store unset`
- **THEN** `UBUNTU_STORE_ID` is unset and `idiot store status` prints `global`

### Requirement: Warning on a store switch
`idiot store use` and `idiot store unset` SHALL, after queueing their
change, print the auth capability's store mismatch warning when the active
credential is paired with a store other than the one now active. The
switch itself SHALL happen regardless, and the exit status SHALL be zero.

#### Scenario: Switching away from the credential's store
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store use prod`
- **THEN** `UBUNTU_STORE_ID` is `prod`'s ID, the command exits zero, and stderr warns that `ci` belongs to `staging` while `prod` is active

#### Scenario: Switching to the credential's store
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store use staging`
- **THEN** no warning is printed

#### Scenario: Back to global
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store unset`
- **THEN** `UBUNTU_STORE_ID` is unset and stderr warns that `ci` belongs to `staging` while the global store is active
