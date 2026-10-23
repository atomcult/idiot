# Spec Delta

## MODIFIED Requirements

### Requirement: Importing and exporting
`idiot auth import <name> <file>` SHALL copy an existing credential file in
under `<name>`, and SHALL fail with its usage when either argument is
missing, and refuse an existing name or a missing file.
`idiot auth export [<name>] [-f <file>]` SHALL output a saved credential, to
`<file>` when given and otherwise to stdout; without a name it SHALL offer an
fzf picker.

#### Scenario: Import
- **WHEN** the user runs `idiot auth import ci ./ci.cred`
- **THEN** a credential named `ci` with the file's contents is saved

#### Scenario: Import missing file
- **WHEN** the user runs `idiot auth import ci ./missing`
- **THEN** the command fails with `file not found`

#### Scenario: Import missing argument
- **WHEN** the user runs `idiot auth import ci`
- **THEN** the command fails with `usage: idiot auth import <name> <file>` and saves nothing

#### Scenario: Export to file
- **WHEN** the user runs `idiot auth export ci -f out.cred`
- **THEN** `out.cred` holds the credential and a confirmation is printed on stderr

## ADDED Requirements

### Requirement: Credential names
Commands that create a credential (`auth add`, `auth import`) SHALL accept
only names made of letters, digits, `.`, `-` and `_` that do not start with
`.`, and SHALL reject any other name without writing anything.

#### Scenario: Path in name
- **WHEN** the user runs `idiot auth import ../../.bashrc ./ci.cred`
- **THEN** the command fails with an invalid-name error and nothing is written

### Requirement: Private credential files
The credentials directory SHALL be accessible only by its owner (0700) and
every credential file idiot writes, including the target of
`auth export -f`, SHALL be readable and writable only by its owner (0600),
regardless of the source file's mode or the user's umask.

#### Scenario: Import from a world-readable file
- **WHEN** the user imports a credential from a file with mode 0664
- **THEN** the saved credential has mode 0600

#### Scenario: Export to a file
- **WHEN** the user runs `idiot auth export ci -f out.cred`
- **THEN** `out.cred` has mode 0600
