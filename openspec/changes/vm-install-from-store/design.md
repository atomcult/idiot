# Design

## Context

`do_install` in `cmd/vm` takes `.snap` files only. It checks each one
exists and marks the set unasserted unless every file has a
`<file>.assert` beside it. It then resolves the running VM
(`_vm_target`: Launchpad user, ssh port), checks passwordless sudo, and
scp's the files and their `.assert`s into a guest temp dir. It runs `snap
ack` for each assertion file when the whole set is asserted, then `snap
install --no-wait` once (`--dangerous` when any snap lacks assertions),
and follows the change through reboots. The VM's architecture is recorded
in `vm.conf` (`arch`, set when the VM is created, default the host's).

`idiot-store` already does most of the lookup. `query::info` reads the v2
channel map with the store header and the credential, refreshing the
discharge on `macaroon-needs-refresh`. `parse_info` gives each
channel/arch entry's revision, version and download URL. `download_status`
sends an authorised GET without following redirects. sh resolves the
credential with `query_cred` (creds.sh) and passes `--file` for `--cred`.
`store check` already turns "not shown / not released / refused" into
advice.

`store-assertions` (change #2) defines how idiot-store fetches an
assertion by type and primary key. This change consumes it and doesn't
redesign it.

## Goals / Non-Goals

**Goals:**
- `idiot vm install <name>[=<channel>]` gives the same result as `snap
  download` plus `idiot vm install <file>`, using idiot's credential and
  store and the VM's architecture.
- Never send the credential to the VM or the CDN, never put it in argv,
  never print it.
- Never install a byte that doesn't match the store's digest.
- Keep the existing install path (one change, ack, reboot following)
  exactly as it is for files.

**Non-Goals:**
- Installing a specific `--revision`, or following snapd's tracking
  semantics beyond what the channel map reports.
- A standalone `idiot store download` command.
- Cache pruning or size limits.
- Generic assertion fetching or display (`store-assertions`).

## Decisions

**The download lives in idiot-store, as `idiot-store download`.** sh
can't do it safely. `curl -H "Authorization: …"` would put the
credential in argv, and sha3-384 isn't in coreutils (`openssl dgst
-sha3-384` isn't guaranteed on the host and isn't in the snap). idiot-store
is already the network client that holds credentials, and already has the
info lookup, the auth header and the refresh logic. idiot-helper stays
offline and credential-free. The `sha3` crate is added to idiot-store
(the same crate and version line idiot-helper uses). It is a build
dependency only: the snap already builds and stages idiot-store.

**Interface.**

```
idiot-store download [--file F] [--store ID] --arch A [--channel C]
                     [--no-assertions] [--progress] --dir DIR <snap>
