## MODIFIED Requirements

### Requirement: Shell integration and install checks
`idiot doctor` SHALL report:
- `✗` when `IDIOT_SHELL` is unset (no integration) or `IDIOT_EVAL_FILE` is
  not a writable file (outdated integration), with the command to load
  it;
- under the snap, `✗` when `IDIOT_SNAP` is unset or not an existing
  directory, and `!` when `IDIOT_DATA_HOME` is not the snap's common
  directory;
- the kind of install (snap with its instance, revision and version, or
  source with its location and version), and `!` when both a source
  install and the snap are present, naming the one this shell uses;
- `✗` when the `idiot-store` helper is missing or does not run (`auth add`
  and `auth status` need it);
- `✗` when the `idiot-helper` binary is missing or does not run (`inspect
  image`, `inspect diff`, `assertion show`, `vm vars` and `vm` need
  it).
When the integration is missing or outdated the command SHALL still run,
directly (confined under the snap), report those checks, and mark the
checks that need the host as skipped with `!`.

#### Scenario: No integration
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot doctor`
- **THEN** the integration line is `✗` suggesting `eval "$(idiot init bash)"` or `idiot init fish | source`, and the exit status is 1

#### Scenario: Outdated integration under the snap
- **WHEN** a shell loaded an integration without `IDIOT_EVAL_FILE` and the user runs `idiot doctor` from the snap
- **THEN** the integration line is `✗`, the host checks are reported as skipped, and the exit status is 1

#### Scenario: Two installs
- **WHEN** both `~/.local/bin/idiot` and the snap are installed and the shell loaded the snap's integration
- **THEN** a `!` line says both are installed and that this shell uses the snap

#### Scenario: idiot-helper missing
- **WHEN** a source install's `lib/idiot/libexec/idiot-helper` was removed and the user runs `idiot doctor`
- **THEN** an install line is `✗` naming `idiot-helper` and the commands that need it, with `make install` to rebuild it, and the exit status is 1

### Requirement: Credential and store checks
`idiot doctor` SHALL report `✗` when the credentials directory is
accessible by anyone but its owner or any credential file is readable or
writable by anyone but its owner, naming the path and the `chmod` that
fixes it; `✗` when `IDIOT_ACTIVE_CRED` names a credential that is no
longer saved; `!` when the active credential's contents differ from the
saved file of that name; `!` for each saved credential with a padded
macaroon, saying snapd's image tooling can't use it and that
`idiot auth use <name>` repairs it; `!` for a saved store file that is
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
- **THEN** a `✗` line says the active credential no longer exists and suggests `idiot auth use` or `idiot auth unset`

#### Scenario: Padded credential
- **WHEN** the saved credential `work` has a padded macaroon
- **THEN** a `!` line names `work`, says snapd can't use it, suggests `idiot auth use work`, and the file is unchanged

#### Scenario: Shell still holds the padded value
- **WHEN** `work` was active in a shell and was repaired by `idiot auth use work` in another shell
- **THEN** `idiot doctor` in the first shell reports that the active credential differs from the saved one and suggests `idiot auth use work`

#### Scenario: Expired active credential
- **WHEN** expiry records exist and the active credential has expired
- **THEN** its line is `✗` suggesting `idiot auth add`
