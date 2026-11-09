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
under `<name>`, and SHALL fail with its usage when either argument is
missing, and refuse an existing name or a missing file.
`idiot auth export [<name>] [-f <file>]` SHALL output a saved credential, to
`<file>` when given and otherwise to stdout; without a name it SHALL offer an
fzf picker.

#### Scenario: Import
- **WHEN** the user runs `idiot auth import ci ./ci.cred`
- **THEN** a credential named `ci` with the file's contents is saved

#### Scenario: Import missing file
- **WHEN** the user runs `idiot auth import ci ./missing`
- **THEN** the command fails with `file not found`

#### Scenario: Import missing argument
- **WHEN** the user runs `idiot auth import ci`
- **THEN** the command fails with `usage: idiot auth import <name> <file>` and saves nothing

#### Scenario: Export to file
- **WHEN** the user runs `idiot auth export ci -f out.cred`
- **THEN** `out.cred` holds the credential and a confirmation is printed on stderr

### Requirement: Activating a credential
`idiot auth login [<name>]` SHALL emit exports setting `UBUNTU_STORE_AUTH`
and `SNAPCRAFT_STORE_CREDENTIALS` to the credential's contents and
`IDIOT_ACTIVE_CRED` to its name. Without a name it SHALL offer an fzf picker
whose preview shows each credential's account status; under the snap, where
the picker runs confined, the preview SHALL show only cached status (as
`auth status --cached`) and never contact the store. Cancelling the picker
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

#### Scenario: Picker preview under the snap
- **WHEN** idiot is installed as a snap and the user runs `idiot auth login` with a credential whose status was never fetched
- **THEN** its preview says the status hasn't been fetched yet, and nothing contacts the store

### Requirement: Account status
`idiot auth status [--refresh]` SHALL show, on stderr, the account details of
the active credential as reported by `snapcraft whoami`, with permission codes
translated to readable descriptions. When the credential was activated by name
the result SHALL be cached per credential under `IDIOT_CACHE_DIR` and reused
until `--refresh` is given. With `--cached` it SHALL show only a cached
result, and otherwise say the status hasn't been fetched yet, without
contacting the store. With no active credential it SHALL report
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

#### Scenario: Cached only
- **WHEN** no status is cached for the active credential `work` and the user runs `idiot auth status --cached`
- **THEN** it says the status hasn't been fetched yet, exits zero, and contacts nothing

### Requirement: Removing a credential
`idiot auth remove [<name>]` SHALL delete the saved credential and its cached
status, offering an fzf picker when no name is given.

#### Scenario: Remove
- **WHEN** the user runs `idiot auth remove work`
- **THEN** the credential file and its cached status are deleted

### Requirement: Credential names
Commands that create a credential (`auth add`, `auth import`) SHALL accept
only names made of letters, digits, `.`, `-` and `_` that do not start with
`.`, and SHALL reject any other name without writing anything.

#### Scenario: Path in name
- **WHEN** the user runs `idiot auth import ../../.bashrc ./ci.cred`
- **THEN** the command fails with an invalid-name error and nothing is written

### Requirement: Private credential files
The credentials directory SHALL be accessible only by its owner (0700) and
every credential file idiot writes, including the target of
`auth export -f`, SHALL be readable and writable only by its owner (0600),
regardless of the source file's mode or the user's umask.

#### Scenario: Import from a world-readable file
- **WHEN** the user imports a credential from a file with mode 0664
- **THEN** the saved credential has mode 0600

#### Scenario: Export to a file
- **WHEN** the user runs `idiot auth export ci -f out.cred`
- **THEN** `out.cred` has mode 0600
