# Spec Delta

## MODIFIED Requirements

### Requirement: Removing a repository
`idiot model remove [<name>]` SHALL delete a user repository's registration
and its clone, offering an fzf picker of the user repositories when no name
is given (cancelling changes nothing). It SHALL refuse a built-in
repository, an unknown name, and any name that isn't a repository name
(letters, digits, `-` and `_`), before deleting anything.

#### Scenario: Remove
- **WHEN** `mine` is registered and cloned and the user runs `idiot model remove mine`
- **THEN** its registration and its clone are both deleted

#### Scenario: Built-in
- **WHEN** the user runs `idiot model remove canonical`
- **THEN** the command fails saying a built-in repository can't be removed

#### Scenario: Path as a name
- **WHEN** the user runs `idiot model remove ../auth/work`
- **THEN** the command fails with an invalid-name error and no file is deleted


### Requirement: Updating clones
`idiot model update <name>` SHALL clone the repository into
`IDIOT_MODELS_DIR/<name>`, or pull it if already cloned, and fail if that
fails, the name is unknown, or it isn't a repository name (letters,
digits, `-` and `_`); a name is checked before any file is read. With no name it SHALL update every built-in
and user repository, carrying on past failures (each reported as a warning)
and exiting non-zero if any failed; with none configured it SHALL fail.
A models directory from before multi-repository support (a single clone at
`IDIOT_MODELS_DIR` itself) SHALL first be moved to `IDIOT_MODELS_DIR/canonical`.
It runs on the host (it needs git and the network).

#### Scenario: First update
- **WHEN** the user runs `idiot model update canonical` and it isn't cloned
- **THEN** `https://github.com/canonical/models` is cloned into `IDIOT_MODELS_DIR/canonical`

#### Scenario: Unknown repository
- **WHEN** the user runs `idiot model update nope`
- **THEN** the command fails with `unknown model repo 'nope'`

#### Scenario: One of several fails
- **WHEN** two repositories are configured and one can't be fetched
- **THEN** the other is still updated, a warning names the failed one, and the exit status is non-zero

#### Scenario: Path as a name
- **WHEN** the user runs `idiot model update ../auth/work`
- **THEN** the command fails with an invalid-name error, without reading the file or running git
