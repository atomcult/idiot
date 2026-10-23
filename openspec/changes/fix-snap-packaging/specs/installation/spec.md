# Spec Delta

## Purpose

How idiot is installed (from source with `make install`, or as a snap) and
the guarantees the shell integration relies on, such as a command path that
stays valid across upgrades.

## ADDED Requirements

### Requirement: Source install
`make install` SHALL install idiot under `PREFIX` (default `~/.local`) with a
launcher at `PREFIX/bin/idiot`; `make uninstall` SHALL remove both, and
`make purge` SHALL additionally remove idiot's user data and cache.

#### Scenario: Default prefix
- **WHEN** the user runs `make install`
- **THEN** `~/.local/bin/idiot` runs idiot and `idiot version` shows the installed version

### Requirement: Snap install
The snap SHALL install with classic confinement and SHALL run the same
commands with the same data locations (`~/.local/share/idiot`,
`~/.cache/idiot`) as a source install. Device-inspection tools SHALL be
available from the optional `core` component (`snap install idiot+core`);
without it, commands needing them SHALL fail naming the component.

#### Scenario: Shared data
- **WHEN** credentials were saved with a source install and the user switches to the snap
- **THEN** `idiot auth list` shows the same credentials

#### Scenario: Inspection tools without the component
- **WHEN** the snap is installed without `core` and the user runs `idiot run debugfs -V`
- **THEN** the command fails telling the user to install `idiot+core`

### Requirement: Stable command path in shell integration
`idiot init` SHALL embed a path to the idiot command that remains valid
across upgrades of the same installation: for the snap, its `/snap/bin`
command; for a source install, the installed launcher. It SHALL NOT embed a
path inside a specific snap revision.

#### Scenario: Snap refresh
- **WHEN** a shell loaded the integration and the snap is then refreshed to a new revision (and old revisions are removed)
- **THEN** `idiot status` in that shell still runs, using the new revision

#### Scenario: Bundled tools in the snap
- **WHEN** the snap is installed and the host has no fzf
- **THEN** pickers such as `idiot store use` still work using the snap's fzf
