# Spec Delta

## MODIFIED Requirements

### Requirement: Wrapper required for commands
Apart from help, version, status, prompt and `init`, a command SHALL refuse
to run when `IDIOT_SHELL` is unset, with an error telling the user how to
install the shell integration. A command that changes the environment SHALL
refuse to run when `IDIOT_SHELL` is set but `IDIOT_EVAL_FILE` is not (an
outdated wrapper), telling the user to reload the integration, and SHALL
change nothing.

#### Scenario: Not initialised
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store use prod`
- **THEN** the command exits non-zero and suggests `eval "$(idiot init bash)"` or `idiot init fish | source`

#### Scenario: Help without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store help`
- **THEN** the store usage is shown

#### Scenario: Prompt without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot prompt`
- **THEN** the segment is printed and the exit status is zero

#### Scenario: Outdated wrapper
- **WHEN** a shell loaded the previous integration and the user runs `idiot store use prod`
- **THEN** the command exits non-zero telling the user to re-run `idiot init` and `UBUNTU_STORE_ID` is unchanged

### Requirement: Host environment from init
Under the snap, the integration printed by `idiot init` SHALL also export
`IDIOT_SNAP` (`/snap/<instance>/current`), `IDIOT_DATA_HOME` and
`IDIOT_CACHE_DIR` (in the snap's per-user common directory), so host
commands find the snap's files and the same state as confined ones. It
SHALL NOT export `SNAP`. A host command run without `IDIOT_SNAP` SHALL fail
telling the user to re-run `idiot init`. Under either install, the
integration SHALL also maintain `IDIOT_PROMPT` (see the prompt
capability), which is exported only while the segment is non-empty.

#### Scenario: Snap integration
- **WHEN** the user loads the integration from the snap
- **THEN** `IDIOT_SNAP` is `/snap/idiot/current`, `IDIOT_DATA_HOME` is `~/snap/idiot/common`, and `SNAP` is unset

#### Scenario: Source integration
- **WHEN** the user loads the integration from a source install in a shell with no store, arch or credential active
- **THEN** only `IDIOT_SHELL` is exported
