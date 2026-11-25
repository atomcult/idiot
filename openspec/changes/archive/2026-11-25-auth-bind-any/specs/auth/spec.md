## MODIFIED Requirements

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
