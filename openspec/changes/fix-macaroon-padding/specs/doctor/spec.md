## MODIFIED Requirements

### Requirement: Credential and store checks
`idiot doctor` SHALL report `✗` when the credentials directory is
accessible by anyone but its owner or any credential file is readable or
writable by anyone but its owner, naming the path and the `chmod` that
fixes it; `✗` when `IDIOT_ACTIVE_CRED` names a credential that is no
longer saved; `!` when the active credential's contents differ from the
saved file of that name; `!` for each saved credential with a padded
macaroon, saying snapd's image tooling can't use it and that
`idiot auth login <name>` repairs it; `!` for a saved store file that is
empty or holds more than one line; and `!` when `UBUNTU_STORE_ID` matches
no saved store. The padding check SHALL change nothing and SHALL be skipped
when the idiot-store helper is missing (its own check reports that).
When credential expiry records exist (see the auth capability), it SHALL
also report `✗` for an expired active credential, `!` for any other
expired credential or one expiring within the warning window, and `!` for
credentials whose expiry is unknown, suggesting `idiot auth status`.
Comparing and checking credentials SHALL NOT print them or pass them as
command arguments.

#### Scenario: Loose permissions
- **WHEN** `~/.local/share/idiot/auth/work` has mode 0644
- **THEN** a `✗` line names the file and `chmod 600`, without showing its contents

#### Scenario: Active credential removed
- **WHEN** `IDIOT_ACTIVE_CRED` is `old` and no credential `old` is saved
- **THEN** a `✗` line says the active credential no longer exists and suggests `idiot auth login` or `idiot auth logout`

#### Scenario: Padded credential
- **WHEN** the saved credential `work` has a padded macaroon
- **THEN** a `!` line names `work`, says snapd can't use it, suggests `idiot auth login work`, and the file is unchanged

#### Scenario: Shell still holds the padded value
- **WHEN** `work` was active in a shell and was repaired by `idiot auth login work` in another shell
- **THEN** `idiot doctor` in the first shell reports that the active credential differs from the saved one and suggests `idiot auth login work`

#### Scenario: Expired active credential
- **WHEN** expiry records exist and the active credential has expired
- **THEN** its line is `✗` suggesting `idiot auth add`
