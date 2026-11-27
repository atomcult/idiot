# Proposal

## Why

snapd's model assertion has two shapes that exclude each other, and the
writer only knows one of them:

| | Ubuntu Core model | Classic model (`classic: true`, with `snaps`) |
|---|---|---|
| gadget and kernel | both required | optional; a kernel only with a gadget |
| `distribution` | not allowed | required (an os-release ID) |
| snaps marked `classic: true` | not allowed | allowed, for app snaps |

`idiot assertion new model` never asks which kind it is writing. Given
`--set classic=true`, it still adds `pc` and `pc-kernel` and never asks
for `distribution`, so `snap sign` rejects the draft.

`idiot assertion check` passes four shapes that snapd rejects:
- a classic model without `distribution`;
- an Ubuntu Core model with `distribution`;
- a classic model with a kernel but no gadget;
- an old-style classic model (no `snaps` list) with `kernel` or `base`.

Validation sets have a similar gap in their components:
- a component can't have both a revision and `presence: invalid`;
- a component has a revision exactly when its snap has one.

None of these component rules is checked.

## What Changes

- **The kind of model comes first.**
  - `idiot assertion new model` asks "Ubuntu Core or classic?" on a terminal, defaulting to Ubuntu Core. `--classic` and `--set classic=true` answer it, for scripts.
  - `edit` keeps the kind the draft has.
  - `new --from` and `--from-store` take the kind from the template.
- **Classic models get their own prompts:**
  - `distribution`, defaulting to `ubuntu`;
  - an optional gadget, with no default;
  - a kernel, asked for only when a gadget was given;
  - `snapd` and the base are proposed as for Ubuntu Core.

  Snaps the store releases with classic confinement get `classic: true`
  in a classic model. In an Ubuntu Core model, the writer warns that such
  a snap can't be installed.
- **Ubuntu Core prompts are unchanged.**
- **The offline check gains the missing model rules:**
  - `distribution` is required in a classic model with `snaps` (and must be an os-release ID: lowercase letters, digits, `.`, `_` and `-`), and not allowed otherwise;
  - a classic model with `snaps` may list a kernel only together with a gadget;
  - an old-style classic model may not have `kernel` or `base`.
- **The offline check gains the component rules for validation sets.**
- **idiot-store's `info` request** asks for each channel's confinement too, and its output gains a confinement column. `store channels` and `store browse` ignore the extra column.
- Nothing is **BREAKING**.

**stdout and the eval file:** drafts written to stdout (`-o -`) can now
hold `distribution` and per-snap `classic`. The info lines idiot-store
prints for `store channels` and the writer gain an eighth column, which
`store channels --tsv` does not show. Nothing is queued in the eval file.

There are no new runtime dependencies, and the snap stages nothing new.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `assertion-writing`:
  - a new "Classic models" requirement;
  - "Model essentials" applies to Ubuntu Core models;
  - "Checking an assertion" lists the new model and validation-set rules.

## Impact

- **Writer:** `cmd/assertion/new` (`--classic`) and `share/assertion.sh` (`as_model_headers`, `as_model_snaps`, `as_model_snap`).
- **Offline check:** `tools/idiot-helper/src/draft.rs` (`check_model`, `check_validation_set`), with tests.
- **idiot-store:** `tools/idiot-store/src/store/query.rs` (`INFO_FIELDS`) and `tools/idiot-store/src/cmd/query.rs` (`info_line`).
- **Docs:** README ("Writing assertions"), `skills/using-idiot/references/commands.md`.
- **Tests:** `tests/bare-host.d/assertion-writer`.
