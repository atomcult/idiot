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

### Requirement: Comparing two images
`idiot inspect diff [--state] [--tsv] <a.img> <b.img>` SHALL read two
Ubuntu Core disk images the way `idiot inspect image` does and print, on
stdout, only what differs between them:
- the model assertion's `authority-id`, `brand-id`, `model`,
  `display-name`, `architecture`, `base`, `grade`, `store`, `revision`,
  `timestamp`, `classic` and `sign-key-sha3-384` headers, and the gadget
  and kernel snaps the model names (from its `snaps` list, or its
  `gadget`/`kernel` headers for a pre-UC20 model); a header one model has
  and the other lacks SHALL count as a difference;
- the first recovery system's label, whether the image is pre-seeded, and
  its first-boot state (`complete`, `not yet complete` with the mode, or
  `encrypted`);
- every seeded snap present in only one image, and for a snap in both,
  its revision and the model's default channel for it;
- every GPT partition, matched by name, present in only one image, and for
  a partition in both, its start, size and type;
- with `--state`, when both images hold readable snapd state: every
  installed snap present in only one, the revision and tracked channel of
  one in both, and every interface connection present in only one.
Options SHALL come before the images. Snapd's changes SHALL NOT be
compared. When `--state` is given and either image has no readable snapd
state, the command SHALL say so on stderr, naming the image, and compare
the rest. The command SHALL need the same tools as `idiot inspect image`,
SHALL fail on a missing file with `image not found` and on a file that
isn't a Core image as `idiot inspect image` does, naming the file, and
SHALL fail with its usage message unless given exactly two images.

#### Scenario: New kernel revision
- **WHEN** `b.img` was built from the same model as `a.img` with a newer pc-kernel and the user runs `idiot inspect diff a.img b.img`
- **THEN** stdout shows pc-kernel's seeded revision in each image (and the model's `timestamp` if it changed), and nothing about the snaps, partitions or headers that are the same

#### Scenario: Different models
- **WHEN** the user compares an image built from `ubuntu-core-24-amd64` with one built from `ubuntu-core-24-amd64-dangerous`
- **THEN** stdout shows the differing `model` and `grade` (and `sign-key-sha3-384` when different keys signed them), and any seeded snap only one of them has

#### Scenario: Identical images
- **WHEN** the user runs `idiot inspect diff a.img copy-of-a.img`
- **THEN** stdout is empty, stderr says the images don't differ, and the command exits zero

#### Scenario: Images differ
- **WHEN** the images differ in anything compared
- **THEN** the command exits zero after printing the differences

#### Scenario: State compared
- **WHEN** both images have completed first boot without encryption and the user runs `idiot inspect diff --state a.img b.img`
- **THEN** stdout also shows installed snaps and connections that differ

#### Scenario: State unreadable
- **WHEN** the user runs `idiot inspect diff --state fresh.img booted.img` and `fresh.img` has never booted
- **THEN** stderr says `fresh.img` has no readable snapd state, the model, system, seed and partition differences are still printed, and no installed snaps or connections are compared

#### Scenario: Not a Core image
- **WHEN** the user runs `idiot inspect diff a.img README.md`
- **THEN** the command fails saying the ubuntu-seed partition of `README.md` was not found

#### Scenario: One image
- **WHEN** the user runs `idiot inspect diff a.img`
- **THEN** the command fails with `usage: idiot inspect diff [options] <a.img> <b.img>`

### Requirement: Comparison output
`idiot inspect diff` SHALL group its human-readable output by section
(model, system, seed snaps, partitions, and with `--state` installed snaps
and connections), omitting sections with no difference, and SHALL mark
each item present only in the first image `-`, only in the second `+`,
and in both but different `~` with each changed field's two values. It
SHALL color that output only when stdout is a terminal and `NO_COLOR` is
unset. With `--tsv` it SHALL instead print one line per differing field,
with five tab-separated columns: section (`model`, `system`, `seed`,
`partition`, `installed`, `connection`), item (the header, snap,
partition or connection), field, the value in the first image and the
value in the second, an empty value meaning the item is absent from that
image; `--tsv` output SHALL never be colored. Messages SHALL go to stderr
in both modes.

#### Scenario: Piped
- **WHEN** the user runs `idiot inspect diff a.img b.img | less`
- **THEN** the output contains no color escape sequences

