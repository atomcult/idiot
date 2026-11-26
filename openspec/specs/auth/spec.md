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
credential names on stderr, each followed by its expiry (`expires in <N>d`,
`expires in <N>h`, `expires in <1h`, `expired`, or `expiry unknown`) read
from the expiry record without contacting the network, then the store it
belongs to: the saved store's name when one has its ID, else the store ID,
`global` for the global store, or `any store` for a credential marked for
any store or with no recorded pairing. It SHALL print nothing when there
are none.

#### Scenario: Listing credentials
- **WHEN** credentials `work` and `personal` are saved and the user runs `idiot auth list`
- **THEN** both names are printed on stderr, one per line, each with its expiry and its store

#### Scenario: Expired credential listed
- **WHEN** the saved credential `old` expired last week
- **THEN** `idiot auth list` shows `old` marked `expired`

#### Scenario: Stores listed
- **WHEN** credential `ci` is paired with store ID `abc123`, saved as store `staging`; `work` with the global store; `personal` is marked for any store; and `old` has no pairing
- **THEN** `idiot auth list` shows `staging` for `ci`, `global` for `work`, and `any store` for `personal` and `old`

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
`IDIOT_ACTIVE_CRED` to its name, and SHALL emit an export of
`IDIOT_CRED_EXPIRES` (the expiry as Unix time) when the credential's
expiry is known, or an unset of it when not. When the credential has
expired or expires within the warning window it SHALL say so on stderr
(and still activate it). Without a name it SHALL offer an fzf picker whose
preview shows each credential's account status; under the snap, where the
picker runs confined, the preview SHALL show only cached status (as
`auth status --cached`) and never contact the store. Cancelling the picker
SHALL change nothing. `idiot auth logout` SHALL emit unsets for all four
variables.

#### Scenario: Login by name
- **WHEN** the user runs `idiot auth login work` through the shell wrapper
- **THEN** `UBUNTU_STORE_AUTH`, `SNAPCRAFT_STORE_CREDENTIALS` and `IDIOT_ACTIVE_CRED` are set in the current shell, and `IDIOT_CRED_EXPIRES` holds its expiry

#### Scenario: Login with unknown expiry
- **WHEN** idiot is installed as a snap and the user logs in to a credential with no expiry record
- **THEN** the credential is activated and `IDIOT_CRED_EXPIRES` is unset

