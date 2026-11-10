# Spec Delta

## MODIFIED Requirements

### Requirement: Activating a store
`idiot store use [<name>]` SHALL emit an export of `UBUNTU_STORE_ID` set to
the saved ID and an export of `IDIOT_ACTIVE_STORE` set to the store's
name, offering an fzf picker when no name is given (cancelling changes
nothing). `idiot store unset` SHALL emit unsets of `UBUNTU_STORE_ID` and
`IDIOT_ACTIVE_STORE`. `idiot store status` SHALL print the active ID, or
`global` when none is set.

#### Scenario: Use
- **WHEN** the user runs `idiot store use staging`
- **THEN** `UBUNTU_STORE_ID` is `abc123` and `IDIOT_ACTIVE_STORE` is `staging` in the current shell

#### Scenario: Unknown store
- **WHEN** the user runs `idiot store use nope`
- **THEN** the command fails with `store 'nope' not found`

#### Scenario: Back to the global store
- **WHEN** the user runs `idiot store unset`
- **THEN** `UBUNTU_STORE_ID` and `IDIOT_ACTIVE_STORE` are unset and `idiot store status` prints `global`
