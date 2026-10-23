# Spec Delta

## MODIFIED Requirements

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

### Requirement: Account status
`idiot auth status [--refresh]` SHALL show, on stderr, the account details of
the active credential (email, username, id, permissions, channels and
expiry) fetched from the Snap Store by idiot itself, with permission codes
translated to readable descriptions. When the credential was activated by name
the result SHALL be cached per credential under `IDIOT_CACHE_DIR` and reused
until `--refresh` is given. With no active credential it SHALL report
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

#### Scenario: Expired credential
- **WHEN** the active credential has expired
- **THEN** the command reports that the store rejected it, exits non-zero, and writes no cache entry

## ADDED Requirements

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