#### Scenario: Machine-readable
- **WHEN** the user runs `idiot inspect diff --tsv a.img b.img` and only pc-kernel's revision differs, 1400 in `a.img` and 1500 in `b.img`
- **THEN** stdout is exactly the line `seed`, `pc-kernel`, `revision`, `1400`, `1500` separated by tabs

#### Scenario: Snap only in one image
- **WHEN** `b.img` seeds an optional snap `a.img` lacks and the user runs `idiot inspect diff --tsv a.img b.img`
- **THEN** stdout has a `seed` row for that snap's revision and one for its channel, each with an empty first value

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
signed assertion. It SHALL need python3, from the host or the `core`
component.

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

### Requirement: Assertion signatures
Unless given `--no-verify`, `idiot inspect assertion` SHALL report for
each assertion one of:
- `verified`, naming the trusted key the chain ends at, when its
  signature and each signature up the chain of account-keys verify and
  the chain ends at a key in this machine's snapd assertion database or
  at Canonical's root key, which idiot bundles together with Canonical's
  `models` key (itself checked against the root);
- `invalid`, saying why, when a signature in the chain doesn't match, a
  signing account-key's `account-id` isn't the signed assertion's
  `authority-id`, or the assertion's timestamp is outside the key's
  `since`/`until`;
- `unverified`, naming the missing key, when an account-key in the chain
  isn't in the stream, the local database, or (with `--remote`) the
  store.
It SHALL look up account-keys in the stream first, then in the local
database, then among the bundled keys, and in the store only with
`--remote`; without `--remote` it
SHALL make no network request. It SHALL NOT change the local assertion
database and SHALL NOT need root or store credentials. When `snap` isn't
available it SHALL use only the stream and the bundled keys, and say so
on stderr. The command SHALL exit 1, after printing every assertion,
when any assertion is `invalid`, and 0 otherwise.

#### Scenario: Canonical model, offline
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.model` without `--remote`, on a classic host whose snapd has never seen Canonical's `models` key
- **THEN** the model is `verified` through the bundled `canonical/models` key, ending at `canonical/root`, and no network request is made

#### Scenario: Brand key not known locally
- **WHEN** the user inspects a model signed by a brand key this machine's snapd has never seen, without `--remote`
- **THEN** the model is `unverified`, naming the key's SHA3-384 and suggesting `--remote`, and the command exits zero

#### Scenario: Fetching the key
- **WHEN** the user runs the same command with `--remote` and the store has the brand's account-key
- **THEN** the key is fetched, its own signature checked up to `canonical/root`, and the model is `verified`

#### Scenario: Tampered assertion
- **WHEN** one header of a signed model has been edited after signing
- **THEN** the model is `invalid` (the signature doesn't match), every other assertion in the stream is still reported, and the command exits 1

#### Scenario: Skipping verification
- **WHEN** the user runs `idiot inspect assertion --no-verify pc.model`
- **THEN** the summary is printed with no signature status, `snap` isn't run, and the command exits zero

### Requirement: Assertion cross-checks
For a `model` assertion, `idiot inspect assertion` SHALL warn on stderr
when:
- its `store` header differs from the active store (`UBUNTU_STORE_ID`),
  or it names a store and no store is active, or a store is active and it
  names none; the warning SHALL name the saved store (`idiot store`) whose
  ID is the model's `store`, when there is one;
- a model with the same `brand-id` and `model` exists in a cloned model
  repository (`idiot model`) with a different revision or different
  content, naming that file and both revisions;
- its `authority-id` differs from its `brand-id`.
When a saved model is identical it SHALL say so on stderr, naming the
file. These checks SHALL NOT change the exit status.

#### Scenario: Wrong store active
- **WHEN** `UBUNTU_STORE_ID` is `acme-store-id` and the user inspects a model whose `store` header is `other-store-id`, saved with `idiot store add other other-store-id`
- **THEN** stderr warns the model's store is `other-store-id` (saved as `other`) while `acme-store-id` is active, and the command still exits zero

#### Scenario: Saved model is newer
- **WHEN** the user inspects revision 2 of a model and the canonical clone holds revision 3 of the same brand and model
- **THEN** stderr warns that the saved model differs, naming its file and revision 3

#### Scenario: Matches a saved model
- **WHEN** the user inspects a file copied with `idiot model pick --signed`
- **THEN** stderr says it matches the saved model, naming the file
