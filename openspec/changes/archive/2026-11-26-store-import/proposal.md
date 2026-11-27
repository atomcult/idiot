# Proposal

## Why

Saving a brand store means finding its ID on the dashboard and typing
`idiot store add <name> <id>` for each one. The account already knows its
stores: `GET /dev/api/account`, which `idiot store roles` uses, returns
each store's ID, display name and the account's roles. `store roles` shows
the ones that aren't saved yet with a `-`, but there is no way to save them
from there.

## What Changes

- **`idiot store import [--all] [--cred <name>]`** saves the stores the
  account belongs to as saved stores. It skips:
  - the global store (ID `ubuntu`), which needs no saved store;
  - stores whose ID is already saved, under any name.
- **On a terminal** it opens an fzf multi-select picker (display name, ID,
  roles), then asks for a name for each picked store. The default is made
  from the display name: lowercased, each run of characters other than
  `a-z` and `0-9` replaced by `-`, with leading and trailing `-` removed.
  Enter accepts the default.
- **`--all`** imports every remaining store under its default name, with
  no picker or prompts. Without a terminal, `--all` is required.
- **Names follow `store add`'s rules and never overwrite.** A default name
  that is already saved for another store gets `-` and the first 4
  characters of the ID. A name typed at the prompt that is taken is asked
  for again. Each saved store is reported on stderr, and a summary line
  counts what was saved and skipped.
- **`--cred <name>`** uses another saved credential for the call, as with
  the other store queries.

Out of scope:
- pairing credentials with the imported stores (`idiot auth bind` does
  that);
- removing saved stores the account no longer belongs to.

Output: everything goes to stderr, and nothing goes to stdout or the eval
file. It is a host command (it uses the network) with no new dependencies:
it uses idiot-store's existing `account` query and fzf.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `store`: adds importing the account's stores as saved stores.

## Impact

- `cmd/store/import` (new), `cmd/store/help`.
- `share/stores.sh`: the default-name helper (and saving a store, shared
  with `store add`).
- `tests/bare-host.d/store`: usage, a non-terminal run without `--all`,
  and name generation (offline).
- README (Checking a store section), the using-idiot skill reference, and
  the store spec.
