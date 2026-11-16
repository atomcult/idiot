# Spec Delta

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
  image`, `inspect diff`, `inspect assertion`, `vm vars` and `vm` need
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
