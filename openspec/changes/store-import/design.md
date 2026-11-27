# Design

## Context

`idiot-store account` (from add-store-commands) prints `store <id>
<display name> <roles>` lines for the account behind a credential. The
global store appears there with ID `ubuntu`. Saved stores are files under
`IDIOT_STORES_DIR` named after the store, holding the bare ID (no
newline). `idiot store add` validates the name (`validate_store_name`) and
refuses an existing one (`require_no_store`). `store_name_for_id` finds a
saved store by ID. `query_cred` (creds.sh) resolves the credential for a
query: the active one, or `--cred`.

## Goals / Non-Goals

**Goals:**
- Save the account's stores in one step, with readable names.
- Never overwrite or rename anything already saved.
- Work unattended with `--all`.

**Non-Goals:**
- Pairing credentials with stores, or removing saved stores the account
  has left.
- Renaming saved stores (`store remove` + `store add` already do it).

## Decisions

**A host command, no new helper call.** It runs `idiot-store account` as
`store roles` does, through `query_cred`, and only adds the sh around it.

**Default names in sh.** `store_default_name "<display name>"` in
stores.sh: `tr 'A-Z' 'a-z'`, `sed 's/[^a-z0-9][^a-z0-9]*/-/g; s/^-//;
s/-$//'`. Non-ASCII letters become `-`, which is acceptable for a default
the user can edit. An empty result (a name made only of symbols) falls back
to `store-` and the ID's first 8 characters. A collision with a saved name
adds `-` and the ID's first 4 characters, and failing that, the whole ID.

**Saving goes through one helper.** `store_save <name> <id>` in stores.sh
(validate, refuse an existing name, write) is used by both `store add` and
`store import`, so the rules can't drift apart.

**The picker and prompts.** fzf `--multi`, with tab-separated lines
`display name / ID / roles` and the ID as the key. Each prompt reads from
`/dev/tty`, showing the default in brackets. Enter accepts it; an invalid
or taken name is reported and asked for again. Cancelling the picker
saves nothing and exits 0.

**Terminal check first.** Without `--all`, stdin and stderr must be
terminals, checked before any network call, so a script that forgets
`--all` fails fast instead of hanging or saving nothing quietly.

## Risks / Trade-offs

- **Long display names make long names.** The prompt shows them for
  editing, and `--all` users can rename afterwards.
- **Two imports at once could race on a name.** `store_save` refuses an
  existing file, so the second one fails for that store rather than
  overwriting it. That's acceptable for an interactive tool.
