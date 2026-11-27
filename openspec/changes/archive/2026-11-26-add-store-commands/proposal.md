# Proposal

## Why

When a snap won't download from a store, the store returns a 404 for half a
dozen different reasons:
- the account isn't a member of the store;
- the snap isn't in that store;
- the snap isn't released for the target architecture or channel;
- the credential lacks a permission;
- the credential is one snapd can't read (the padding bug just fixed);
- or simply the wrong store is active.

Diagnosing the padding bug took a long investigation because nothing told
these apart. idiot also offers no view of what a store contains, or of which
role the user has in it, though it already manages credentials and stores.

The Snap Store APIs that snapcraft uses, and the dashboard's documented
store API, answer these questions. Most of them work with any credential;
listing a brand store's snaps needs a store admin.

## What Changes

New `idiot store` subcommands (all host commands: they use the network):

- **`idiot store check <snap>`** says whether the active credential can
  download `<snap>` from the active store for the target architecture
  (`UBUNTU_STORE_ARCH`, else the host's) and channel (`--channel`, default
  `stable`). When it can't, it names the cause. It looks the snap up as
  snapd does, tries the download without and with the credential (without
  downloading the snap), reads the user's role in the store, and checks the
  credential offline: expiry, `package_access`, padding, and its store
  pairing. Exit 0 when the download works, 1 otherwise.
- **`idiot store channels <snap>`** prints the snap's channel map in the
  active store: track/risk, then the revision and version per architecture,
  with the target architecture highlighted. It is named `channels` because
  `store status` already shows the active store.
- **`idiot store roles`** lists the stores the account belongs to and its
  roles in each (`admin`, `review`, `view`, `access`), marking the active
  store.
- **`idiot store snaps`** lists the snaps in the active store: all of them
  for a store admin using a credential with `store_admin`, otherwise only
  the snaps the account publishes or collaborates on there, saying that the
  list is partial and why. On a terminal, with `--pick`, it opens an fzf
  picker and prints the chosen name.
- **`--cred <name>`** on every new subcommand uses that saved credential for
  the call instead of the active one, without changing the shell. When a
  call needs `store_admin` and the credential lacks it, idiot says so and
  names any saved credential that has it.

Output: `store channels`, `store roles` and `store snaps` print their data
on stdout (tab-separated with `--tsv`, aligned otherwise), with messages on
stderr. `store check` prints its report on stderr. Nothing new goes into the
eval file. There are no new runtime dependencies (the store calls are in
idiot-store, which the snap already builds) and nothing new to stage.

Out of scope: `store members`, `idiot vm install <snap-name>`, image-build
pre-flight checks, and limiting a credential to particular stores
(`store_ids`, whose request field isn't documented).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `store`: adds checking a snap's availability, its channel map, the
  account's store roles, and a store's snap list.

## Impact

- `tools/idiot-store`:
  - new read-only calls: `GET /dev/api/account`,
    `GET /api/v2/stores/{id}/snaps` (dashboard), `GET /v2/snaps/info/{name}`
    (api.snapcraft.io, with `Snap-Device-Store`), and a download probe that
    reads only the response status;
  - reading a credential's permissions offline from its `acl` caveat;
  - subcommands that print results as lines for sh to format, never the
    credential.
- `cmd/store/`: `check`, `channels`, `roles`, `snaps`, help, and the group
  help. `share/creds.sh`: a helper that finds saved credentials with a
  permission.
- Tests: Rust unit tests on recorded response shapes, and bare-host checks
  for usage, errors and offline paths (no network credentials in the
  container).
- README (a Stores section), the using-idiot skill reference, and the
  `store` spec.
