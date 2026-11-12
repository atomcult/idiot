# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Credential storage
Credentials SHALL be stored one per file, named after the credential, in
`IDIOT_AUTH_DIR` (default `$XDG_DATA_HOME/idiot/auth`, falling back to
`~/.local/share/idiot/auth`). `idiot auth list` SHALL print the saved
credential names on stderr, each followed by its expiry (`expires in <N>d`,
`expires in <N>h`, `expires in <1h`, `expired`, or `expiry unknown`) read
from the expiry record without contacting the network, and print nothing
when there are none.

#### Scenario: Listing credentials
- **WHEN** credentials `work` and `personal` are saved and the user runs `idiot auth list`
- **THEN** both names are printed on stderr, one per line, each with its expiry

#### Scenario: Expired credential listed
- **WHEN** the saved credential `old` expired last week
- **THEN** `idiot auth list` shows `old` marked `expired`

#### Scenario: Custom directory
- **WHEN** `IDIOT_AUTH_DIR` is set
- **THEN** credentials are read from and written to that directory

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

### Requirement: Removing a credential
`idiot auth remove [<name>]` SHALL delete the saved credential, its cached
status and its expiry record, offering an fzf picker when no name is
given.

#### Scenario: Remove
- **WHEN** the user runs `idiot auth remove work`
- **THEN** the credential file, its cached status and its expiry record are deleted