```

It prints tab-separated lines on stdout for sh, in the style of `probe`
and `store-snaps`:
- `snap <path> <name> <revision> <version> downloaded|cached`: success.
  The assertions are at `<path>` with `.snap` replaced by `.assert`
  (absent with `--no-assertions`).
- `unknown`: the store doesn't show the snap (info 404).
- `unreleased <channel> <arch>`, then one `available <channel>
  <revision>` line per channel that has a release for `<arch>`.
- `refused <http-status>`: the download URL refused (401/403/404).

These outcomes exit 0, so sh can phrase them. The usual exits remain: 1
local/usage, 2 credential rejected, 3 network or store error. A digest
or size mismatch is exit 3 ("sha3-384 mismatch for <name> revision
<rev>"), with the partial file removed. `--progress` draws one
self-overwriting line on stderr (`<name> <rev>: 34.1/120.0 MB`). sh
passes it only when stderr is a terminal. Progress is the one piece of UX
in the helper, because only the process reading the body knows how far it
has got.

**Resolution.** `download` reuses `info()` and `parse_info`, picking the
entry whose channel equals the normalised channel and whose arch equals
`--arch`, as `probe` does. Two extra fields are read: the top-level
`snap-id`, and `download.sha3-384` and `download.size` (both already in
the `download` object `INFO_FIELDS` requests; confirmed live). Today
`normalise_channel` only prefixes `latest/` when there is no `/`. It
becomes snapd's parse: a first segment that is a risk (`stable`,
`candidate`, `beta`, `edge`) means `latest/<risk>[/<branch>]`. So
`edge/fix` becomes `latest/edge/fix`, not `edge/fix`. `store check` uses
the same function and gains the fix.

**The redirect is followed by hand, without the credential.** The client
keeps `Policy::none()`. `download` sends the authorised GET to the store's
download URL, refreshing once on a 401 asking for it, as `download_status`
does. On a 3xx it takes `Location` and fetches it with a fresh request
carrying no `Authorization` header. The CDN URL holds its own token
(confirmed live for a public snap: 302 to
`*.cdn.snapcraftcontent.com/...?token=...`). A 2xx straight from the store
is read as the body. That request uses a client with no overall timeout
(reqwest's blocking default of 30 s would cut off a kernel snap) but with
a connect timeout and a read timeout.

**Streaming, verification, atomic placement.** The body is copied in
64 KiB chunks into `DIR/.<name>_<rev>.snap.<pid>.partial`, created with
mode 0600, and fed to SHA3-384 as it goes. When it finishes, the size and
digest must match `download.size` and `download.sha3-384` (hex in the
info response). On a match the file is renamed to `DIR/<name>_<rev>.snap`.
On a mismatch, an error or an interrupt it is removed. `DIR` is created
with mode 0700 if missing.

**Cache.** sh passes `--dir "${IDIOT_CACHE_DIR}/snaps"`. Before
downloading, an existing `<name>_<rev>.snap` is hashed again. If it
matches it is reported `cached`; if not it is deleted and downloaded
again. The store is still asked which revision the channel points at on
every run, so the cache never serves a stale release. Keys are name and
revision: revisions are per snap, and the digest check catches anything
else. Private snaps land in the cache, hence 0700/0600. Nothing prunes it
yet (see Open questions).

**Assertions come from `store-assertions`, in-process.** That change
defines `idiot-store assertion [--file] [--store] [--chain] <type>
<header>=<value>...`. Its `--chain` fetches the prerequisites depth-first,
each printed after everything it needs, deduplicated, and stops at what
snapd trusts built in. It sends `Snap-Device-Store`, snapd's `max-format`
values and the credential, and exits 4 for not found. All this change
needs from it is the function behind that subcommand, called inside the
same `download` run (one process, so a refreshed discharge serves the
snap and its assertions):

> chain(auth, store, "snap-revision", snap-sha3-384=<digest>) → the
> stream as text, or not-found

`download` passes the digest it computed itself, unpadded base64url
(store-assertions also accepts hex). Confirmed live: the hex value 404s as
a path key and the base64url one returns the assertion. The snap-revision's
prerequisites take in the snap-declaration (series 16), the developer's
and publisher's accounts, and the store key's account-key. That is the
set `snap download` writes, though not necessarily in its order. A
prerequisites-first stream is what `snap ack` needs, so the spec says
only "each after the assertions it needs". `download` then checks that the snap-revision's `snap-id`,
`snap-revision` and `snap-size` agree with the info entry, and writes the
stream to `<name>_<rev>.assert.<pid>.partial`, then renames it next to the
snap.

If this change is implemented before `store-assertions`, task 1.3 writes
that function as a minimal private walk over the same prerequisite rules
(snap-revision → snap-declaration and developer account; snap-declaration
→ publisher account; each → its signing account-key → that key's
account; stop at Canonical's account and root key). It is marked for
`store-assertions` to replace, so there are never two copies. This change
adds no assertion subcommand.

The guest's snapd verifies the signatures when they are acknowledged.
idiot doesn't verify them again. Assertions are fetched fresh on every
run, even for a cached snap, because a snap-declaration can gain
revisions. A snap that is `cached` but whose assertion fetch fails is an
error, not a silent `--dangerous`. `--no-assertions` (sh passes it for
`--dangerous`) skips the fetch.

**Argument parsing in `do_install`.** Each argument is classified in this
order:
1. `*.snap` is a file (existing checks).
2. `*=*` is a store snap with a channel.
3. Anything containing `/` is an error: not a `.snap` file and not a snap
   name.
4. Anything else is a store snap on `--channel` (default `stable`).

Names follow snapd's rule (2–40 of `a-z0-9-`, at least one letter, no
leading, trailing or doubled `-`), checked with `case` patterns. Channels
are 1–3 `/`-separated non-empty segments of `[a-z0-9._-]`. The help and
usage line say `<snap>...` with this explained. `--cred` goes through
`query_cred` at parse time, so an unknown credential fails before any
network or ssh. `--channel` and `--cred` are rejected when no store snap
is given: they would be silently ignored otherwise.

**Order of work.** Parse and validate, then `_vm_target` and the sudo
check (cheap, and they fail fast before a long download), then read `arch`
from `vm.conf` (falling back to the host's for a VM without one). Then
fetch each store snap in argument order with `_store_fetch`, a private
helper in `cmd/vm` that runs `store_helper` with `--file`/`--store`/
`--arch`/`--channel`, maps the outcome lines to messages, and appends the
cached path to `files`. Then the existing unasserted check, copy, ack,
install and follow, unchanged. A store snap's guest name is known (from
the `snap` line), so `snap info` in the guest isn't needed for it. Messages
come from `store_label` for the store, and say `anonymously` when
`query_cred` found nothing. The `store check` suggestion includes `--arch`
and `--channel`, and `--cred` when one was given.

**Mixed asserted and unasserted keeps today's rule.** snapd ignores
assertions on a `--dangerous` sideload: `readSideInfo` only derives side
info from assertions when `--dangerous` isn't set, so a store snap in a
dangerous change installs as `x<n>`. Splitting the install into two
changes would break the one-change guarantee that a kernel plus gadget
pair relies on. So the rule stays: one change, `--dangerous` for all,
said out loud. The message now names the store snaps that lose their
revision and suggests installing them separately to keep it.

**Why not let the guest download it?** The VM's snapd has no credential
for the user's store account, may be on another store (its model's), and
may have no network. Downloading on the host keeps the credential on the
host and reuses the cache across VMs and resets.

**What was confirmed live (unauthenticated GETs only):**
- `GET api.snapcraft.io/v2/snaps/info/hello-world?fields=download,revision,version`
  with `Snap-Device-Series: 16` returns top-level `snap-id` and, per
  channel-map entry, `download.{sha3-384 (hex), size, url, deltas}`.
- The download URL answers 302 to a CDN URL carrying a `token` query
  parameter.
- `GET /v2/assertions/snap-revision/<base64url sha3-384>`,
  `/v2/assertions/snap-declaration/16/<snap-id>`,
  `/v2/assertions/account-key/<key-id>` and `/v2/assertions/account/<id>`
  with `Accept: application/x.ubuntu.assertion` return the assertion
  text. A miss is a 404 with a JSON `error-list` (`not-found`).

**From docs or assumption, to check with the user's account:**
- An authorised download of a private brand-store snap redirects the same
  way, and the CDN URL needs no credential.
- The download URL in an info response requested with
  `Snap-Device-Store` needs no store header of its own.
- Assertions for private snaps are served (anonymously or with the
  credential).
- The channel map reflects the store's risk fallback for closed channels,
  as snapd would see it.

## Risks / Trade-offs

- **Cache growth.** Kernel and gadget snaps are large, and every new
  revision adds a file. Mitigation: the path is printed in the download
  message, and pruning is an open question rather than a silent policy.
- **Large downloads with no resume.** An interrupted download starts
  again. Acceptable for a dev loop; the partial file is always removed.
- **Order with `store-assertions`.** If this lands first it carries a
  private fetch function that change #2 must take over. The tasks say so,
  to avoid two copies.
- **Ambiguous arguments.** A snap called `foo` and a file `./foo` without
  `.snap` used to fail ("not a .snap file"). `foo` now means the store,
  and `./foo` still fails. A typo in a file name without `./` now reaches
  the store and fails as `unknown`, which suggests `store check`. That is
  clear enough.
- **Mixed installs losing revisions.** Kept on purpose (above). The
  message makes it visible.

## Open questions

- Should the cache be pruned (e.g. `idiot clean downloads`, or keep the
  last N revisions per snap), and should `idiot doctor` report its size?
- Is `--revision <n>` wanted? The v2 info channel map can't serve it. It
  would need the refresh API's `download` action with a revision, which
  needs more design.
- Should a standalone `idiot store download <snap>` (host, to a directory)
  ship with this, since the helper subcommand makes it nearly free?
- Should mixing store snaps with an unasserted file offer `--separate`
  (asserted change first, then the dangerous one) instead of only
  warning?
