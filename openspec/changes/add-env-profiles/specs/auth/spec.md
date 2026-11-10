# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Removing a credential
`idiot auth remove [<name>]` SHALL delete the saved credential, its cached
status and its store pairing, offering an fzf picker when no name is given.

#### Scenario: Remove
- **WHEN** the user runs `idiot auth remove work`
- **THEN** the credential file, its cached status and its store pairing are deleted
