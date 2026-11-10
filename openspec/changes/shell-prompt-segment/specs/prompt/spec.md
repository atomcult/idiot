# Spec Delta

## Purpose

A compact, always-visible summary of the store, arch and credential active
in the current shell, cheap enough to show in the shell prompt on every
redraw.

## ADDED Requirements

### Requirement: Prompt segment
`idiot prompt` SHALL print to stdout the non-empty values of its fields,
in order, joined by `·` and followed by a newline, and SHALL print nothing
at all when every field is empty. The default fields are:
- `store`: `IDIOT_ACTIVE_STORE` when both it and `UBUNTU_STORE_ID` are
  set, otherwise `UBUNTU_STORE_ID`; empty for the global store;
- `arch`: `UBUNTU_STORE_ARCH`; empty when unset (the host default is not
  shown);
- `cred`: `IDIOT_ACTIVE_CRED`, or `auth` when `UBUNTU_STORE_AUTH` or
  `SNAPCRAFT_STORE_CREDENTIALS` is set without a credential name; empty
  when not logged in.
The command SHALL read only environment variables: it SHALL NOT contact
the network or read idiot's state files. It SHALL run without the shell
integration (`IDIOT_SHELL` unset), SHALL write nothing to stderr or the
eval file on success, SHALL exit zero, and SHALL NOT change the usage
counter. Under the snap it SHALL run confined.

#### Scenario: Everything active
- **WHEN** the user has run `idiot store use staging`, `idiot arch set arm64` and `idiot auth login work`, then runs `idiot prompt`
- **THEN** stdout is `staging·arm64·work` followed by a newline

#### Scenario: Partly active
- **WHEN** only `UBUNTU_STORE_ARCH=arm64` is set
- **THEN** stdout is `arm64` followed by a newline, with no leading or trailing separator

#### Scenario: Nothing active
- **WHEN** no store, arch or credential is set and the user runs `idiot prompt | wc -c`
- **THEN** `wc` prints `0` and the exit status is zero

#### Scenario: Store ID without a name
- **WHEN** `UBUNTU_STORE_ID` is `abc123` and `IDIOT_ACTIVE_STORE` is unset
- **THEN** the store field is `abc123`

#### Scenario: Unnamed credential
- **WHEN** `SNAPCRAFT_STORE_CREDENTIALS` is set and `IDIOT_ACTIVE_CRED` is not
- **THEN** the cred field is `auth`

#### Scenario: Without the shell integration
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot prompt`
- **THEN** it prints the segment and exits zero

#### Scenario: Not counted
- **WHEN** the user runs `idiot prompt` many times
- **THEN** the invocation count shown by `idiot status` is unchanged

### Requirement: Segment format
`idiot prompt --format <fields>` SHALL print the comma-separated fields
given, in that order, instead of the default `store,arch,cred`. Besides
the default fields it SHALL accept `lp` (`IDIOT_LP_USER`, empty when unset)
and `profile` (`IDIOT_ACTIVE_PROFILE`, empty when unset).
`--separator <sep>` SHALL replace `·` with `<sep>`. Empty fields SHALL be
skipped without leaving a separator. An unknown field SHALL fail with an
error naming it and the known fields, on stderr, with a non-zero exit and
nothing on stdout.

#### Scenario: Custom fields
- **WHEN** `IDIOT_ACTIVE_CRED` is `work`, `UBUNTU_STORE_ARCH` is `arm64` and the user runs `idiot prompt --format cred,arch`
- **THEN** stdout is `work·arm64`

#### Scenario: Custom separator
- **WHEN** the store is `staging` and the arch `arm64`, and the user runs `idiot prompt --separator ' | '`
- **THEN** stdout is `staging | arm64`

#### Scenario: Unknown field
- **WHEN** the user runs `idiot prompt --format store,colour`
- **THEN** the command exits non-zero naming `colour` and the known fields, and stdout is empty

### Requirement: Prompt variable
The shell integration from `idiot init` SHALL keep an exported variable
`IDIOT_PROMPT` equal to what `idiot prompt` prints with the default
fields (without the newline), computing it in the user's shell without
starting any process. It SHALL update it when the integration is loaded
and after every call of the `idiot` (or `iot`) function, whatever the
command's exit status, and SHALL unset it when the segment is empty. The
bookkeeping SHALL NOT change the function's return status. Changes made to
the underlying variables by other means show up in `IDIOT_PROMPT` after
the next `idiot` call.

#### Scenario: After switching store
- **WHEN** the user runs `idiot store use staging` in bash, zsh or fish
- **THEN** `IDIOT_PROMPT` starts with `staging` in that shell

#### Scenario: After logging out of everything
- **WHEN** the user runs `idiot store unset`, `idiot arch unset` and `idiot auth logout`
- **THEN** `IDIOT_PROMPT` is unset

#### Scenario: Visible to child processes
- **WHEN** `IDIOT_PROMPT` is `staging·arm64` and a prompt program such as starship runs
- **THEN** it sees `IDIOT_PROMPT=staging·arm64` in its environment

#### Scenario: No process per prompt
- **WHEN** a prompt shows `$IDIOT_PROMPT` and the user presses Enter on an empty line
- **THEN** no idiot process (and no other process) is started to compute the segment

#### Scenario: Status preserved
- **WHEN** a wrapped command exits with status 3
- **THEN** `IDIOT_PROMPT` is updated and the `idiot` function still returns 3
