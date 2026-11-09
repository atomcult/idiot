# Spec Delta

## MODIFIED Requirements

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
