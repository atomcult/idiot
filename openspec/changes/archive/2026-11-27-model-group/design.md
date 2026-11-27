# Design

## Context

command-reorg (archived 2026-11-27, shipped in v0.3.0) added the alias table in `share/aliases.sh`. The dispatcher rewrites an old name before dispatch, preferring a two-word match.

`assertion` is a real group, but `assertion pick` and `assertion repos` can still be aliases. The table is consulted before the group's directory is, so the rewrite wins:
- `idiot assertion repos list` matches the two-word entry `assertion repos`;
- it becomes `idiot model list`, with the remaining words passed on.

## Goals / Non-Goals

**Goals:**
- `model` reads as it did before v0.3.0.
- Every v0.3.0 name keeps working, with a note.

**Non-Goals:**
- Changing what the model commands do.
- Moving `assertion fetch` or `assertion show` again.

## Decisions

### Move the files back with `git mv`
`cmd/assertion/pick` → `cmd/model/pick` and `cmd/assertion/repos/*` → `cmd/model/*`, fixing each file's `common.sh` depth, usage strings and hints. History follows through both moves.

### Swap the alias lines
The `model pick|assertion pick` and `model|assertion repos` lines become:

```
assertion pick|model pick
assertion repos|model
```

`assertion repos` with nothing after it becomes `model`, which shows the model help, as `assertion repos` did.

*Alternative:* keep `assertion pick` and `repos` as real commands alongside `model`. That would give one thing two names in help, which is the confusion this change removes.

### Two-level dispatch stays
`dispatch_subcmd` keeps supporting a subgroup, even though no group has one after this change. The shell-integration scenario now describes it generically, because removing working dispatch code is outside the scope of this change.

## Risks / Trade-offs

- **[Risk]** Someone learned `assertion repos` in v0.3.0. → It keeps working, and the note names `model`.
- **[Trade-off]** v0.3.0's notes say `model` is now `assertion repos`. The next tag's notes will say the reverse.
