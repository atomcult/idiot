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
`idiot auth add <name>` SHALL create a new store credential without
snapcraft. It SHALL prompt on the terminal for the Ubuntu One email and
password (the password not echoed), and for a one-time password only when
the account requires two-factor authentication. The credential SHALL carry:
- the permissions given by `--permissions <list>`, or picked interactively
  with `--permissions` and no value, defaulting to `package_access`,
  `package_manage`, `package_metrics`, `package_push`, `package_register`,
  `package_release` and `package_update`;
- the snaps given by `--snaps <list>` and channels given by
  `--channels <list>`, unrestricted by default;
- the expiry given by `--expires` as an ISO 8601 date/time or `<N>d` days
  from now, defaulting to one year from now.
It SHALL refuse a name that already exists, an expiry in the past, or an
unknown permission, before prompting. It SHALL save nothing if the login
fails, and SHALL never print the password or the credential.

#### Scenario: New credential
- **WHEN** the user runs `idiot auth add work` and enters valid credentials
- **THEN** a credential named `work` is saved with the default permissions, no snap or channel restriction and a one-year expiry, and a confirmation is printed on stderr

#### Scenario: Scoped credential
- **WHEN** the user runs `idiot auth add ci --permissions package_push,package_release --snaps my-gadget --channels edge --expires 30d`
- **THEN** `idiot auth status` for `ci` lists only those two permissions, the `edge` channel, and an expiry 30 days out

#### Scenario: Permission picker
- **WHEN** the user runs `idiot auth add ci --permissions` and selects two permissions in the picker
- **THEN** the credential carries exactly those two permissions

#### Scenario: Two-factor account
- **WHEN** the account has two-factor authentication enabled
- **THEN** the user is asked for a one-time password after the password, and the credential is saved once it is accepted

#### Scenario: Failed login
- **WHEN** the store rejects the email or password
- **THEN** the store's error message is shown, no credential file is created, and the exit status is non-zero

#### Scenario: Existing name
- **WHEN** a credential named `work` exists and the user runs `idiot auth add work`
- **THEN** the command fails telling the user to remove it first, without prompting

#### Scenario: Expiry in the past
- **WHEN** the user runs `idiot auth add ci --expires 2020-01-01`
- **THEN** the command fails before prompting and saves nothing

#### Scenario: snapcraft not installed
- **WHEN** snapcraft is not installed
- **THEN** `idiot auth add` and `idiot auth status` still work

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
the active credential (email, username, id, permissions, channels and
expiry) fetched from the Snap Store by idiot itself, with permission codes
translated to readable descriptions. When the credential was activated by name
the result SHALL be cached per credential under `IDIOT_CACHE_DIR` and reused
until `--refresh` is given. With `--cached` it SHALL show only a cached
result, and otherwise say the status hasn't been fetched yet, without
contacting the store. With no active credential it SHALL report
`Not logged in` and exit zero. A rejected or expired credential SHALL be
reported as such with a non-zero exit, and SHALL NOT be cached.

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

#### Scenario: Expired credential
- **WHEN** the active credential has expired
- **THEN** the command reports that the store rejected it, exits non-zero, and writes no cache entry

### Requirement: Credential format compatibility
Credentials created by idiot SHALL be in the format `snapcraft export-login`
produces, usable as `SNAPCRAFT_STORE_CREDENTIALS` by snapcraft, and
credentials exported by snapcraft SHALL work with every `idiot auth`
command.

#### Scenario: idiot credential used by snapcraft
- **WHEN** a credential made by `idiot auth add` is activated with `idiot auth login`
- **THEN** `snapcraft whoami` reports the same account

#### Scenario: snapcraft credential used by idiot
- **WHEN** a file from `snapcraft export-login` is imported with `idiot auth import`
- **THEN** `idiot auth status` for it shows the account details

### Requirement: Removing a credential
`idiot auth remove [<name>]` SHALL delete the saved credential, its cached
status and its store pairing, offering an fzf picker when no name is given.

#### Scenario: Remove
- **WHEN** the user runs `idiot auth remove work`
- **THEN** the credential file, its cached status and its store pairing are deleted

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

### Requirement: Credential store pairing
idiot SHALL record, per saved credential, the store the credential belongs
to: a store ID, or the global store. `idiot auth add <name>` SHALL record
the store active when it saves the credential (`UBUNTU_STORE_ID`, or global
when unset) and name it in its confirmation. `idiot auth bind [<name>]`
SHALL record the active store for the named credential, or for the active
credential when no name is given, failing when there is neither, and print
the pairing on stderr. `idiot auth import` SHALL record no pairing. A
credential with no recorded pairing SHALL be treated as belonging to any
store. The record SHALL hold no credential contents, and SHALL live with
the credential under `IDIOT_AUTH_DIR` without appearing in `idiot auth
list` or any credential picker.

#### Scenario: Pairing at creation
- **WHEN** `UBUNTU_STORE_ID` is `abc123` and the user runs `idiot auth add ci` and logs in
- **THEN** credential `ci` is paired with store `abc123`, and the confirmation names that store

#### Scenario: Global at creation
- **WHEN** `UBUNTU_STORE_ID` is unset and the user runs `idiot auth add work`
- **THEN** credential `work` is paired with the global store

#### Scenario: Re-pairing
- **WHEN** credential `ci` is active and paired with the global store, `UBUNTU_STORE_ID` is `abc123`, and the user runs `idiot auth bind`
- **THEN** `ci` is paired with `abc123`

#### Scenario: Nothing to bind
- **WHEN** no credential is active and the user runs `idiot auth bind`
- **THEN** the command fails with its usage and records nothing

#### Scenario: Imported credential
- **WHEN** the user imports a credential with `idiot auth import`
- **THEN** it has no pairing and no command warns about its store

#### Scenario: Listing unaffected
- **WHEN** credential `ci` has a pairing and the user runs `idiot auth list`
- **THEN** only `ci` is listed

### Requirement: Store mismatch warning
When the active credential (`IDIOT_ACTIVE_CRED`) is paired with a store
other than the active one (`UBUNTU_STORE_ID`, or global when unset),
`idiot status`, `idiot auth login`, `idiot store use`, `idiot store unset`,
`idiot profile use` and `idiot profile save` SHALL print a warning on
stderr naming the credential, the store it is paired with and the active
store (by saved store name where one matches, else by ID), and how to
switch store or re-pair with `idiot auth bind`. For commands that change
the environment the check SHALL use the settings as they are after the
command. The warning SHALL NOT change the exit status, what is exported or
unset, or stdout, and SHALL never contact the network. No warning SHALL be
printed when the credential has no pairing or no named credential is
active.

#### Scenario: Login into the wrong store
- **WHEN** credential `ci` is paired with store `staging` (`abc123`), `UBUNTU_STORE_ID` is unset, and the user runs `idiot auth login ci`
- **THEN** `ci` is activated, the command exits zero, and stderr warns that `ci` belongs to `staging` while the global store is active

#### Scenario: Matching store
- **WHEN** credential `ci` is paired with `abc123` and the user runs `idiot auth login ci` with `UBUNTU_STORE_ID` `abc123`
- **THEN** no warning is printed

#### Scenario: Unpaired credential
- **WHEN** the active credential has no pairing
- **THEN** no command prints a store mismatch warning
