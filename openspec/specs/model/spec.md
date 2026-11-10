# model Specification

## Purpose

Keeping local clones of model-assertion repositories (Canonical's, plus any
the user registers) and copying a model assertion out of them, signed
(`*.model`) or unsigned (`*.json`), for building or signing images.

## Requirements

### Requirement: Model repositories
idiot SHALL know two registries of model repositories, each a directory of
files named after the repository and holding its git URL: the built-in one
shipped with idiot (`canonical` → `https://github.com/canonical/models`)
and the user's, `IDIOT_MODEL_REPOS_DIR` (default
`$XDG_DATA_HOME/idiot/model-repos`). Clones SHALL live in
`IDIOT_MODELS_DIR/<name>` (default `$XDG_DATA_HOME/idiot/models`).
`idiot model list` SHALL print both registries on stderr, each repository
with its URL and a mark when it is cloned, and `(none)` for an empty one.

#### Scenario: Nothing cloned yet
- **WHEN** no repository has been cloned and the user runs `idiot model list`
- **THEN** `canonical` is listed under Built-in with its URL and no clone mark, and User shows `(none)`

#### Scenario: Cloned
- **WHEN** `canonical` has been cloned
- **THEN** `idiot model list` marks it with `✓`

### Requirement: Registering a repository
`idiot model add <name> <url>` SHALL save a user repository, accepting only
names made of letters, digits, `-` and `_`. It SHALL refuse a name a
built-in repository has, and a name already registered, and SHALL NOT clone
anything (it says to run `idiot model update <name>`).

#### Scenario: Add
- **WHEN** the user runs `idiot model add mine https://example.com/models.git`
- **THEN** `mine` appears under User in `idiot model list`, not yet cloned

#### Scenario: Invalid name
- **WHEN** the user runs `idiot model add ../x https://example.com/m.git`
- **THEN** the command fails with an invalid-name error and nothing is written

#### Scenario: Built-in name
- **WHEN** the user runs `idiot model add canonical https://example.com/m.git`
- **THEN** the command fails saying it can't shadow the built-in repository

#### Scenario: Duplicate
- **WHEN** `mine` is registered and the user runs `idiot model add mine <url>`
- **THEN** the command fails telling the user to remove it first

### Requirement: Removing a repository
`idiot model remove [<name>]` SHALL delete a user repository's registration
and its clone, offering an fzf picker of the user repositories when no name
is given (cancelling changes nothing). It SHALL refuse a built-in
repository and an unknown name.

#### Scenario: Remove
- **WHEN** `mine` is registered and cloned and the user runs `idiot model remove mine`
- **THEN** its registration and its clone are both deleted

#### Scenario: Built-in
- **WHEN** the user runs `idiot model remove canonical`
- **THEN** the command fails saying a built-in repository can't be removed

### Requirement: Updating clones
`idiot model update <name>` SHALL clone the repository into
`IDIOT_MODELS_DIR/<name>`, or pull it if already cloned, and fail if that
fails or the name is unknown. With no name it SHALL update every built-in
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

### Requirement: Picking a model assertion
`idiot model pick [--signed|--unsigned] [-o <file>|-o -] [<dir>]` SHALL
offer, in fzf with a preview, every `*.model` (`--signed`), `*.json`
(`--unsigned`) or both found in the clones, and copy the selected file into
`<dir>` (default: the current directory) under its own name, into `<file>`
with `-o`, or to stdout with `-o -`; it SHALL say where it copied the file
(on stderr). Cancelling SHALL write nothing and exit zero. It SHALL fail
when nothing is cloned (pointing at `idiot model update`), when no file
matches, when `<dir>` doesn't exist, and when a directory follows `-o`.

#### Scenario: To stdout
- **WHEN** the user runs `idiot model pick --signed -o - >pc.model` and selects `ubuntu-core-24-amd64-dangerous.model`
- **THEN** `pc.model` holds the assertion (starting `type: model`) and nothing else

#### Scenario: Into a directory
- **WHEN** the user runs `idiot model pick --unsigned ./out` and selects `canonical/ubuntu-core-24-pi-arm64.json`
- **THEN** `./out/ubuntu-core-24-pi-arm64.json` is written

#### Scenario: Nothing cloned
- **WHEN** no repository is cloned and the user runs `idiot model pick`
- **THEN** the command fails telling the user to run `idiot model update` first

#### Scenario: Missing directory
- **WHEN** the user runs `idiot model pick ./nodir` and `./nodir` doesn't exist
- **THEN** the command fails saying the destination directory does not exist

#### Scenario: Cancelled
- **WHEN** the user closes the picker without choosing
- **THEN** nothing is written and the command exits zero
