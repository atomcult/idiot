# installation Specification

## Purpose

How idiot is installed (from source with `make install`, or as a snap),
where each install keeps its state, and the guarantees the shell
integration relies on, such as a command path that stays valid across
upgrades.

## Requirements

### Requirement: Source install
`make install` SHALL install idiot under `PREFIX` (default `~/.local`) with a
launcher at `PREFIX/bin/idiot`; `make uninstall` SHALL remove both, and
`make purge` SHALL additionally remove idiot's user data and cache.

#### Scenario: Default prefix
- **WHEN** the user runs `make install`
- **THEN** `~/.local/bin/idiot` runs idiot and `idiot version` shows the installed version

### Requirement: Snap install
The snap SHALL be strictly confined and SHALL declare no plugs or slots. It
SHALL keep idiot's state in the snap's per-user common directory
(`~/snap/<instance>/common`), shared by all revisions and by host commands.
Device-inspection tools SHALL be available from the optional `core`
component (`snap install idiot+core`); without it, commands needing them
SHALL fail naming the component.

#### Scenario: No interfaces
- **WHEN** the snap is installed
- **THEN** `snap connections idiot` lists nothing and every command works

#### Scenario: State across refreshes
- **WHEN** a credential was saved and the snap is refreshed to a new revision
- **THEN** `idiot auth list` still shows it

#### Scenario: Inspection tools without the component
- **WHEN** the snap is installed without `core` and the user runs `idiot run debugfs -V`
- **THEN** the command fails telling the user to install `idiot+core`

#### Scenario: Bundled tools without the host's
- **WHEN** the snap is installed and the host has no fzf
- **THEN** pickers such as `idiot store use` still work using the snap's fzf

### Requirement: Data directory
idiot's data directory SHALL be `${XDG_DATA_HOME:-~/.local/share}/idiot` for
a source install and `~/snap/<instance>/common` for the snap, and
`IDIOT_DATA_HOME` SHALL override it. The default locations other
capabilities name under `$XDG_DATA_HOME/idiot` (credentials, stores, models,
the Launchpad user) SHALL be read relative to this directory; their own
override variables (`IDIOT_AUTH_DIR`, `IDIOT_STORES_DIR`, …) still win.

#### Scenario: Snap credentials
- **WHEN** idiot is installed as a snap and the user runs `idiot auth add work`
- **THEN** the credential is saved in `~/snap/idiot/common/auth/work`

#### Scenario: Source and snap kept apart
- **WHEN** credentials were saved with a source install and the user switches to the snap
- **THEN** `idiot auth list` shows none until the user copies `~/.local/share/idiot` into `~/snap/idiot/common`

### Requirement: Stable command path in shell integration
`idiot init` SHALL embed a path to the idiot command that remains valid
across upgrades of the same installation: for the snap, its `/snap/bin`
command; for a source install, the installed launcher. It SHALL NOT embed a
path inside a specific snap revision.

#### Scenario: Snap refresh
- **WHEN** a shell loaded the integration and the snap is then refreshed to a new revision (and old revisions are removed)
- **THEN** `idiot status` in that shell still runs, using the new revision
