# Spec Delta

## MODIFIED Requirements

### Requirement: Shell wrapper generation
`idiot init <shell>` SHALL print, on stdout, shell code for bash, zsh or fish
that sets `IDIOT_SHELL` to the shell's name and defines an `idiot` function
(and an `iot` alias function). For each call the function SHALL create a
private eval file readable only by the user, pass its path to the idiot
executable in `IDIOT_EVAL_FILE`, let the executable's stdout and stderr reach
the caller unchanged, then evaluate the eval file in the current shell, delete
it, and return the executable's exit status. Any other shell name SHALL fail
with an error naming the supported shells.

#### Scenario: bash integration
- **WHEN** the user runs `eval "$(idiot init bash)"`
- **THEN** `IDIOT_SHELL` is `bash` and `idiot` and `iot` are shell functions in the current shell

#### Scenario: fish integration
- **WHEN** the user runs `idiot init fish | source`
- **THEN** `IDIOT_SHELL` is `fish` and `idiot` and `iot` are fish functions that source the eval file

#### Scenario: exit status preserved
- **WHEN** a wrapped command exits non-zero
- **THEN** the wrapper function returns that same status after evaluating anything the command wrote to the eval file

#### Scenario: eval file cleaned up
- **WHEN** a wrapped command finishes, fails, or is interrupted
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
