# Proposal

## Why

Getting a store snap into a VM takes three steps today: `snap download`
on the host (which uses snapd's own login, not idiot's credential, and the
host's architecture and store unless told otherwise), then `idiot vm
install` with the `.snap` and `.assert` it left. For a brand store, a
private snap or an arm64 VM on an amd64 laptop, getting the first step
right is most of the work. idiot already knows the active store, the
active credential and the VM's architecture, and `idiot-store` already
reads channel maps and download URLs (`store check`, `store channels`).

## What Changes

- **`idiot vm install` takes store snap names** as well as `.snap` files:
  `idiot vm install hello-world`, `idiot vm install pc-kernel=24/edge`,
  or `idiot vm install --channel beta app ./my-gadget.snap`. Arguments
  are read like this:
  - one ending in `.snap` is a file, as today;
  - `<name>=<channel>` is a store snap from that channel;
  - any other argument is a store snap name.
- **`--channel <channel>`** sets the channel for store snaps without
  `=<channel>` (default `stable`). It never applies to files.
- **Store snaps are downloaded on the host** for the VM's architecture (its
  recorded `arch`, not `UBUNTU_STORE_ARCH`), from the active store
  (`UBUNTU_STORE_ID`, else the global store). The download uses the active
  credential, or the saved one `--cred <name>` names, and is anonymous when
  there is none (saying so). The credential never enters the VM.
- **Downloads are verified and cached.** Each snap is checked against the
  store's size and sha3-384 before use, and kept in
  `IDIOT_CACHE_DIR/snaps/` (mode 0700). A later install of the same
  revision reuses the cached file after checking its digest again.
- **Store snaps come with their assertions**: snap-declaration,
  snap-revision, and the account-keys and accounts they need. They are
  fetched with the assertion fetch from `store-assertions` and written
  next to the snap as `<name>_<rev>.assert`, the set `snap download`
  writes. The existing install path then acknowledges them and installs
  without `--dangerous`, in the same single snapd change as any files.
- **Mixed with an unasserted file**, the existing rule stays: one change,
  all `--dangerous`. The command names the store snaps that lose their
  store revision because of it.
- **Failures are reported before anything is copied**: an invalid name or
  channel, a store that doesn't show the snap, no release for the VM's
  architecture in that channel (listing the channels that have one), or
  the store refusing the download. Each points at `idiot store check`
  with the same arch and channel.
- **`idiot-store download`** is a new helper subcommand. It resolves the
  release, downloads it (sending the credential to the store only, never
  to the CDN it redirects to), verifies it, fetches its assertions and
  prints one result line for sh.

Out of scope:
- `--revision` (the channel map only lists released revisions);
- a standalone `idiot store download` (the helper subcommand makes it
  easy later);
- pruning the download cache;
- fetching arbitrary assertions, which is `store-assertions`.

Output: stdout and the eval file are unchanged. `vm install` still writes
only to stderr, now with download progress too. `idiot-store download`
prints its result on stdout to the calling sh only. It is still a host
command (`cmd/vm` is already one). No new runtime dependency: the snap
already stages `idiot-store`. The `sha3` crate becomes a build-time
dependency of idiot-store (idiot-helper already uses it).

Uses `store-assertions` (implemented and archived) for the assertion fetch.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `vm`: "Installing snaps into a VM" accepts store snap names and
  `--channel`/`--cred`. A new requirement covers downloading from the
  store for the VM: architecture, credential, verification, cache,
  assertions and failures.

## Impact

- `cmd/vm` (`do_install`: argument parsing, the download step before
  copying, help text). It now includes `creds.sh` and `stores.sh`.
- `tools/idiot-store`: `download` subcommand (`src/cmd/`, `src/store/`),
  `sha3` dependency, fixtures and tests. It uses `store-assertions`'
  library function for the assertions. `normalise_channel` learns
  snapd's `<risk>/<branch>` form, which also fixes `idiot store check
  --channel edge/fix`.
- `tests/bare-host.d/`: an anonymous download of a public snap (digest,
  assertion stream verified with `idiot inspect assertion`, cache reuse,
  a corrupted cache entry replaced), and `vm install` argument errors.
- README (VM section), `skills/using-idiot/references/commands.md`, and
  the vm spec.