#### Scenario: Login to an expiring credential
- **WHEN** the credential `work` expires in 2 days and the user runs `idiot auth login work`
- **THEN** it is activated and stderr says it expires in 2d

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot auth login nope`
- **THEN** the command fails with `credentials 'nope' not found` and no variables change

#### Scenario: Picker cancelled
- **WHEN** the user runs `idiot auth login` and cancels the picker
- **THEN** the command exits zero and no variables change

#### Scenario: Logout
- **WHEN** the user runs `idiot auth logout`
- **THEN** `UBUNTU_STORE_AUTH`, `SNAPCRAFT_STORE_CREDENTIALS`, `IDIOT_ACTIVE_CRED` and `IDIOT_CRED_EXPIRES` are unset

#### Scenario: Picker preview under the snap
- **WHEN** idiot is installed as a snap and the user runs `idiot auth login` with a credential whose status was never fetched
- **THEN** its preview says the status hasn't been fetched yet, and nothing contacts the store

### Requirement: Account status
`idiot auth status [--refresh]` SHALL show, on stderr, the account details of
the active credential (email, username, id, permissions, channels and
expiry) fetched from the Snap Store by idiot itself, with permission codes
translated to readable descriptions and the expiry followed by how long is
left (`in 41d`) or `expired`. When the credential was activated by name
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

#### Scenario: Time left
- **WHEN** the active credential expires in 41 days
- **THEN** the expiry line shows the timestamp followed by `in 41d`

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
produces, usable as `SNAPCRAFT_STORE_CREDENTIALS` by snapcraft and as
`UBUNTU_STORE_AUTH` by snapd's image tooling (`snap download`,
`snap prepare-image`, `ubuntu-image`), and credentials exported by
snapcraft SHALL work with every `idiot auth` command. `idiot auth add`
SHALL save the root and discharge macaroons as the store and Ubuntu One
returned them, and no credential idiot writes SHALL contain base64 padding
(`=`) in either macaroon.

#### Scenario: idiot credential used by snapcraft
- **WHEN** a credential made by `idiot auth add` is activated with `idiot auth login`
- **THEN** `snapcraft whoami` reports the same account

#### Scenario: idiot credential used by snapd
- **WHEN** a credential made by `idiot auth add` for an account with access to brand store `ce-team-test` is activated with that store active
- **THEN** `snap download ce-dummy` downloads the snap instead of failing with a 404

#### Scenario: No padding
- **WHEN** `idiot auth add` saves a credential
- **THEN** neither of its macaroons ends in `=`

#### Scenario: snapcraft credential used by idiot
- **WHEN** a file from `snapcraft export-login` is imported with `idiot auth import`
- **THEN** `idiot auth status` for it shows the account details

### Requirement: Removing a credential
`idiot auth remove [<name>]` SHALL delete the saved credential, its cached
status and its expiry record, offering an fzf picker when no name is
given.

#### Scenario: Remove
- **WHEN** the user runs `idiot auth remove work`
- **THEN** the credential file, its cached status and its expiry record are deleted

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
to: a store ID, the global store, or any store. `idiot auth add <name>`
SHALL record the store active when it saves the credential
(`UBUNTU_STORE_ID`, or global when unset), or any store with `--any`, and
name it in its confirmation. `idiot auth bind [<name>]` SHALL record the
active store for the named credential, or for the active credential when no
name is given, failing when there is neither, and print the pairing on
stderr; with `--any` it SHALL record any store instead. `idiot auth import`
SHALL record no pairing. A credential marked for any store, or with no
recorded pairing, SHALL be treated as belonging to every store. The record
SHALL hold no credential contents, and SHALL live with the credential under
`IDIOT_AUTH_DIR` without appearing as a credential in `idiot auth list` or
any credential picker.

#### Scenario: Pairing at creation
- **WHEN** `UBUNTU_STORE_ID` is `abc123` and the user runs `idiot auth add ci` and logs in
- **THEN** credential `ci` is paired with store `abc123`, and the confirmation names that store

#### Scenario: Global at creation
- **WHEN** `UBUNTU_STORE_ID` is unset and the user runs `idiot auth add work`
- **THEN** credential `work` is paired with the global store

#### Scenario: Any store at creation
- **WHEN** `UBUNTU_STORE_ID` is `abc123` and the user runs `idiot auth add --any personal` and logs in
- **THEN** credential `personal` is marked for any store, and the confirmation says so

#### Scenario: Re-pairing
- **WHEN** credential `ci` is active and paired with the global store, `UBUNTU_STORE_ID` is `abc123`, and the user runs `idiot auth bind`
- **THEN** `ci` is paired with `abc123`

#### Scenario: Marking for any store
- **WHEN** credential `personal` is paired with the global store and the user runs `idiot auth bind --any personal`
- **THEN** `personal` is marked for any store, and stderr says so

#### Scenario: Back to one store
- **WHEN** credential `personal` is marked for any store, `UBUNTU_STORE_ID` is `abc123`, and the user runs `idiot auth bind personal`
- **THEN** `personal` is paired with `abc123`

#### Scenario: Nothing to bind
- **WHEN** no credential is active and the user runs `idiot auth bind` or `idiot auth bind --any`
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
switch store, re-pair with `idiot auth bind`, or mark it for any store with
`idiot auth bind --any`. For commands that change the environment the check
SHALL use the settings as they are after the command. The warning SHALL NOT
change the exit status, what is exported or unset, or stdout, and SHALL
never contact the network. No warning SHALL be printed when the credential
is marked for any store, has no pairing, or no named credential is active.

#### Scenario: Login into the wrong store
- **WHEN** credential `ci` is paired with store `staging` (`abc123`), `UBUNTU_STORE_ID` is unset, and the user runs `idiot auth login ci`
- **THEN** `ci` is activated, the command exits zero, and stderr warns that `ci` belongs to `staging` while the global store is active, suggesting `idiot store use staging`, `idiot auth bind ci` and `idiot auth bind --any ci`

#### Scenario: Matching store
- **WHEN** credential `ci` is paired with `abc123` and the user runs `idiot auth login ci` with `UBUNTU_STORE_ID` `abc123`
- **THEN** no warning is printed

#### Scenario: Any-store credential in a brand store
- **WHEN** credential `personal` is marked for any store and the user runs `idiot store use dsmr6`, `idiot auth login personal` and `idiot status`
- **THEN** none of them prints a store mismatch warning

#### Scenario: Unpaired credential
- **WHEN** the active credential has no pairing
- **THEN** no command prints a store mismatch warning

### Requirement: Credential expiry record
idiot SHALL record when each saved credential expires, without the
network where possible: the earliest expiry among the credential's
macaroon caveats (root and discharge), or, when the caveats carry none,
the expiry the store reports for it. `idiot auth add` and
`idiot auth import` SHALL record it when they save a credential;
`idiot auth status` SHALL record it for the active named credential when
no record exists; `idiot auth login` SHALL record it when no record exists
and idiot is not installed as a snap. The record SHALL live in
`IDIOT_AUTH_DIR`, SHALL be readable and writable only by its owner, SHALL
hold no part of the credential, and SHALL NOT appear as a credential in
`auth list` or any picker. Reading the expiry SHALL NOT print, log or pass
on the command line any part of the credential. A credential whose expiry
cannot be read SHALL still be saved, with its expiry unknown.

#### Scenario: Recorded at creation
- **WHEN** the user runs `idiot auth add ci --expires 30d` and the login succeeds
- **THEN** `idiot auth list` shows `ci` expiring in 30 days (or 29 days and some hours) without contacting the store

#### Scenario: Recorded on import
- **WHEN** the user imports a `snapcraft export-login` file that expires on 2027-03-01
- **THEN** the credential's expiry is recorded as 2027-03-01 without contacting the store

#### Scenario: Earliest caveat wins
- **WHEN** a credential's root macaroon expires on 2027-11-09 and its discharge on 2027-05-01
- **THEN** its recorded expiry is 2027-05-01

#### Scenario: Unreadable expiry
- **WHEN** the user imports a file that is not a store credential
- **THEN** it is still saved, a message on stderr says its expiry is unknown, and `idiot auth list` shows `expiry unknown` for it

#### Scenario: Secret stays secret
- **WHEN** the expiry of any credential is read or recorded
- **THEN** no part of the credential appears on stdout, stderr, in the process list, or in the record file

#### Scenario: Private record
- **WHEN** an expiry record is written under a umask of 022
- **THEN** the record file has mode 0600

### Requirement: Expiry warning
When the active credential's expiry is known and it has expired, or
expires within `IDIOT_CRED_WARN_DAYS` days (default 7; `0` disables this
early warning but not the expired one; a value that is not a non-negative
integer means 7), the first command dispatched each day (excluding help,
version, status, prompt and init) SHALL print one warning on stderr naming
the credential and how long it has left or that it has expired, before the
command's own output. The warning SHALL be shown at most once per
credential per calendar day (UTC) across all the user's shells, SHALL NOT
change the command's exit status, output or eval file, and SHALL NOT
contact the network.

#### Scenario: Expiring soon
- **WHEN** the active credential `work` expires in 3 days and the user runs `idiot store list` for the first time today
- **THEN** stderr starts with a warning that `work` expires in 3d, and the store list follows

#### Scenario: Only once a day
- **WHEN** the user then runs `idiot store list` again, in the same or another shell
- **THEN** no expiry warning is printed

#### Scenario: Expired
- **WHEN** the active credential expired yesterday and the user runs any counted command
- **THEN** the first such command today warns that it has expired and suggests `idiot auth add`

#### Scenario: Window configured
- **WHEN** `IDIOT_CRED_WARN_DAYS` is `30` and the active credential expires in 20 days
- **THEN** the warning is shown; with the default it is not

#### Scenario: Early warning disabled
- **WHEN** `IDIOT_CRED_WARN_DAYS` is `0` and the active credential expires in 2 days
- **THEN** no warning is shown

#### Scenario: Expiry unknown
- **WHEN** the active credential has no expiry record
- **THEN** no warning is shown

### Requirement: Repairing padded credentials
When `idiot auth login`, `idiot profile use` or `idiot auth export` uses a
saved credential whose root or discharge macaroon has base64 padding, it
SHALL first rewrite the saved file without the padding, keeping the file
private, and print on stderr that the credential was repaired. It SHALL
then use the repaired credential: what `auth login` and `profile use`
export, and what `auth export` outputs. The repair SHALL change nothing
else in the credential, SHALL never contact the network, and SHALL never
print the credential or pass it as a command argument. A credential
without padding SHALL be left untouched, with nothing printed. When the
idiot-store helper is missing, the command SHALL use the credential as
saved and warn that it could not be checked.

#### Scenario: Repaired on login
- **WHEN** the saved credential `work` has a padded root macaroon and the user runs `idiot auth login work`
- **THEN** the saved file no longer has the padding, stderr says `work` was repaired, and `UBUNTU_STORE_AUTH` and `SNAPCRAFT_STORE_CREDENTIALS` hold the repaired credential

#### Scenario: Unpadded credential untouched
- **WHEN** the saved credential `ci` has no padding and the user runs `idiot auth login ci`
- **THEN** the file is unchanged and no repair message is printed

#### Scenario: Repaired on export
- **WHEN** the saved credential `work` is padded and the user runs `idiot auth export work -f out.cred`
- **THEN** both the saved file and `out.cred` hold the repaired credential

#### Scenario: Private after repair
- **WHEN** a credential is repaired
- **THEN** its file is still readable and writable only by its owner
