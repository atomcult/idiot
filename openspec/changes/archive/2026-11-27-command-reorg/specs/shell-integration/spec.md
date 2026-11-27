## MODIFIED Requirements

### Requirement: Command dispatch
`idiot <command> [args]` SHALL run the matching executable under `cmd/`; a
command group (a directory) SHALL dispatch its first argument to the matching
subcommand, up to two levels deep. `help`, `-h` or `--help` (or no
subcommand) SHALL show the group's usage on stderr. A renamed command's old
name SHALL be resolved to its new one before dispatch (see the
command-layout capability). An unknown command or subcommand SHALL fail
with an error naming it.

#### Scenario: Group subcommand
- **WHEN** the user runs `idiot store use prod`
- **THEN** the `store use` subcommand runs with `prod` as its argument

#### Scenario: Group help
- **WHEN** the user runs `idiot auth` with no subcommand
- **THEN** the auth usage is printed on stderr and the exit status is zero

#### Scenario: Two levels deep
- **WHEN** the user runs `idiot assertion repos update canonical`
- **THEN** the `assertion repos update` subcommand runs with `canonical` as its argument

#### Scenario: Unknown command
- **WHEN** the user runs `idiot frobnicate`
- **THEN** the command exits non-zero with `unknown command: frobnicate`

### Requirement: Wrapper required for commands
Apart from help, version, status, prompt, `init` and `doctor` (which then
runs a limited report, see the doctor capability), a command SHALL refuse
to run when `IDIOT_SHELL` is unset, with an error telling the user how to install
the shell integration. Help SHALL never need the integration: a group's
help, and a command's help asked for with `help`, `-h` or `--help` as its
first argument or `-h`/`--help` anywhere before `--` (for `idiot run`,
only as its first argument), SHALL be shown without it, and under the snap
SHALL be shown without handing off to the host. A command that changes the
environment SHALL refuse to run when `IDIOT_SHELL` is set but
`IDIOT_EVAL_FILE` is not (an outdated wrapper), telling the user to reload
the integration, and SHALL change nothing.

#### Scenario: Not initialised
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store use prod`
- **THEN** the command exits non-zero and suggests `eval "$(idiot init bash)"` or `idiot init fish | source`

#### Scenario: Help without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store help`
- **THEN** the store usage is shown

#### Scenario: Command help without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot vm list --help` or `idiot store browse --help`
- **THEN** that command's usage is shown and the exit status is zero

#### Scenario: A tool's own help still needs the wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot run ls --help`
- **THEN** the command exits non-zero suggesting `idiot init`

#### Scenario: Prompt without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot prompt`
- **THEN** the segment is printed and the exit status is zero

#### Scenario: Doctor without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot doctor`
- **THEN** a report is shown that marks the shell integration as broken

#### Scenario: Outdated wrapper
- **WHEN** a shell loaded the previous integration and the user runs `idiot store use prod`
- **THEN** the command exits non-zero telling the user to re-run `idiot init` and `UBUNTU_STORE_ID` is unchanged
