# Proposal

## Why

Telling two Core images apart (yesterday's build and today's, a dangerous
and a signed one, the image a customer has and the one we shipped) means
running `idiot inspect image` on each and comparing the screens by eye:
a changed kernel revision, a different gadget channel or a resized
partition is easy to miss. The extraction `inspect image` already does is
all a comparison needs, but it lives in one file as globals and can't be
called twice.

## What Changes

- New `idiot inspect diff [--state] [--tsv] <a.img> <b.img>`: compares
  two Ubuntu Core images and prints only what differs:
  - the model assertion's headers (brand, model, grade, architecture,
    base, store, revision, timestamp, signing key, display name) and the
    gadget and kernel it names;
  - the recovery system label, whether each image is pre-seeded, and its
    first-boot state;
  - every seeded snap: added, removed, or a different revision or default
    channel;
  - the GPT partitions, by name: added, removed, or a different start,
    size or type;
  - with `--state`, snapd's state in each image: installed snaps
    (revision, tracked channel) and interface connections.
- The comparison goes to **stdout** (it is the data the user asked for),
  colored only when stdout is a terminal and `NO_COLOR` is unset. With
  `--tsv` it is one tab-separated row per differing field
  (`section`, `item`, `field`, `a`, `b`), never colored, like
  `idiot run --list`. Identical images print nothing on stdout and say so
  on stderr. The exit status is 0 whether or not the images differ.
- The extraction moves from `cmd/inspect/image` into a new
  `share/image.sh` module both commands use; it reads each image's
  partition table once (today `sfdisk` runs twice per image) and writes
  what it finds as small files in a work directory instead of globals.
  `idiot inspect image`'s output is unchanged.
- `share/common.sh` gains `stdout_colors`, the stdout counterpart of the
  existing stderr color variables.

Output channels: `inspect diff` writes to stdout (new command);
`inspect image` still writes only to stderr. Nothing is queued in the
eval file. No new runtime dependencies: the same sfdisk, python3,
debugfs and mtools `inspect image` needs, from the host or the `core`
component, which the snap already stages. `inspect diff` is a host
command (it reads the user's files and needs `core`'s tools).

## Capabilities

### New Capabilities

### Modified Capabilities
- `inspect`: a new requirement for comparing two images (and its
  machine-readable form); `inspect image`'s requirements are unchanged.

## Impact

New `cmd/inspect/diff` and `share/image.sh`; `cmd/inspect/image` reduced
to argument parsing and printing; `cmd/inspect/help` lists `diff`;
`share/common.sh` (`stdout_colors`). README's inspect section if it lists
the subcommands.
