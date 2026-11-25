# Design

## Context

Each credential `<name>` in `IDIOT_AUTH_DIR` can have a pairing record
`.store.<name>` holding one line, `store: <id>` (empty for the global
store). `auth add` and `auth bind` write it (`record_cred_store`),
`auth remove` deletes it, `auth import` writes none. `cred_store_id` reads
it and fails when there's no record. `warn_store_mismatch` (called by
`status`, `auth login`, `store use`/`unset`, `profile use`/`save`) is
silent when `cred_store_id` fails, and otherwise compares the result with
the active store. Nothing displays a credential's pairing apart from that
warning.

## Goals / Non-Goals

**Goals:**
- A user can mark a credential for any store, at `auth add` or later with
  `auth bind`, and undo it with plain `auth bind`.
- The mismatch warning offers that as a fix.
- `auth list` shows every credential's store, so the pairings are visible
  without waiting for a warning.

**Non-Goals:**
- Pairing with a list of stores. A credential belongs to one store or to
  all of them; nobody has asked for a subset.
- Changing the default pairing at `auth add`.
- Showing the pairing in `idiot status`, `auth status` or `idiot doctor`.
  The status line is already long, and doctor doesn't warn about pairings.
  `auth list` is the one place to look.
- Working out from the store which stores a credential works in.

## Decisions

**The any-store marker is the record `store: *`.** `cred_store_id`
returns `*`, and the callers check for it. `*` can't be a real store ID,
which is alphanumeric. A separate line or file was the alternative, but one
record with one key keeps `auth remove`, the permissions and the
"hidden from `auth list`" rule exactly as they are.

**An explicit marker rather than deleting the record.** Today "marked for
any store" and "never paired" behave the same, so deleting `.store.<name>`
would also work. The marker records the user's decision. An unpaired
credential's store is unknown (imported, or saved before pairing existed),
and idiot may later learn it, for example from the store, as `auth status`
already does for an older credential's expiry. A deliberate "any store"
must never be overwritten that way. `auth list` shows both as `any store`
because that is what they do now.

**`--any` is a flag on the existing commands**, not a new `auth unbind`.
`auth bind --any [<name>]` uses the same name rules as `auth bind`: the
active credential by default, failing when there's none. `auth add --any`
writes `store: *` instead of the active store's ID, and its confirmation
says `for any store`. Plain `auth bind` is the undo, as it already re-pairs.

**Labels.** The warning keeps `store_label` (`the global store`,
`store 'staging'`, `store <id>`). `auth list` needs a short form: the saved
store's name, else the ID, `global`, or `any store`. That is a small
helper in `creds.sh` (next to `cred_store_id`, using `store_name_for_id`
from `stores.sh`, which `auth list` then includes). The column comes after
the expiry, dimmed, so the expiry stays aligned where it is now.

**The warning's third fix** goes on the same hint line:
`switch: <fix> · re-pair: idiot auth bind <name> · any store: idiot auth bind --any <name>`.

**Confinement and output are unchanged.** `auth bind` and `auth list` are
confined: they only touch idiot's state. `auth add` stays a host command.
Everything still goes to stderr, nothing is added to the eval file, and the
record never holds credential contents. It is written with the auth
directory's private permissions, like the existing record.

## Risks / Trade-offs

- **Downgrades.** An older idiot reads `store: *` as an unknown store ID
  and warns "belongs to store *". It is harmless, and running
  `idiot auth bind` there fixes it.
- **Hiding a real mismatch.** A credential marked for any store that in
  fact works in only one store won't be warned about. The user opted in,
  `auth list` shows it, and plain `auth bind` undoes it.
- **A longer warning.** The hint line grows to three fixes. It is still a
  single dimmed line, and the fix the user wants is copyable from it.
