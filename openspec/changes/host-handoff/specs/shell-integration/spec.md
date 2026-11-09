# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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

### Requirement: User environment left alone
Neither `idiot init` nor any command SHALL set, change or unset PATH,
LD_LIBRARY_PATH or any other variable that affects how programs are found or
loaded in the user's shell. Only the variables a command exists to manage
(store, credential, architecture, Launchpad and `IDIOT_*` variables) SHALL
change. A handoff SHALL set idiot's own variables for the host-side process
only. Tools bundled in the snap SHALL get any path or library setup only in
their own process; host tools SHALL run with the user's environment.
`idiot run --shell` SHALL start a child shell with the bundled tools first
on PATH, leaving the calling shell unchanged.

#### Scenario: After init
- **WHEN** the user runs `eval "$(idiot init bash)"`
- **THEN** PATH and LD_LIBRARY_PATH are exactly as before

#### Scenario: After a host command
- **WHEN** the user runs `idiot vm` under the snap and it exits
- **THEN** PATH and LD_LIBRARY_PATH in the shell are exactly as before, and no `IDIOT_HOST`, `IDIOT_SNAP` or `SNAP` variable is set

#### Scenario: Host tools see the user's environment
- **WHEN** a host command runs `qemu-system-x86_64` or `lxc`
- **THEN** that process sees the user's PATH and no LD_LIBRARY_PATH added by idiot

#### Scenario: Child shell
- **WHEN** the user runs `idiot run --shell` and exits that shell
- **THEN** inside it the bundled tools were found first, and the original shell's PATH is unchanged
