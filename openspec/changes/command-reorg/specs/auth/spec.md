## MODIFIED Requirements

### Requirement: Activating a credential
`idiot auth use [<name>]` SHALL emit exports setting `UBUNTU_STORE_AUTH`
and `SNAPCRAFT_STORE_CREDENTIALS` to the credential's contents and
`IDIOT_ACTIVE_CRED` to its name, and SHALL emit an export of
`IDIOT_CRED_EXPIRES` (the expiry as Unix time) when the credential's
expiry is known, or an unset of it when not. When the credential has
expired or expires within the warning window it SHALL say so on stderr
(and still activate it). Without a name it SHALL offer an fzf picker whose
preview shows each credential's account status; under the snap, where the
picker runs confined, the preview SHALL show only cached status (as
`auth status --cached`) and never contact the store. Cancelling the picker
SHALL change nothing. `idiot auth unset` SHALL emit unsets for all four
variables.

#### Scenario: Login by name
- **WHEN** the user runs `idiot auth use work` through the shell wrapper
- **THEN** `UBUNTU_STORE_AUTH`, `SNAPCRAFT_STORE_CREDENTIALS` and `IDIOT_ACTIVE_CRED` are set in the current shell, and `IDIOT_CRED_EXPIRES` holds its expiry

#### Scenario: Login with unknown expiry
- **WHEN** idiot is installed as a snap and the user activates a credential with no expiry record
- **THEN** the credential is activated and `IDIOT_CRED_EXPIRES` is unset

#### Scenario: Login to an expiring credential
- **WHEN** the credential `work` expires in 2 days and the user runs `idiot auth use work`
- **THEN** it is activated and stderr says it expires in 2d

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot auth use nope`
- **THEN** the command fails with `credentials 'nope' not found` and no variables change

#### Scenario: Picker cancelled
- **WHEN** the user runs `idiot auth use` and cancels the picker
- **THEN** the command exits zero and no variables change

#### Scenario: Logout
- **WHEN** the user runs `idiot auth unset`
- **THEN** `UBUNTU_STORE_AUTH`, `SNAPCRAFT_STORE_CREDENTIALS`, `IDIOT_ACTIVE_CRED` and `IDIOT_CRED_EXPIRES` are unset

#### Scenario: Picker preview under the snap
- **WHEN** idiot is installed as a snap and the user runs `idiot auth use` with a credential whose status was never fetched
- **THEN** its preview says the status hasn't been fetched yet, and nothing contacts the store

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
- **WHEN** a credential made by `idiot auth add` is activated with `idiot auth use`
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

### Requirement: Store mismatch warning
When the active credential (`IDIOT_ACTIVE_CRED`) is paired with a store
other than the active one (`UBUNTU_STORE_ID`, or global when unset),
`idiot status`, `idiot auth use`, `idiot store use`, `idiot store unset`,
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
- **WHEN** credential `ci` is paired with store `staging` (`abc123`), `UBUNTU_STORE_ID` is unset, and the user runs `idiot auth use ci`
- **THEN** `ci` is activated, the command exits zero, and stderr warns that `ci` belongs to `staging` while the global store is active, suggesting `idiot store use staging`, `idiot auth bind ci` and `idiot auth bind --any ci`

#### Scenario: Matching store
- **WHEN** credential `ci` is paired with `abc123` and the user runs `idiot auth use ci` with `UBUNTU_STORE_ID` `abc123`
- **THEN** no warning is printed

#### Scenario: Any-store credential in a brand store
- **WHEN** credential `personal` is marked for any store and the user runs `idiot store use dsmr6`, `idiot auth use personal` and `idiot status`
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
no record exists; `idiot auth use` SHALL record it when no record exists
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

### Requirement: Repairing padded credentials
When `idiot auth use`, `idiot profile use` or `idiot auth export` uses a
saved credential whose root or discharge macaroon has base64 padding, it
SHALL first rewrite the saved file without the padding, keeping the file
private, and print on stderr that the credential was repaired. It SHALL
then use the repaired credential: what `auth use` and `profile use`
export, and what `auth export` outputs. The repair SHALL change nothing
else in the credential, SHALL never contact the network, and SHALL never
print the credential or pass it as a command argument. A credential
without padding SHALL be left untouched, with nothing printed. When the
idiot-store helper is missing, the command SHALL use the credential as
saved and warn that it could not be checked.

#### Scenario: Repaired on login
- **WHEN** the saved credential `work` has a padded root macaroon and the user runs `idiot auth use work`
- **THEN** the saved file no longer has the padding, stderr says `work` was repaired, and `UBUNTU_STORE_AUTH` and `SNAPCRAFT_STORE_CREDENTIALS` hold the repaired credential

#### Scenario: Unpadded credential untouched
- **WHEN** the saved credential `ci` has no padding and the user runs `idiot auth use ci`
- **THEN** the file is unchanged and no repair message is printed

#### Scenario: Repaired on export
- **WHEN** the saved credential `work` is padded and the user runs `idiot auth export work -f out.cred`
- **THEN** both the saved file and `out.cred` hold the repaired credential

#### Scenario: Private after repair
- **WHEN** a credential is repaired
- **THEN** its file is still readable and writable only by its owner
