# Spec Delta

## ADDED Requirements

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
- **THEN** stdout shows the differing `model`, `grade` and `sign-key-sha3-384`, and any seeded snap only one of them has

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
