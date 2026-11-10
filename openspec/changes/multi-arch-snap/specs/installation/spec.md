# Spec Delta

## MODIFIED Requirements

### Requirement: Snap install
The snap SHALL be built for amd64, arm64, armhf and riscv64, with every
component available on each. It SHALL be strictly confined and SHALL declare
no plugs or slots. It
SHALL keep idiot's state in the snap's per-user common directory
(`~/snap/<instance>/common`), shared by all revisions and by host commands.
Tools beyond the snap's own fzf SHALL come from optional components (`core`,
`vm`, `tpm`, `network`, `hardware`, `boot`, `debug`; see the bundled-tools
capability), each installed with `snap install idiot+<name>`; without the
one a command needs, the command SHALL fail naming that component.

#### Scenario: No interfaces
- **WHEN** the snap is installed
- **THEN** `snap connections idiot` lists nothing and every command works

#### Scenario: State across refreshes
- **WHEN** a credential was saved and the snap is refreshed to a new revision
- **THEN** `idiot auth list` still shows it

#### Scenario: Inspection tools without the component
- **WHEN** the snap is installed without `core`, the host has no debugfs, and the user runs `idiot run debugfs -V`
- **THEN** the command fails telling the user to install `idiot+core`

#### Scenario: TPM tools without the component
- **WHEN** the snap is installed with `core` but without `tpm`, the host has no tpm2-tools, and the user runs `idiot run tpm2_pcrread`
- **THEN** the command fails telling the user to install `idiot+tpm`

#### Scenario: Bundled tools without the host's
- **WHEN** the snap is installed and the host has no fzf
- **THEN** pickers such as `idiot store use` still work using the snap's fzf

#### Scenario: Components across refreshes
- **WHEN** `idiot+vm` is installed and the snap is refreshed to a new revision
- **THEN** `idiot vm` uses the `vm` component of the new revision without the shell integration being reloaded

#### Scenario: On an arm64 device
- **WHEN** the user runs `snap install idiot idiot+core` on an arm64 Ubuntu Core device
- **THEN** both install and `idiot run sgdisk --version` runs the arm64 build
