# shell-integration Specification

## Purpose

How idiot changes the user's current shell: the generated shell wrapper, the
contract that environment changes travel through a private eval file while
stdout stays plain output, and how commands are dispatched and gated on the
wrapper being installed.

## Requirements

### Requirement: Shell wrapper generation
`idiot init <shell>` SHALL print, on stdout, shell code for bash, zsh or fish
that sets `IDIOT_SHELL` to the shell's name and defines an `idiot` function
(and an `iot` alias function). For each call the function SHALL create a
private eval file readable only by the user, in a directory the idiot
executable can write (under the snap, the snap's per-user runtime
directory), pass its path to the executable in `IDIOT_EVAL_FILE`, and let
the executable's stdout and stderr reach the caller unchanged. Afterwards it
SHALL read the eval file, delete it, and evaluate what it read in the
current shell. It SHALL return the executable's exit status when that is
non-zero, and otherwise the status of the evaluation. Any other shell name
SHALL fail with an error naming the supported shells.

#### Scenario: bash integration
- **WHEN** the user runs `eval "$(idiot init bash)"`
- **THEN** `IDIOT_SHELL` is `bash` and `idiot` and `iot` are shell functions in the current shell

#### Scenario: fish integration
- **WHEN** the user runs `idiot init fish | source`
- **THEN** `IDIOT_SHELL` is `fish` and `idiot` and `iot` are fish functions that evaluate the eval file

#### Scenario: exit status preserved
- **WHEN** a wrapped command exits non-zero
- **THEN** the wrapper function returns that same status after evaluating anything the command wrote to the eval file

#### Scenario: eval file cleaned up
- **WHEN** a wrapped command finishes, fails, or is interrupted, including during a command handed off to the host
- **THEN** its eval file no longer exists

#### Scenario: unsupported shell
- **WHEN** the user runs `idiot init tcsh`
- **THEN** the command exits non-zero with an error listing bash, zsh and fish

### Requirement: Eval contract for stdout
Commands that change the user's environment SHALL write their
`export`/`unset` statements (`set --erase` under fish) only to the file named
by `IDIOT_EVAL_FILE`, with values single-quoted so any value round-trips
literally. Nothing written to stdout SHALL ever be evaluated. stdout SHALL be
reserved for a command's data output (including the output of child tools
such as `idiot run`), so it can be piped or redirected; messages, progress,
help and errors SHALL go to stderr.

#### Scenario: Message output
- **WHEN** a command reports progress or results
- **THEN** the text appears on stderr

#### Scenario: Value with special characters
- **WHEN** a command exports a value containing spaces, quotes or `$`
- **THEN** after evaluation the variable holds exactly that value

#### Scenario: Unset under fish
- **WHEN** a command clears a variable and `IDIOT_SHELL` is `fish`
- **THEN** it writes `set --erase VAR`; under other shells it writes `unset VAR`

#### Scenario: Tool output is not evaluated
- **WHEN** the user runs `idiot run cat notes.txt` and the file contains `touch /tmp/pwned`
- **THEN** the line is printed to stdout and `/tmp/pwned` is not created

#### Scenario: Data output in a pipe
- **WHEN** the user runs `idiot auth export work | wc -c`
- **THEN** `wc` receives the credential and nothing is evaluated or echoed to the terminal

#### Scenario: Interactive shell
- **WHEN** the user runs `idiot run --shell`
- **THEN** the shell is interactive with its output on the terminal as it happens

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

#### Scenario: Outdated wrapper
- **WHEN** a shell loaded the previous integration and the user runs `idiot store use prod`
- **THEN** the command exits non-zero telling the user to re-run `idiot init` and `UBUNTU_STORE_ID` is unchanged

### Requirement: Colour handling
Colour escapes SHALL be used only when stderr is a terminal or `--color`
(or `FORCE_COLOR`) is given, and never when `NO_COLOR` is set.

#### Scenario: Piped stderr
- **WHEN** stderr is not a terminal and neither `--color` nor `FORCE_COLOR` is set
- **THEN** output contains no ANSI escape sequences

#### Scenario: NO_COLOR wins
- **WHEN** `NO_COLOR` is set and `--color` is given
- **THEN** output contains no ANSI escape sequences

### Requirement: Host commands
Under the snap, a command that needs anything beyond idiot's own state and
the bundled fzf (host tools, the network, devices, or files the user names)
SHALL run outside confinement: the confined command SHALL write to the eval
file a single invocation of the host-side idiot from
`/snap/<instance>/current` with the same command and arguments, and exit
zero; the wrapper's evaluation runs it in the user's shell, and its exit
status becomes the wrapper's. Help for such a command SHALL be shown without
a handoff. Outside a snap every command SHALL run directly.

#### Scenario: VM from the snap
- **WHEN** idiot is installed as a snap and the user runs `idiot vm --arch arm64`
- **THEN** QEMU runs from the user's shell, outside confinement, and the wrapper returns QEMU's exit status

#### Scenario: State-only command
- **WHEN** idiot is installed as a snap and the user runs `idiot store use prod`
- **THEN** the command runs confined and exports `UBUNTU_STORE_ID` without a handoff

#### Scenario: Survives refresh
- **WHEN** the snap is refreshed while a shell has the integration loaded
- **THEN** the next host command runs the new revision's files

#### Scenario: Help stays confined
- **WHEN** the user runs `idiot vm --help` under the snap
- **THEN** the usage is shown and nothing is handed off

### Requirement: Host environment from init
Under the snap, the integration printed by `idiot init` SHALL also export
`IDIOT_SNAP` (`/snap/<instance>/current`), `IDIOT_DATA_HOME` and
`IDIOT_CACHE_DIR` (in the snap's per-user common directory), so host
commands find the snap's files and the same state as confined ones. It
SHALL NOT export `SNAP`. A host command run without `IDIOT_SNAP` SHALL fail
telling the user to re-run `idiot init`.

#### Scenario: Snap integration
- **WHEN** the user loads the integration from the snap
- **THEN** `IDIOT_SNAP` is `/snap/idiot/current`, `IDIOT_DATA_HOME` is `~/snap/idiot/common`, and `SNAP` is unset

#### Scenario: Source integration
- **WHEN** the user loads the integration from a source install
- **THEN** only `IDIOT_SHELL` is exported

### Requirement: PATH and LD_LIBRARY_PATH only as needed
Neither `idiot init` nor any command SHALL change PATH or LD_LIBRARY_PATH in
the user's shell. A tool bundled in the snap SHALL get any PATH or library
setup it needs only in its own process; host tools SHALL run with the
user's PATH and libraries. `idiot run --shell` SHALL start a child shell
with the bundled tools first on PATH, leaving the calling shell unchanged.

#### Scenario: After init
- **WHEN** the user runs `eval "$(idiot init bash)"`
- **THEN** PATH and LD_LIBRARY_PATH are exactly as before

#### Scenario: After a host command
- **WHEN** the user runs `idiot vm` under the snap and it exits
- **THEN** PATH and LD_LIBRARY_PATH in the shell are exactly as before

#### Scenario: Host tools see the user's environment
- **WHEN** a host command runs `qemu-system-x86_64` or `lxc`
- **THEN** that process sees the user's PATH and no LD_LIBRARY_PATH added by idiot

#### Scenario: Child shell
- **WHEN** the user runs `idiot run --shell` and exits that shell
- **THEN** inside it the bundled tools were found first, and the original shell's PATH is unchanged
