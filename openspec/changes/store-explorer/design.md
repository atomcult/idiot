# Design

## Context

What exists (from add-store-commands and store-import):
- `idiot-store account`, `store-snaps --store <id>` (admins with
  `store_admin`) and `info <snap> [--store <id>]` print tab-separated
  lines. sh decides and formats.
- `cmd/store/snaps` builds a store's snap list: the full list for an admin
  with `store_admin`, else the account's snaps there, with a partial-list
  note. Its admin pre-check is `store_admin_check` in `share/stores.sh`
  (from store-members). `cmd/store/channels` formats one channel map.
- `query_cred` (creds.sh) resolves the active credential or `--cred`.
  Authorised calls send snapd's header and refresh the discharge in memory.
- fzf 0.44.1 is bundled (Ubuntu 24.04's package; `host/bin/fzf` for host
  commands). `require fzf` finds the host's first.

What the store offers, and how sure we are:

| Call | Gives | Who | Source |
|---|---|---|---|
| `GET api.snapcraft.io/v2/snaps/info/<name>?fields=…` | per channel and arch: revision, version, `released-at`, `created-at`, `type`, `base`, `confinement`, `epoch`, `download.{size,sha3-384,url}`, `architectures`; and `snap.{snap-id,publisher,private,unlisted,title,summary,license,store-url}` | anyone who can see the snap | **live**, anonymous, on `hello`; the allowed `fields` are listed in the error for an unknown one (`grade` isn't one) |
| `GET dashboard.snapcraft.io/api/v2/snaps/<name>/releases` | `revisions[]` (every upload: `revision`, `version`, `architectures`, `status`, `created_at`, `base`, `confinement`, `grade`, `epoch`, `size`, `sha3-384`, `attributes`, `build_url`), `releases[]` (history: `channel`, `track`, `risk`, `branch`, `architecture`, `revision` or null for a close, `when`, `expiration-date`, `progressive`), `snap` (tracks, channels, `default-track`), `_links` with `next` | `package_access`; roles not documented | dashboard v2 snaps reference; snapcraft's `list_releases` and `snapcraft.models.Releases`. **Live**: answers 401 `macaroon-authorization-required` anonymously, so the path exists |
| `GET dashboard.snapcraft.io/api/v2/snaps/<name>/channel-map` | current map incl. closed channels and progressive state | `package_access` | docs, snapcraft `get_channel_map`; live 401 anonymously. **Not used** (see below) |
| `GET dashboard.snapcraft.io/api/v2/stores/<id>/snaps` | per snap also `id`, `store`, `other-stores`, `latest-release.{channel,revision,timestamp,version}`; `?q=`, `?publisher=` | store admin, `store_admin` | dashboard v2 stores reference; not yet in our fixture |

The releases page holds up to 500 release records and is paged with
`?page=` / `?size=` and `_links.next`. Names differ between the two dashboard
endpoints (`created_at`/`build_url` there, `created-at`/`build-url` in
channel-map), and snapcraft's model calls the hash `sha3_384`. So the parser
accepts both spellings of each.

## Goals / Non-Goals

**Goals:**
- One command that answers "where is what" across a store, interactively
  or as `--tsv`, without a store admin where the data allows it.
- No network call per cursor move after the first for a snap. No call at
  all for filters that fail validation.
- The store snap list stays defined in one place.

**Non-Goals:**
- Replacing `store channels` or `store snaps` (both stay as they are).
- Closed channels, progressive release controls, or branch expiry as
  actions. They are shown where the history has them, but nothing is
  changed.
- Searching the global store's catalogue (`/v2/snaps/find`). Named snaps
  cover the public-snap case.
- Assertions, downloads, installs (`store-assertions`,
  `vm-install-from-store`).

## Decisions

**Command shape: `idiot store browse`, a new leaf next to `snaps` and
`channels`.** `browse` is a superset: it starts from the list `snaps` prints
and drills into what `channels` shows, plus revisions. We considered
extending `store snaps` with filters and an fzf mode. That would turn a
simple list command into the explorer and blur `--pick`, which `store check
"$(idiot store snaps --pick)"` relies on. A separate verb keeps both
contracts. `browse` (not `explore`) matches `idiot inspect changes`'s
browser and is short. It is a host command (network).

**The snap list moves to `share/stores.sh`.** store-members, applied
first, already moved the admin pre-check there: `store_admin_check <id>`
prints `ok`, `role` or `permission`, and `store_admin_alternatives` words
the `--cred` hint. The list is built on them, not on a copy:
`store_snap_list` sets `store_snap_rows` (name, publisher,
private|public, then the latest release's channel, revision and version,
`-` outside the full list) and `store_snap_partial` (`""`, `role` or
`permission`); `store_partial_note <why> <command>` says why the list is
partial. It sets variables rather than printing, since a function run in
`$(…)` can't hand back the partial reason. `store snaps` calls it and cuts
the first three fields. Its output, notes and exits stay the same,
checked against the previous version with a stub and by the existing
bare-host cases. (Adapted while applying: this decision first named a
printing `store_snap_rows` with `_partial_note` lifted unchanged from
`cmd/store/snaps`.)

**idiot-store `channel-maps [--file] [--store <id>] <snap>...`, a new
subcommand, not a change to `info`.** `info`'s line format is read by
`store channels` and `store check`. A multi-snap call with more fields needs
the snap name on every line, so it gets its own subcommand. It reuses
`info_request` and `parse_info` (extended to read the extra fields).
`info` prints what it prints today. Lines, in argument order:

```
snap     <name> <snap-id> <publisher username> <private|public> <title>
channel  <name> <track/risk[/branch]> <arch> <revision> <version> <released-at>
         <created-at> <type> <base|-> <confinement> <epoch> <size> <sha3-384>
hidden   <name>                        (the store answered 404)
```

The epoch is printed as snapd writes it (`0`, `1*`), else
`read=…;write=…`. Tabs and newlines in titles become spaces, and the title
goes last. Up to four requests run at once on scoped threads sharing one
client and `Auth`. A refresh is serialised behind a mutex, so it happens at
most once. Exit 0 even when some snaps are hidden, 2 rejected, 3 any other
store error. Why Rust: these are authorised HTTPS calls with the
credential in a header. sh would have to give the macaroon to curl in argv
or a file, and the house rule is that only idiot-store touches it.

**idiot-store `releases [--file] <snap>`.** It reads every page,
following `_links.next` only when it stays on dashboard.snapcraft.io, so
the credential is never sent elsewhere. A hard cap of 20 pages (10,000
releases) stops it, with a note on stderr. Lines:

```
revision <rev> <version> <arch,arch> <status> <created-at> <base|-> <confinement>
         <grade> <epoch> <size> <sha3-384>
release  <track/risk[/branch]> <arch> <rev|-> <when> <expires|-> <progressive %|->
refused  permission <perm> | not-collaborator | no-credential
```

`refused` maps 403 `macaroon-permission-required`, 404/403 (the store
won't say whether the snap is absent or ours) and 401 without a credential.
It exits 0, like `store-snaps`, so sh can show the map and explain. The
dashboard's `channel-map` endpoint is not used. The history plus v2 info
cover the same ground, and v2 info is what devices actually see in the
active store.

**`store-snaps` lines gain fields at the end**: `<snap-id>
<latest channel|-> <latest revision|-> <latest version|->` from
`latest-release`. Existing readers take fields 2-4 by position and are
unaffected. browse shows the latest release in the list for admins without
fetching every map.

**sh filters, Rust doesn't.** idiot-store prints everything. browse turns
the lines into TSV rows in a per-run cache and filters them in sh: `case`
for globs (POSIX, no regex translation) and awk for the rest. ISO times
compare as strings, so `--since` is `substr(time, 1, 10) >= since`. The
rules:
- `--channel`: a bare risk becomes `latest/<risk>` (as `normalise_channel`
  does). `*` comes from the glob. A revision row matches when any channel
  it is currently in matches.
- "Currently in" and "unreleased" come from the v2 channel map, not the
  history. So they agree with the channel rows and with what snapd sees in
  the active store.
- Values are checked before any call: revision digits only, a
  `YYYY-MM-DD` date, the confinement enum, architectures from
  `STORE_ARCHES`. A bad value is a usage error.

**When data is fetched.**
- The list is fetched first.
- With no row filter and no `--tsv`, each snap's map is fetched lazily by
  its preview.
- With a row filter or `--tsv`, all listed snaps' maps are fetched up front
  in one `channel-maps` call, with `reading N channel maps…` on stderr.
  The list can only be filtered by row contents once those are known.
- `releases` is fetched when a snap is opened, or up front for
  `--revisions` and the revision-only filters, one snap at a time with
  progress.
- Each result is written to a temp file and renamed into the cache, so a
  preview fzf kills mid-fetch leaves nothing half-written. The next preview
  fetches again. That is the only case where a snap is fetched twice.

**The cache** is a `mktemp -d` directory (mode 0700), exported to
previews as `IDIOT_BROWSE_DIR`, and removed by an EXIT/INT/TERM trap. It
holds store data only, never the credential. Private snaps' names and
versions are there for the length of the run.

**fzf wiring (0.44-compatible).**
- Previews and the opened view call the command itself through hidden
  entries, `"${0}" __preview snap {1}` and `__preview row <snap> {..}`.
  They are not in `--help` and are never dispatched by idiot. They read
  the exported cache dir, the store ID (already in the environment), the
  `--cred` file path (a path, not a credential) and the filters, and then
  fetch on a cache miss.
- fzf is run with `SHELL=/bin/sh`: it runs previews with `$SHELL -c`, and
  the user's shell may be fish.
- Rows carry ANSI colour (target arch bold, closed or refused dim) and fzf
  gets `--ansi`.
- The list and the opened view are two fzf runs in a sh loop. Escape in
  the opened view returns to the list with the saved `--print-query`
  query. Enter prints the row's key fields and exits.
- The prompts are `snap> ` and `<snap>> `.
- No `become`, `transform` or `--with-shell`, which 0.44 lacks or which
  are recent.

**Credential handling.** No credential is ever on argv: children inherit
`UBUNTU_STORE_AUTH` from the environment, and `--cred` passes
`--file <path>` as the other queries do. Without names, browse needs a
credential (as `store snaps` does). With names, anonymous works for public
snaps, and `releases` is then skipped with the `no-credential` explanation
and no call. Missing-permission hints reuse `creds_with_permission`
(`store_admin` for the full list, `package_access` for histories).

**Terminal rule.** Without `--tsv`, stdin and stderr must be terminals,
checked before any call, as in `store import`. stdout may be a pipe: the
chosen row goes there, and fzf draws on the tty.

**Other changes.** The printed row (snap, channel, arch, revision) is the
key `vm-install-from-store` (`<snap>=<channel>`) and `store-assertions`
(snap-revision by snap and revision) take. `assertion-writer` may use
`browse` to pick revisions. None of them is changed here.

## Risks / Trade-offs

- **Who may read `releases` is undocumented.** Brand-store admins or
  reviewers may be refused for snaps they don't collaborate on. → The
  explorer degrades to the channel map with an explanation. A with-user
  task records what each role gets, and the explanation is tuned to it.
- **Big stores make eager fetching slow** (a few hundred snaps, four at a
  time). → Fetching is lazy unless a row filter needs the data. Progress is
  on stderr, and snap-level filters (`--name`, `--publisher`,
  `--private`) cut the list before any map is fetched.
- **Rate limits** on api.snapcraft.io are unknown. → Four concurrent
  requests, no retries other than the refresh. A 429 is a store error
  (exit 3) naming the status.
- **Response details beyond the docs** (history order, `_links.next`
  form, `latest-release` presence, the hash's spelling). → The first task
  records real responses as fixtures (account details replaced), and
  parsers are written from them, accepting both spellings.
- **A preview fetch can race the next preview** for the same snap. →
  Atomic rename; the worst case is a second fetch.

## Open Questions

- Which brand-store roles (admin, reviewer, viewer) can read
  `/api/v2/snaps/<name>/releases` for snaps they neither publish nor
  collaborate on? This only affects the wording of the refusal.
- Does the v2 info channel map in a brand store include snaps the store
  includes from other stores? That affects named-snap browsing only, not
  the list (`store-snaps` leaves included snaps out by design).
- Is a four-request concurrency limit right, or should it be configurable?
  Deferrable: it can be tuned without changing behaviour.
