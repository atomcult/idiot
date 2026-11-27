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
- **WHEN** a group holds a subgroup directory and the user runs `idiot <group> <subgroup> <command> <arg>`
- **THEN** that command runs with `<arg>` as its argument

#### Scenario: Unknown command
- **WHEN** the user runs `idiot frobnicate`
- **THEN** the command exits non-zero with `unknown command: frobnicate`
