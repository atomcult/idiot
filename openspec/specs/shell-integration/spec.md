# shell-integration Specification

## Purpose

How idiot changes the user's current shell: the generated shell wrapper, the
contract that stdout carries only eval-able code, and how commands are
dispatched and gated on the wrapper being installed.

## Requirements

### Requirement: Shell wrapper generation
`idiot init <shell>` SHALL print, on stdout, shell code for bash, zsh or fish
that sets `IDIOT_SHELL` to the shell's name and defines an `idiot` function
(and an `iot` alias function) which runs the idiot executable, evaluates
everything it wrote to stdout in the current shell, and returns the
executable's exit status. Any other shell name SHALL fail with an error naming
the supported shells.

#### Scenario: bash integration
- **WHEN** the user runs `eval "$(idiot init bash)"`
- **THEN** `IDIOT_SHELL` is `bash` and `idiot` and `iot` are shell functions in the current shell

#### Scenario: fish integration
- **WHEN** the user runs `idiot init fish | source`
- **THEN** `IDIOT_SHELL` is `fish` and `idiot` and `iot` are fish functions that eval each line of output

#### Scenario: exit status preserved
- **WHEN** a wrapped command exits non-zero
- **THEN** the wrapper function returns that same status after evaluating any output

#### Scenario: unsupported shell
- **WHEN** the user runs `idiot init tcsh`
- **THEN** the command exits non-zero with an error listing bash, zsh and fish

### Requirement: Eval contract for stdout
Every idiot command SHALL write to stdout only statements that are safe to
evaluate in the user's shell (`export`/`unset`, or `set --erase` under fish).
All user-facing output (messages, progress, listings, help, errors, and the
output of child tools) SHALL go to stderr. Exported values SHALL be
single-quoted so that any value round-trips literally.

#### Scenario: Message output
- **WHEN** a command reports progress or results
- **THEN** the text appears on stderr and nothing is evaluated by the wrapper

#### Scenario: Value with special characters
- **WHEN** a command exports a value containing spaces, quotes or `$`
- **THEN** after evaluation the variable holds exactly that value

#### Scenario: Unset under fish
- **WHEN** a command clears a variable and `IDIOT_SHELL` is `fish`
- **THEN** it emits `set --erase VAR`; under other shells it emits `unset VAR`

### Requirement: Command dispatch
`idiot <command> [args]` SHALL run the matching executable under `cmd/`; a
command group (a directory) SHALL dispatch its first argument to the matching
subcommand, up to two levels deep. `help`, `-h` or `--help` (or no
subcommand) SHALL show the group's usage on stderr. An unknown command or
subcommand SHALL fail with an error naming it.

#### Scenario: Group subcommand
- **WHEN** the user runs `idiot store use prod`
- **THEN** the `store use` subcommand runs with `prod` as its argument

#### Scenario: Group help
- **WHEN** the user runs `idiot auth` with no subcommand
- **THEN** the auth usage is printed on stderr and the exit status is zero

#### Scenario: Unknown command
- **WHEN** the user runs `idiot frobnicate`
- **THEN** the command exits non-zero with `unknown command: frobnicate`

### Requirement: Wrapper required for commands
Apart from help, version, status and `init`, a command SHALL refuse to run
when `IDIOT_SHELL` is unset, with an error telling the user how to install
the shell integration.

#### Scenario: Not initialised
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store use prod`
- **THEN** the command exits non-zero and suggests `eval "$(idiot init bash)"` or `idiot init fish | source`

#### Scenario: Help without wrapper
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot store help`
- **THEN** the store usage is shown

### Requirement: Colour handling
Colour escapes SHALL be used only when stderr is a terminal or `--color`
(or `FORCE_COLOR`) is given, and never when `NO_COLOR` is set.

#### Scenario: Piped stderr
- **WHEN** stderr is not a terminal and neither `--color` nor `FORCE_COLOR` is set
- **THEN** output contains no ANSI escape sequences

#### Scenario: NO_COLOR wins
- **WHEN** `NO_COLOR` is set and `--color` is given
- **THEN** output contains no ANSI escape sequences
