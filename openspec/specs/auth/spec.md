# auth Specification

## Purpose

Saving named Snap Store credentials and switching the current shell between
them, so store tooling (snapcraft, snap store APIs) acts as the chosen
account.

## Requirements

### Requirement: Credential storage
Credentials SHALL be stored one per file, named after the credential, in
`IDIOT_AUTH_DIR` (default `$XDG_DATA_HOME/idiot/auth`, falling back to
`~/.local/share/idiot/auth`). `idiot auth list` SHALL print the saved
credential names on stderr, and print nothing when there are none.

#### Scenario: Listing credentials
- **WHEN** credentials `work` and `personal` are saved and the user runs `idiot auth list`
- **THEN** both names are printed on stderr, one per line

#### Scenario: Custom directory
- **WHEN** `IDIOT_AUTH_DIR` is set
- **THEN** credentials are read from and written to that directory

### Requirement: Creating a credential
`idiot auth add <name>` SHALL create a new store credential by running
`snapcraft export-login` interactively and save the result as `<name>`. It
SHALL refuse a name that already exists and SHALL save nothing if the login
fails.

#### Scenario: New credential
- **WHEN** the user runs `idiot auth add work` and completes the snapcraft login
- **THEN** a credential named `work` is saved and a confirmation is printed on stderr

#### Scenario: Existing name
- **WHEN** a credential named `work` exists and the user runs `idiot auth add work`
- **THEN** the command fails telling the user to remove it first

#### Scenario: Failed login
- **WHEN** `snapcraft export-login` fails
- **THEN** no credential file is created and the command exits non-zero

### Requirement: Importing and exporting
`idiot auth import <name> <file>` SHALL copy an existing credential file in
under `<name>`, refusing an existing name or a missing file.
`idiot auth export [<name>] [-f <file>]` SHALL output a saved credential, to
`<file>` when given and otherwise to stdout; without a name it SHALL offer an
fzf picker.

#### Scenario: Import
- **WHEN** the user runs `idiot auth import ci ./ci.cred`
- **THEN** a credential named `ci` with the file's contents is saved

#### Scenario: Import missing file
- **WHEN** the user runs `idiot auth import ci ./missing`
- **THEN** the command fails with `file not found`

#### Scenario: Export to file
- **WHEN** the user runs `idiot auth export ci -f out.cred`
- **THEN** `out.cred` holds the credential and a confirmation is printed on stderr

### Requirement: Activating a credential
`idiot auth login [<name>]` SHALL emit exports setting `UBUNTU_STORE_AUTH`
and `SNAPCRAFT_STORE_CREDENTIALS` to the credential's contents and
`IDIOT_ACTIVE_CRED` to its name. Without a name it SHALL offer an fzf picker
whose preview shows each credential's account status. Cancelling the picker
SHALL change nothing. `idiot auth logout` SHALL emit unsets for all three
variables.

#### Scenario: Login by name
- **WHEN** the user runs `idiot auth login work` through the shell wrapper
- **THEN** the three variables are set in the current shell

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot auth login nope`
- **THEN** the command fails with `credentials 'nope' not found` and no variables change

#### Scenario: Picker cancelled
- **WHEN** the user runs `idiot auth login` and cancels the picker
- **THEN** the command exits zero and no variables change

#### Scenario: Logout
- **WHEN** the user runs `idiot auth logout`
- **THEN** `UBUNTU_STORE_AUTH`, `SNAPCRAFT_STORE_CREDENTIALS` and `IDIOT_ACTIVE_CRED` are unset

### Requirement: Account status
`idiot auth status [--refresh]` SHALL show, on stderr, the account details of
the active credential as reported by `snapcraft whoami`, with permission codes
translated to readable descriptions. When the credential was activated by name
the result SHALL be cached per credential under `IDIOT_CACHE_DIR` and reused
until `--refresh` is given. With no active credential it SHALL report
`Not logged in` and exit zero.

#### Scenario: Cached status
- **WHEN** the status for credential `work` was fetched before and the user runs `idiot auth status`
- **THEN** the cached details are shown without contacting the store

#### Scenario: Refresh
- **WHEN** the user runs `idiot auth status --refresh`
- **THEN** the cache is discarded and fresh details are fetched

#### Scenario: Not logged in
- **WHEN** no credential is active
- **THEN** `Not logged in` is printed and the exit status is zero

### Requirement: Removing a credential
`idiot auth remove [<name>]` SHALL delete the saved credential and its cached
status, offering an fzf picker when no name is given.

#### Scenario: Remove
- **WHEN** the user runs `idiot auth remove work`
- **THEN** the credential file and its cached status are deleted
