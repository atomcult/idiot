# Spec Delta

## MODIFIED Requirements

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
It SHALL need sfdisk, debugfs and mtools, from the host or the `core`
component, and SHALL NOT need python3. An image without an `ubuntu-seed` partition SHALL fail
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

#### Scenario: Host without Python
- **WHEN** the host has no python3 (an Ubuntu Core 26 device with `idiot+core` installed) and the user runs `idiot inspect image ubuntu-core-24-amd64.img`
- **THEN** the same summary is printed as on a host with python3

### Requirement: Assertion summary
`idiot inspect assertion [--remote] [--no-verify] <file|->` SHALL read an
assertion stream from the file, or from stdin when given `-`, and print on
stdout, for each assertion in the order they appear:
- its type, its primary key headers, `authority-id`, the brand or account
  it is about, `sign-key-sha3-384`, and its `timestamp` and `revision`
  when present;
- its remaining headers, a multi-line header summarised by its number of
  entries unless the type's summary below shows it;
- for `model`: each snap with its type, default channel, presence and id,
  and each validation set with its mode (a pre-UC20 model: its kernel,
  gadget and required snaps);
- for `account-key`: the public key's algorithm, size and SHA3-384, and
  its `since`/`until` validity;
- for `validation-set`: each snap with its presence and revision;
- for `system-user`: its models, `since`/`until` and the number of SSH
  keys, showing only whether a password is set and never the password
  hash;
- for `snap-declaration` and `snap-revision`: the snap's id, name,
  publisher or developer, and for a revision its number, size and
  SHA3-384;
- for any other type: its headers and its body's size.
It SHALL color that output only when stdout is a terminal and `NO_COLOR`
is unset. It SHALL fail on a missing file, on input with no assertion in
it, and on an assertion it can't parse (naming its position in the
stream). JSON input SHALL fail saying it is an unsigned model, not a
signed assertion. It SHALL NOT need python3 or any other tool from the host or a
component; `snap`, when it is installed, only adds keys.

#### Scenario: A signed model
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.model`
- **THEN** stdout shows `type: model`, brand `canonical`, model `ubuntu-core-24-amd64`, grade `signed`, its signing key, and a table of its snaps (pc, pc-kernel, core24, snapd, console-conf…) with type, default channel, presence and id

#### Scenario: From stdin
- **WHEN** the user runs `snap known account-key account-id=canonical | idiot inspect assertion -`
- **THEN** every account-key in the stream is summarised in turn, each with its name, key size and SHA3-384

#### Scenario: System user password
- **WHEN** the user inspects a system-user assertion that carries a password hash
- **THEN** the output says a password is set and contains no part of the hash

#### Scenario: Unsigned model
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.json`
- **THEN** the command fails saying the file is an unsigned model, not a signed assertion

#### Scenario: Not an assertion
- **WHEN** the user runs `idiot inspect assertion README.md`
- **THEN** the command fails saying no assertion was found

#### Scenario: Host without Python
- **WHEN** the host has no python3 and the user runs `idiot inspect assertion ubuntu-core-24-amd64.model`
- **THEN** the same summary is printed, and the signature is checked, as on a host with python3
