# Design

## Context

- **The writer.** `share/assertion.sh` builds a model in two steps. `as_model_headers` asks for the headers. `as_model_snaps` then adds the `--snap` snaps, proposes the essential snaps from `as_essentials`, and offers more.
- **Essentials.** `as_essentials` always gives a gadget and a kernel role. `as_model_snaps` loops until each role has an answer, and dies without a terminal when there is no default.
- **Snap lookups.** `as_model_snap` looks each snap up with idiot-store's `info`, whose `channel` lines are `channel name arch revision version released type`. `Channel.confinement` is parsed, but the info request doesn't ask for it (`INFO_FIELDS`), so the store leaves it out, and it isn't printed.
- **The offline check.** `check_model` in `tools/idiot-helper/src/draft.rs`:
  - skips the gadget and kernel requirement for a classic model;
  - knows nothing about `distribution`;
  - treats every model without `snaps` as old-style, needing `kernel` and `gadget` unless it is classic.

## Goals / Non-Goals

**Goals:**
- Write a classic model `snap sign` accepts, from a terminal or a script.
- Make `check` reject every shape that snapd's `assembleModel` rejects for these headers.

**Non-Goals:**
- Writing old-style models without `snaps`. They are still checked, but the writer only writes the extended form.
- Writing components into validation sets or models. They are checked, and kept on edit, as before.
- Moving component revisions along on `edit --refresh`. That is a separate change, if it is wanted.

## Decisions

### The kind of model is a header, decided early
`as_model_headers` settles `classic` right after `architecture`:
1. `--classic` is `--set classic=true`. Both mark it as given.
2. `edit`, and a template, already have the header (`as_has classic`).
3. Otherwise, on a terminal, `as_ask "Ubuntu Core or classic? [core/classic]" core`.
4. Without a terminal, it is Ubuntu Core.

A classic model then gets `as_header distribution "Distribution (os-release ID)" ubuntu 1`, checked against `^[a-z0-9._-]+$` before going on.

Everything after this branches on one variable, `as_classic`, set from the header, so `edit` follows the same paths as `new`.

*Alternative:* a separate `assertion new classic-model` type. But snapd has one model type, and `edit` and `check` would need it too.

### Essentials by kind
`as_essentials` takes the kind. For classic it prints the `snapd` and base lines, then `gadget?` and `kernel?` roles with no default. In `as_model_snaps`, a role ending in `?` is optional:
- An empty answer skips it.
- Without a terminal, it is skipped unless `--snap` gave one.
- `kernel?` is skipped when no snap of type gadget is listed by then.

The base stays required. snapd requires `base` in every model with `snaps`, classic or not.

### Confinement from idiot-store's info line
`INFO_FIELDS` gains `confinement`, which the store then returns per channel-map entry (checked against the store: `code` gives `classic`). `info_line` gains an eighth column with it (`strict`, `classic` or `devmode`), from the `Channel.confinement` it already parses. `as_model_snap` reads column 8:
- **In a classic model**, a `classic`-confined app snap gets `as_put f classic true`.
- **In an Ubuntu Core model**, it warns: `<snap> has classic confinement in <channel>: an Ubuntu Core device can't install it`.

`store channels` reads the lines by column, so one more column changes nothing there. Its TSV output is built separately.

### Check rules, as snapd orders them
`check_model` follows `assembleModel`'s order:
1. Without `snaps`: `grade` and `storage-safety` are refused (as now). A classic model additionally refuses `kernel` and `base`. A non-classic model needs `architecture`, `gadget` and `kernel` (as now).
2. `distribution`:
   - classic with `snaps`: it must be there and match `^[a-z0-9._-]*$`, non-empty;
   - otherwise, if present: an error, "only for a classic model with snaps".
3. With `snaps`:
   - not classic: one gadget and one kernel (as now);
   - classic: a kernel with no gadget is an error, "a classic model can list a kernel only with a gadget".

`check_validation_set` gains a components pass that follows `checkValidationSetComponent`:
- a component's `presence` is `required`, `optional` or `invalid`, or it is given in the string shortcut;
- an invalid component may not have a revision;
- a revision is required when the snap has one, unless the component is invalid;
- a revision is not allowed when the snap has none.

Errors name `snaps[<snap>].components.<name>`.

## Risks / Trade-offs

- **[Risk]** snapd changes these rules. → Each rule has a test built from snapd's error text, and the README says the check follows snapd's master.
- **[Trade-off]** A classic model's kernel prompt depends on the gadget answer. That makes one more branch in the terminal flow, which the bare-host writer check covers without a terminal: `--classic` with no gadget, and with `--snap pc=24/stable --snap pc-kernel=24/stable`.
- **[Trade-off]** The confinement used is the one in the chosen channel and architecture. A snap that is strict in one channel and classic in another is marked from the channel the model names.
