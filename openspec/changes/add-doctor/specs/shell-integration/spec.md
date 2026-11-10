# Spec Delta

## MODIFIED Requirements

### Requirement: Wrapper required for commands
Apart from help, version, status, `init` and `doctor` (which then runs a
limited report, see the doctor capability), a command SHALL refuse to run
when `IDIOT_SHELL` is unset, with an error telling the user how to install
the shell integration. A command that changes the environment SHALL refuse
to run when `IDIOT_SHELL` is set but `IDIOT_EVAL_FILE` is not (an outdated
wrapper), telling the user to reload the integration, and SHALL change
nothing.

#### Scenario: Not initialised
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store use prod`
- **THEN** the command exits non-zero and suggests `eval "$(idiot init bash)"` or `idiot init fish | source`

#### Scenario: Help without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store help`
- **THEN** the store usage is shown

#### Scenario: Doctor without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot doctor`
- **THEN** a report is shown that marks the shell integration as broken

#### Scenario: Outdated wrapper
- **WHEN** a shell loaded the previous integration and the user runs `idiot store use prod`
- **THEN** the command exits non-zero telling the user to re-run `idiot init` and `UBUNTU_STORE_ID` is unchanged
