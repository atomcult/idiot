# Proposal

## Why

Ubuntu Core work keeps needing assertions that only the store holds: the
`snap-declaration` and `snap-revision` that make a side-loaded snap
asserted, a brand's `account` and `account-key` to check a model, a
`validation-set`, a dedicated store's `store` assertion. Today that means
`snap known --remote` (which needs snapd, and a snap-id or a sha3-384 the
user has to dig out first) or `snap download` (which downloads the whole
snap to get its `.assert`). idiot already talks to the store and already
summarises assertions offline (`idiot inspect assertion`); joining the two
gives one command that fetches what you name, by the names you know. It is
also the fetch that `vm-install-from-store` and `assertion-writer` need.

## What Changes

- **`idiot store assertion <type> [<key>...]`** (a host command: it uses the
  network) fetches an assertion from the store's assertions service, as the
  active credential sees it (anonymously without one; `--cred <name>` for
  one call), and prints it on stdout through `idiot inspect assertion`'s
  summariser, signature check included.
  - Shortcuts by what the user knows: `account <account-id>`,
    `account-key <sha3-384>`, `store [<store-id>]` (default the active
    store), `model <brand-id>/<model>`, `snap-declaration <snap-name>`
    (the snap-id looked up in the active store), `snap-revision <snap-name>
    <revision>` or `snap-revision <sha3-384>` (hex or base64url),
    `validation-set <account-id>/<name>[=<sequence>]` (latest sequence by
    default).
  - Any other type the store serves (`serial`, `system-user`,
    `snap-developer`, `snap-build`, `validation`, `repair`, `preseed`,
    `confdb-schema`, `snap-resource-revision`, `snap-resource-pair`, …) by
    its primary-key headers as `header=value` words, as `snap known`
    takes them, with `series` defaulting to `16`.
  - `--chain` adds the prerequisites snapd needs to accept it (signing
    account-keys up the chain, the accounts and the declaration it refers
    to), prerequisites first and each once, leaving out what snapd trusts
    built in. The result verifies offline and can be given to `snap ack`.
  - `--raw` prints the assertion stream itself on stdout instead of the
    summary; `-o <file>` writes it to a file and prints nothing on stdout.
  - `--no-verify` passes through to the summariser.
- **idiot-store `assertion` subcommand**: `idiot-store assertion [--file F]
  [--store ID] [--arch A] [--chain] <type> <header=value>...` prints the raw
  stream on stdout and nothing else. It resolves `snap-name=` (to a snap-id)
  and `snap-name=` + `snap-revision=` (to a sha3-384) itself, and keeps one
  refreshed credential across every request of a chain. A new exit status 4
  means "not found" (or not shown to this credential), so callers tell
  absent from a store error. `vm-install-from-store` and
  `assertion-writer` call this subcommand directly.

Output: `idiot store assertion` writes the summary (or, with `--raw`, the
raw stream) to stdout, and messages to stderr. Nothing is queued in the
eval file. No other command's output changes. There are no new runtime
dependencies and nothing new for the snap to stage: the calls are in
idiot-store and the summary in idiot-helper, both already built, and no new
crate is needed.

Out of scope: changing `idiot inspect assertion --remote` (it keeps using
`snap known --remote`; see design.md's open questions), adding fetched
assertions to snapd's database (`snap ack` stays the user's step), hashing a
local `.snap` to find its revision, fetching by headers other than the
primary key (e.g. all account-keys of an account, which the store's API
doesn't offer), and staging stores.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `store`: adds fetching assertions from the store, by shortcut or primary
  key, summarised or raw, with their prerequisite chain. The credential
  rules are stated in the new requirement itself rather than by modifying
  "Credential for store queries", which other in-flight changes
  (`store-explorer`, `store-members`) may also touch.

## Impact

- `tools/idiot-store`: an `assertion` subcommand; a table of assertion types
  and their primary keys (from snapd's `asserts`); `GET
  /v2/assertions/{type}/{key...}` on api.snapcraft.io with `Accept:
  application/x.ubuntu.assertion` and snapd's `max-format`s; snap-id and
  sha3-384 lookups through `/v2/snaps/info` (already used by `info`) and,
  for a revision not in the channel map, the `/v2/snaps/refresh` download
  action; a reader for an assertion's top-level headers to follow the
  chain; exit status 4.
- `cmd/store/assertion`, `cmd/store/help`; it pipes into
  `cmd/inspect/assertion` (unchanged).
- Tests: Rust unit tests on recorded public responses; bare-host checks for
  usage, unknown types, missing keys, `--cred` with an unknown name, and an
  anonymous fetch of a public Canonical assertion when the container can
  reach the store.
- README (Stores section), `skills/using-idiot/references/commands.md`, the
  `store` spec.
