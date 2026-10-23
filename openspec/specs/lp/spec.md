# lp Specification

## Purpose

Recording the user's Launchpad username, used as the login for Ubuntu Core
devices and VMs (`idiot vm ssh`/`scp`) and for Launchpad-related commands.

## Requirements

### Requirement: Setting the username
`idiot lp set [<username>]` SHALL save the username to `IDIOT_LP_USER_FILE`
(default `$XDG_DATA_HOME/idiot/lp-user`) and emit an export of
`IDIOT_LP_USER`. Without an argument it SHALL prompt for the username on the
terminal; an empty answer SHALL be rejected. `idiot lp unset` SHALL delete the
saved file and emit an unset of `IDIOT_LP_USER`.

#### Scenario: Set
- **WHEN** the user runs `idiot lp set alice`
- **THEN** `IDIOT_LP_USER` is `alice` and `alice` is saved for later shells

#### Scenario: Empty prompt
- **WHEN** the user runs `idiot lp set` and enters nothing
- **THEN** the command fails with `username is required`

#### Scenario: Unset
- **WHEN** the user runs `idiot lp unset`
- **THEN** the saved file is removed and `IDIOT_LP_USER` is unset

### Requirement: Resolving the username
The active username SHALL be `IDIOT_LP_USER` when set, otherwise the saved
file's contents. `idiot lp status` SHALL print it, or a hint to run
`idiot lp set` when there is none. Commands that need it SHALL fail with that
hint when it is missing.

#### Scenario: New shell
- **WHEN** a username was saved earlier and `IDIOT_LP_USER` is unset
- **THEN** `idiot lp status` prints the saved username

#### Scenario: Missing username
- **WHEN** no username is set or saved and the user runs `idiot vm ssh`
- **THEN** the command fails telling the user to run `idiot lp set <username>`
