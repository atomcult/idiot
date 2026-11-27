# Proposal

## Why

`idiot store snaps` lists a store's snaps and `idiot store channels <snap>`
shows one snap's channel map. Questions about a whole store still need many
calls and some reading:
- which snaps have an `arm64` revision in `latest/edge`?
- where is version `1.4*` released?
- which private snaps does `alice` publish, and which have no revision in
  any open channel?
- what was in `latest/stable` last week, and which uploaded revisions were
  never released?

The store answers these. The v2 info call already returns, for each channel
and architecture, the type, base, confinement, epoch, size and SHA3-384.
For publishers and collaborators, the dashboard's
`/api/v2/snaps/<name>/releases` returns every revision (including
unreleased and rejected ones) and the release history. idiot asks for
neither yet. Store work on brand stores (`ce-team-test`, `dsmr6`) involves
exactly these questions.

## What Changes

- **`idiot store browse [<snap>...]`** (new, host command) explores the
  active store in fzf:
  - **The snap list** is the same one `store snaps` shows: the full list for
    an admin with `store_admin`, otherwise the snaps the account publishes
    or collaborates on (with the same partial-list note), and the account's
    snaps in the global store. Snaps named on the command line are explored
    instead, which also works for public snaps the account has nothing to
    do with.
  - **The preview** shows the highlighted snap's channel map: one row per
    channel with each architecture's revision and version, the target
    architecture highlighted, and the snap's publisher, visibility and type.
  - **Enter opens the snap**: its channel map one row per channel and
    architecture, and, when the credential is allowed to read it, every
    revision with its status, architectures, version, upload time and the
    channels it is released to. The preview shows a revision's details:
    base, confinement, grade, epoch, size, SHA3-384 and its release history.
    Escape goes back to the snap list.
  - **Enter on a row prints it** on stdout (snap, channel, architecture,
    revision, tab-separated) and exits, so other commands can take the
    choice.
- **Filters**, as options, which narrow both the snap list and each snap's
  rows: `--arch`, `--channel` (`edge`, `2.0/*`, `*/stable`), `--version`
  (a glob), `--revision`, `--publisher`, `--private` / `--public`, `--name`
  (a glob), `--type`, `--base`, `--confinement`, `--since <date>` (released
  or uploaded since), and, for revisions, `--status` and `--unreleased`. A
  snap with no row left is left out of the list. fzf's own query narrows
  further while browsing.
- **`--tsv`** prints the filtered rows instead of opening fzf: one per
  snap, channel and architecture, or, with `--revisions`, one per
  revision. Without a terminal, `--tsv` is required.
- **`--cred <name>`** uses another saved credential for the call, as the
  other store queries do.
- **idiot-store** gains two read-only queries:
  - `channel-maps [--store <id>] <snap>...`: several snaps' channel maps,
    each entry with the extra fields above;
  - `releases <snap>`: the dashboard's revisions and release history (needs
    `package_access`; the store refuses it to accounts that are neither
    publisher nor collaborator, and the explorer then shows the channel map
    only and says why).

  `store-snaps` lines gain the snap ID and the latest release after the
  existing fields. `store snaps` and `store channels` behave as before.

Output: `store browse` prints only the chosen row (interactive) or the
filtered rows (`--tsv`) on stdout. Progress, the partial-list note and
refusals go to stderr. Nothing goes into the eval file, and no other
command's stdout changes. There are no new runtime dependencies: the
explorer uses fzf (bundled) and idiot-store (built by the snap already).
Nothing new needs staging.

Out of scope:
- writing to the store (releasing, closing channels);
- assertions for a revision (`store-assertions` fetches them);
- downloading or installing a chosen revision (`vm-install-from-store`
  can take the printed row);
- the store's daily releases feed and metrics.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `store`: adds exploring a store's snaps, channel maps and revisions with
  filters, and extends the credential requirement for store queries to
  `store browse`.

## Impact

- `tools/idiot-store`: `channel-maps` (v2 info with more `fields`, several
  snaps per run), `releases` (`GET /api/v2/snaps/<name>/releases`, paged),
  extra fields on `store-snaps` lines; test fixtures for the new responses.
- `cmd/store/browse` (new), `cmd/store/help`. `share/stores.sh`: the snap
  list `store snaps` builds, moved there (on store-members'
  `store_admin_check`) so both commands share it.
  `cmd/store/snaps` uses it unchanged in behaviour.
- `tests/bare-host.d/store`: usage, filter parsing, the non-terminal rule
  and no-credential paths (offline).
- README (Checking a store), the using-idiot skill reference, the `store`
  spec.
