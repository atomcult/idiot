# Proposal

## Why

Writing the inspect and model baselines turned up five bugs, two of them
about files idiot shouldn't touch:

- `idiot model update ../auth/<name>` reads a saved credential's first
  line as a git URL, and git's error prints it to the terminal.
- `idiot model remove ../<path>` deletes files outside the model registry.
  `model add` validates names; `remove` and `update` don't.
- `idiot inspect image --extract-state <img>` takes the image as the file
  name and fails with the usage message, so the documented default
  (`./state.json`) can't be reached.
- `--extract-state=` (empty) saves nothing and exits zero.
- `--extract-state` on an image with no snapd state (never booted, or its
  data encrypted) saves nothing and exits zero.

## What Changes

- `idiot model remove` and `idiot model update` accept only the names
  `model add` does (letters, digits, `-`, `_`) and fail on any other
  before reading or deleting anything.
- `--extract-state` takes the next argument as the file only when another
  argument (the image) follows it; otherwise it saves to `./state.json`.
- `--extract-state=` with no file name fails before inspecting.
- When `--extract-state` was asked for and the image has no readable snapd
  state, the command prints the summary and then fails saying why.

stdout: unchanged (everything here is stderr). Nothing is queued in the
eval file. No new dependencies.

## Capabilities

### New Capabilities

### Modified Capabilities
- `model`: removing and updating refuse names that aren't repository
  names.
- `inspect`: `--extract-state`'s default file, an empty value, and an image
  with no state.

## Impact

`share/models.sh` (one name check shared by add, remove and update),
`cmd/model/{add,remove,update}`, `cmd/inspect/image`.
