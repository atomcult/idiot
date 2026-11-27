## MODIFIED Requirements

### Requirement: Credential for store queries
`idiot store check`, `idiot store channels`, `idiot store roles`,
`idiot store snaps` and `idiot store browse` SHALL use the active
credential, or the saved credential named by `--cred <name>` for that call
only, without changing any environment variable. They SHALL fail with
`credentials '<name>' not found` for an unknown name. A query that needs a
permission the credential lacks SHALL say which permission is missing and
name the saved credentials that have it (read offline from each
credential), or say that none does and how to create one (`idiot auth add
<name> --permissions …`). No query SHALL print, log or pass on a command
line any part of a credential, and all SHALL be host commands.

#### Scenario: Another credential for one call
- **WHEN** credential `work` is active and the user runs `idiot store snaps --cred ce-admin`
- **THEN** the query uses `ce-admin`, and afterwards `work` is still the active credential

#### Scenario: Missing permission named
- **WHEN** the active credential lacks `store_admin`, saved credential `ce-admin` has it, and the user runs `idiot store snaps` as an admin of the active store
- **THEN** stderr says the full list needs `store_admin` and suggests `idiot store snaps --cred ce-admin`

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot store roles --cred nope` and no credential `nope` is saved
- **THEN** the command fails with `credentials 'nope' not found` and contacts nothing

#### Scenario: Browsing with another credential
- **WHEN** credential `work` is active and the user runs `idiot store browse --cred ce-admin --tsv`
- **THEN** the snap list and channel maps are read with `ce-admin`, and afterwards `work` is still the active credential

## ADDED Requirements

### Requirement: Exploring a store
`idiot store browse [<snap>...]` SHALL open an fzf explorer of the active
store's snaps. Without snap names, the list SHALL be the one `idiot store
snaps` prints for the same store and credential, with the same
partial-list note on stderr. With snap names, it SHALL explore those
snaps instead, read as the credential sees them (anonymously without
one). The preview of a highlighted snap SHALL show its publisher,
visibility and type and its channel map, one row per channel with each
architecture's revision and version, the target architecture
(`UBUNTU_STORE_ARCH`, else the host's) highlighted. Enter on a snap SHALL
open it: one row per channel and architecture of its channel map, then
one row per revision when the revision history is available. The preview
of a row SHALL show the revision's version, architectures, base,
confinement, grade, epoch, size, SHA3-384 and, with the history, its
upload time, status and every release of it. Escape in an opened snap
SHALL go back to the snap list, and Escape in the snap list SHALL exit 0
printing nothing. Enter on a row SHALL print on stdout the snap name,
channel (`-` for a revision row), architecture (comma-separated for a
revision row) and revision, tab-separated, and exit 0. A snap the store
doesn't show SHALL be listed as not visible, with a preview suggesting
`idiot store check <snap>`. Each snap's data SHALL be fetched from the
store at most once per run, and nothing fetched SHALL be kept after the
command exits.

#### Scenario: Picking a revision
- **WHEN** the user runs `idiot store browse`, opens `ce-dummy`, and presses Enter on its `latest/edge` `arm64` row, revision 4
- **THEN** stdout is `ce-dummy`, `latest/edge`, `arm64` and `4`, tab-separated, and the command exits 0

#### Scenario: Back to the list
- **WHEN** the user opens a snap and presses Escape
- **THEN** the snap list is shown again with the same query

#### Scenario: Cancelled
- **WHEN** the user presses Escape in the snap list
- **THEN** nothing is printed on stdout and the command exits 0

#### Scenario: Named snaps
- **WHEN** no credential is active and the user runs `idiot store browse hello core22`
- **THEN** the explorer lists `hello` and `core22` with their public channel maps

#### Scenario: Snap not visible
- **WHEN** the user runs `idiot store browse nope` and the store doesn't show `nope`
- **THEN** `nope` is listed as not visible and its preview suggests `idiot store check nope`

#### Scenario: Partial list noted
- **WHEN** the account is a viewer of the active store and runs `idiot store browse`
- **THEN** the list holds only the snaps it publishes or collaborates on there, and stderr says the list is partial as `idiot store snaps` does

### Requirement: Revision history in the explorer
For each opened snap, `idiot store browse` SHALL read the snap's revisions
and release history from the store with the credential: every uploaded
revision with its version, architectures, status (such as `Published`,
`Rejected`), upload time, base, confinement, grade, epoch, size and
SHA3-384, and each release (channel, architecture, revision or a closed
channel, time, expiry, progressive percentage). When the store refuses it
(no credential, not a publisher or collaborator, or a missing
permission), the explorer SHALL show the channel map only and say once on
stderr that the revision history needs a credential with `package_access`
for an account that publishes or collaborates on the snap, naming the
reason the store gave.

#### Scenario: Unreleased revisions shown
- **WHEN** the account publishes `ce-dummy`, whose revision 5 was uploaded but never released, and the user opens `ce-dummy`
- **THEN** a revision row for 5 is listed with no channels, and its preview shows its status and upload time

#### Scenario: History refused
- **WHEN** the account is neither publisher nor collaborator of `hello` and the user opens `hello`
- **THEN** only the channel map rows are listed, and stderr says the revision history needs a publisher's or collaborator's credential with `package_access`

### Requirement: Filtering store exploration
`idiot store browse` SHALL accept filters that all have to match, applied
to each snap's rows (channel map rows and revision rows):
`--arch <arch>[,<arch>…]`, `--channel <channel>` (a risk alone meaning
`latest/<risk>`, and `*` matching any track, risk or branch, as in `2.0/*`
or `*/stable`), `--version <glob>`, `--revision <n>`, `--type <type>`,
`--base <base>`, `--confinement <strict|classic|devmode>`, `--since
<YYYY-MM-DD>` (released, or for revisions uploaded, on or after that
day), and, for revision rows only, `--status <status>` and `--unreleased`
(revisions in no open channel). `--channel` SHALL leave out revision rows
not released to a matching channel. Snap-level filters SHALL apply to the
list: `--name <glob>`, `--publisher <username>`, and `--private` or
`--public`. A snap with no row left after the row filters SHALL be left
out of the list. `--status` and `--unreleased` SHALL fail before
contacting the store when combined with `--tsv` but not `--revisions`,
saying they apply to revisions. An unknown option or a malformed value
SHALL fail with a usage message before contacting the store.

#### Scenario: Architecture and channel
- **WHEN** the user runs `idiot store browse --arch arm64 --channel edge`
- **THEN** the list holds only snaps with an `arm64` revision in `latest/edge`, and each opened snap shows only those rows

#### Scenario: Version glob
- **WHEN** the user runs `idiot store browse --version '1.4*' --tsv`
- **THEN** every printed row's version starts with `1.4`

#### Scenario: Publisher and visibility
- **WHEN** the user runs `idiot store browse --publisher alice --private`
- **THEN** the list holds only private snaps published by `alice`

#### Scenario: Unreleased revisions
- **WHEN** the user runs `idiot store browse ce-dummy --unreleased --revisions --tsv`
- **THEN** only revisions of `ce-dummy` in no open channel are printed

#### Scenario: Malformed filter
- **WHEN** the user runs `idiot store browse --since yesterday`
- **THEN** the command fails with a usage message and contacts nothing

### Requirement: Machine-readable exploration
`idiot store browse --tsv` SHALL print the filtered rows on stdout
instead of opening fzf, tab-separated with no colour: one row per snap,
channel and architecture (snap, channel, architecture, revision, version,
release time, type, base or `-`, confinement, publisher, `private` or
`public`), or with `--revisions` one row per revision (snap, revision,
version, architectures comma-separated, status, upload time, channels
comma-separated or `-`, base or `-`, confinement, grade). Snaps the store
doesn't show and refused histories SHALL be reported on stderr only.
Progress while fetching SHALL go to stderr. Without a terminal on stdin
and stderr, and without `--tsv`, the command SHALL fail before contacting
the store, saying to pass `--tsv`.

#### Scenario: Channel rows
- **WHEN** the user runs `idiot store browse ce-dummy --tsv | cut -f2,3,4`
- **THEN** each line is a channel, an architecture and a revision of `ce-dummy`'s channel map

#### Scenario: Revision rows
- **WHEN** the account publishes `ce-dummy` and the user runs `idiot store browse ce-dummy --revisions --tsv`
- **THEN** each line is one revision of `ce-dummy`, released or not, with its status and channels

#### Scenario: No terminal
- **WHEN** the user runs `idiot store browse </dev/null`, without `--tsv`
- **THEN** the command fails saying to pass `--tsv`, and contacts nothing
