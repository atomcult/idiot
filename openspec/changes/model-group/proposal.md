# Proposal

## Why

command-reorg moved the model repositories under `idiot assertion` as
`assertion repos …` and `assertion pick`. In use that reads worse, not
better:
- `assertion repos` is a subgroup inside a group, and the only one in idiot;
- `pick` sits beside `new` and `sign` while working on something else: local clones of git repositories;
- the group's help mixes two subjects.

A `model` group says what it is about in one word.

## What Changes

- **`idiot model` is back:**
  - `list`, `add`, `remove` and `update` manage the model repositories;
  - `pick` copies a model out of them.

  These are the same commands with the same options and output as
  `assertion repos …` and `assertion pick`.
- **`idiot assertion` keeps** `fetch`, `show`, `new`, `edit`, `check` and `sign`. Its help names `idiot model` for models kept in repositories.
- **The aliases turn round.**
  - `model` and `model pick` stop being aliases.
  - `assertion pick` and `assertion repos …` become aliases of `model pick` and `model …`, with the usual note. v0.3.0, already pushed, shipped those names.
- **Main help:** `model` is listed again, under "Assertions & inspection".
- Nothing is **BREAKING**: every v0.3.0 name keeps working.

**stdout and the eval file:** no command's stdout or eval-file output
changes. The alias note goes to stderr only.

There are no new runtime dependencies, and the snap stages nothing new.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `command-layout`:
  - "One group for assertions" gives way to "Assertion and model groups";
  - the alias table and the main help change to match.
- `model`: the requirements name `idiot model …` again.
- `inspect`, `repo-cloning`: the references to `assertion pick` and `assertion repos update` change back.
- `shell-integration`: the two-level dispatch scenario no longer names `assertion repos`, which goes away.

## Impact

- **Command files:** `cmd/assertion/pick` and `cmd/assertion/repos/*` move back to `cmd/model/` (`git mv`).
- **Group help:** `cmd/assertion/help` changes.
- **Aliases:** `share/aliases.sh` changes.
- **Hints:** the ones naming `assertion pick` or `assertion repos` in `cmd/` and `share/` change back.
- **Docs:** README, `skills/using-idiot/` and CLAUDE.md.
- **Tests:** `tests/bare-host.d/aliases`.
