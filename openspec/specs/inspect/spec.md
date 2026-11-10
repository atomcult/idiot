# inspect Specification

## Purpose

Looking inside the things Ubuntu Core development produces without booting
or installing them: a Core disk image (its model, seed and, once booted,
snapd's state), a snap file's metadata, and this machine's snap changes.

## Requirements

### Requirement: Image summary
`idiot inspect image <disk.img>` SHALL locate the image's `ubuntu-seed`
partition from its GPT and print, on stderr:
- the model as `<brand-id>/<model>`, from the model assertion of the first
  recovery system in the seed, or `unknown` if there is none;
- whether the image is pre-seeded (`yes` when snapd's state says so);
- the first-boot state: `complete` when ubuntu-data's modeenv says `run`,
  `not yet complete` (with the mode, when one is recorded) otherwise, and
  `encrypted` when ubuntu-data is a LUKS volume;
- every snap in the seed with its revision and the model's default channel
  for it, and every optional snap the model lists but the seed lacks,
  marked `not seeded`.
It SHALL need sfdisk, python3, debugfs and mtools, from the host or the
`core` component. An image without an `ubuntu-seed` partition SHALL fail
saying it may not be an Ubuntu Core image; a missing file SHALL fail with
`image not found`.

#### Scenario: Fresh image
- **WHEN** the user runs `idiot inspect image ubuntu-core-24-amd64.img` on an image that has never booted
- **THEN** it prints the model `canonical/ubuntu-core-24-amd64`, `Pre-seeded: no`, `First boot: not yet complete`, and the seeded snaps (pc-kernel, snapd, console-conf, core24, pc) with revisions and channels

#### Scenario: Not a Core image
- **WHEN** the user runs `idiot inspect image` on a file with no `ubuntu-seed` partition
- **THEN** the command fails saying the ubuntu-seed partition was not found

#### Scenario: Optional snap not seeded
- **WHEN** the model lists an optional snap that the seed doesn't contain
- **THEN** that snap is listed as `not seeded` with the model's channel

### Requirement: Booted image details
When the image's ubuntu-data is readable (not encrypted) and holds snapd's
`state.json`, `idiot inspect image` SHALL also print, on stderr, the
installed snaps (revision and tracked channel), the interface connections
(plug and slot), and the changes (id, status, time and summary), with the
error lines of any change in the Error state under it. An image whose
ubuntu-data is encrypted SHALL get the summary only.

#### Scenario: After first boot
- **WHEN** the image has completed its first boot without full-disk encryption and the user runs `idiot inspect image` on it
- **THEN** `First boot: complete` is printed, followed by the installed snaps, the connections (`(none)` if there are none) and the changes, such as `Initialize system state`

#### Scenario: Encrypted data
- **WHEN** the image's ubuntu-data partition is a LUKS volume
- **THEN** `First boot: encrypted` is printed and no installed snaps, connections or changes

### Requirement: Extracting snapd's state
With `--extract-state [<file>]` (or `--extract-state=<file>`), given
before the image, `idiot inspect image` SHALL also copy the image's snapd
`state.json` to `<file>`, or to `./state.json` when no file is given, and
say where it was saved. The argument after `--extract-state` SHALL be taken
as the file only when another argument follows it. `--extract-state=` with
no file SHALL fail before inspecting. When the image has no readable snapd
state (never booted, or its ubuntu-data encrypted), the command SHALL print
the summary and then fail saying no state could be saved. Options SHALL
come before the image; an option after it SHALL fail with the usage
message.

#### Scenario: Save the state
- **WHEN** the user runs `idiot inspect image --extract-state st.json booted.img` on a booted image
- **THEN** `st.json` holds the image's snapd state as valid JSON and the command exits zero

#### Scenario: Option after the image
- **WHEN** the user runs `idiot inspect image booted.img --extract-state`
- **THEN** the command fails with `usage: idiot inspect image [options] <disk.img>`

#### Scenario: Default file
- **WHEN** the user runs `idiot inspect image --extract-state booted.img` on a booted image
- **THEN** the state is saved to `./state.json`

#### Scenario: Empty file name
- **WHEN** the user runs `idiot inspect image --extract-state= booted.img`
- **THEN** the command fails saying `--extract-state=` needs a file name, before inspecting

#### Scenario: No state in the image
- **WHEN** the user runs `idiot inspect image --extract-state st.json fresh.img` on an image that has never booted
- **THEN** the summary is printed, then the command fails saying the image has no snapd state to save, and `st.json` isn't created

### Requirement: Snap metadata
`idiot inspect snap <snap-file>` SHALL print the snap's `snap/snapcraft.yaml`
if the snap contains one and its `meta/snap.yaml` otherwise, on stderr,
highlighted as YAML when `bat` is available. A file that contains neither
SHALL fail naming both paths; a missing file SHALL fail with `file not
found`. It SHALL need unsquashfs.

#### Scenario: Snap without its snapcraft.yaml
- **WHEN** the user runs `idiot inspect snap idiot_v0.1.0_amd64.snap` (which carries only meta/snap.yaml)
- **THEN** the snap's `meta/snap.yaml` is printed, starting `name: idiot`

#### Scenario: Not a snap
- **WHEN** the user runs `idiot inspect snap README.md`
- **THEN** the command fails saying neither snap/snapcraft.yaml nor meta/snap.yaml was found

### Requirement: Browsing changes
`idiot inspect changes` SHALL show this machine's `snap changes` in an fzf
list, previewing `snap change <id>` for the highlighted change, and SHALL
print nothing and exit zero when the list is closed. It SHALL need fzf and
snap.

#### Scenario: Browse and close
- **WHEN** the user runs `idiot inspect changes`, moves through the list and presses Escape
- **THEN** each highlighted change's tasks were shown in the preview, and the command exits zero printing nothing
