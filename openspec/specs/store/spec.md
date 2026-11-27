# store Specification

## Purpose

Saving Dedicated Snap Store IDs under readable names and switching the
current shell between them via `UBUNTU_STORE_ID`.

## Requirements

### Requirement: Saved stores
Stores SHALL be saved one per file in `IDIOT_STORES_DIR` (default
`$XDG_DATA_HOME/idiot/stores`), the file holding the store ID.
`idiot store add <name> <store-id>` SHALL accept only names made of letters,
digits, `-` and `_`, and refuse an existing name. `idiot store remove <name>`
SHALL delete a saved store. `idiot store list` SHALL print names and IDs on
stderr, or a hint when none are saved; it uses only idiot's state.

`idiot store list --account [--tsv] [--cred <name>]` SHALL instead print on
stdout each store the account belongs to: its saved name when one has its
ID, its ID, its display name, and the account's roles (`admin`, `review`,
`view`, `access`), marking the active store. `--tsv` SHALL print the same
as tab-separated fields (active, saved name, ID, roles, display name).
When the account belongs to no store other than the global one, it SHALL
say so on stderr. `--tsv` and `--cred` SHALL be refused without
`--account`.

#### Scenario: Add
- **WHEN** the user runs `idiot store add staging abc123`
- **THEN** a store `staging` with ID `abc123` is saved

#### Scenario: Invalid name
- **WHEN** the user runs `idiot store add ../x abc123`
- **THEN** the command fails and nothing is written

#### Scenario: Duplicate
- **WHEN** `staging` exists and the user runs `idiot store add staging def456`
- **THEN** the command fails and the saved ID is unchanged

#### Scenario: Empty list
- **WHEN** no stores are saved and the user runs `idiot store list`
- **THEN** a hint to use `idiot store add` is printed

#### Scenario: The account's stores
- **WHEN** the account is an admin of `ce-team-test` and a viewer of `dsmr6`, `dsmr6` is active, and the user runs `idiot store list --account`
- **THEN** stdout lists both with their roles, `dsmr6` marked active

#### Scenario: Old name
- **WHEN** the user runs `idiot store roles --tsv`
- **THEN** stderr notes that it is now `idiot store list --account`, and stdout holds the same rows `idiot store list --account --tsv` prints

### Requirement: Activating a store
`idiot store use [<name>]` SHALL emit an export of `UBUNTU_STORE_ID` set to
the saved ID and an export of `IDIOT_ACTIVE_STORE` set to the store's
name, offering an fzf picker when no name is given (cancelling changes
nothing). `idiot store unset` SHALL emit unsets of `UBUNTU_STORE_ID` and
`IDIOT_ACTIVE_STORE`. `idiot store status` SHALL print the active ID, or
`global` when none is set.

#### Scenario: Use
- **WHEN** the user runs `idiot store use staging`
- **THEN** `UBUNTU_STORE_ID` is `abc123` and `IDIOT_ACTIVE_STORE` is `staging` in the current shell

#### Scenario: Unknown store
- **WHEN** the user runs `idiot store use nope`
- **THEN** the command fails with `store 'nope' not found`

#### Scenario: Back to the global store
- **WHEN** the user runs `idiot store unset`
- **THEN** `UBUNTU_STORE_ID` and `IDIOT_ACTIVE_STORE` are unset and `idiot store status` prints `global`

### Requirement: Warning on a store switch
`idiot store use` and `idiot store unset` SHALL, after queueing their
change, print the auth capability's store mismatch warning when the active
credential is paired with a store other than the one now active. The
switch itself SHALL happen regardless, and the exit status SHALL be zero.

#### Scenario: Switching away from the credential's store
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store use prod`
- **THEN** `UBUNTU_STORE_ID` is `prod`'s ID, the command exits zero, and stderr warns that `ci` belongs to `staging` while `prod` is active

#### Scenario: Switching to the credential's store
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store use staging`
- **THEN** no warning is printed

#### Scenario: Back to global
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store unset`
- **THEN** `UBUNTU_STORE_ID` is unset and stderr warns that `ci` belongs to `staging` while the global store is active

### Requirement: Credential for store queries
`idiot store check`, `idiot store channels`, `idiot store list --account`,
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
- **WHEN** the user runs `idiot store list --account --cred nope` and no credential `nope` is saved
- **THEN** the command fails with `credentials 'nope' not found` and contacts nothing

#### Scenario: Browsing with another credential
- **WHEN** credential `work` is active and the user runs `idiot store browse --cred ce-admin --tsv`
- **THEN** the snap list and channel maps are read with `ce-admin`, and afterwards `work` is still the active credential

### Requirement: Checking a snap's availability
`idiot store check <snap>` SHALL determine whether the credential can
download `<snap>` from the active store (`UBUNTU_STORE_ID`, else the global
store) for the target architecture (`UBUNTU_STORE_ARCH`, else the host's)
and channel (`--channel <channel>`, default `stable`). It SHALL look the
snap up as snapd does (sending the store ID), try the download both
anonymously and with the credential, reading only the response status and
never the snap itself, and read the account's roles in the active store.
It SHALL also check the credential offline (expired, missing
`package_access`, base64 padding snapd can't read, paired with another
store). It SHALL print a report on stderr: one line per check (`✓` fine,
`!` worth knowing, `✗` the problem) followed by a conclusion naming the
cause and what to do. It SHALL exit 0 when the credential can download the
snap and 1 otherwise. Without an active credential it SHALL check anonymous
access only, and say so.

#### Scenario: Downloadable
- **WHEN** the active credential can download `ce-dummy` from the active store for the target architecture
- **THEN** the report ends with a `✓` conclusion and the command exits 0

#### Scenario: Not a member of the store
- **WHEN** `ce-team-test` is active, the snap can't be found or downloaded, and the account has no role in `ce-team-test`
- **THEN** the conclusion says the account isn't a member of `ce-team-test`, to ask a store admin to add it, and the command exits 1

#### Scenario: Not released for the target
- **WHEN** the snap exists in the store but has no revision for `arm64` in `latest/stable`
- **THEN** the conclusion says so and lists the channels that do have an `arm64` revision

#### Scenario: Credential snapd can't read
- **WHEN** the active credential has base64 padding
- **THEN** the report has a `✗` line saying snapd would download anonymously, and suggests `idiot auth use <name>`

#### Scenario: Only anonymous access
- **WHEN** no credential is active and the snap is public
- **THEN** the report says only anonymous access was checked and the command exits 0

### Requirement: Channel map
`idiot store channels <snap>` SHALL print on stdout the snap's channel map
in the active store, as the credential sees it: one row per channel
(`<track>/<risk>`, and branch when present) with each architecture's
revision and version. The target architecture (`UBUNTU_STORE_ARCH`, else
the host's) SHALL be highlighted on a terminal, and `--arch <arch>` SHALL
show only that architecture. `--tsv` SHALL print one tab-separated row per
channel and architecture: channel, architecture, revision, version,
release time. A snap the store doesn't show SHALL fail with a message
suggesting `idiot store check <snap>`.

#### Scenario: Channel map with the target highlighted
- **WHEN** `UBUNTU_STORE_ARCH` is `arm64` and the user runs `idiot store channels ce-dummy`
- **THEN** stdout lists each channel's revisions per architecture with the `arm64` column highlighted

#### Scenario: Machine-readable map
- **WHEN** the user runs `idiot store channels ce-dummy --tsv | grep arm64`
- **THEN** each line is a channel, `arm64`, a revision, a version and a time, separated by tabs, with no colour

#### Scenario: Snap not visible
- **WHEN** the store doesn't show `nope` to the credential
- **THEN** the command fails with a message suggesting `idiot store check nope`

### Requirement: Listing a store's snaps
`idiot store snaps` SHALL print on stdout the snaps in the active store,
one per line (name, then publisher and whether it is private), using the
store's full list when the account is an admin of the store and the
credential has `store_admin`. Otherwise it SHALL print the snaps in that
store that the account publishes or collaborates on, and say on stderr
that the list is partial and what the full list needs. With the global
store active it SHALL list the snaps the account publishes or collaborates
on there, without a partial-list note. `--tsv` SHALL print
tab-separated fields; `--pick` SHALL open an fzf picker over the list and
print only the chosen name.

#### Scenario: Full list for an admin
- **WHEN** the account is an admin of the active store and the credential has `store_admin`
- **THEN** stdout lists every snap registered in or added to the store

#### Scenario: Partial list
- **WHEN** the account is a viewer of the active store
- **THEN** stdout lists only the snaps it publishes or collaborates on there, and stderr says the list is partial because the full list needs an admin with `store_admin`

#### Scenario: Picking a snap
- **WHEN** the user runs `idiot store check "$(idiot store snaps --pick)"` and picks `ce-dummy`
- **THEN** `store check` runs for `ce-dummy`

### Requirement: Importing the account's stores
`idiot store import [--all] [--cred <name>]` SHALL save, as saved stores,
the stores the account behind the credential belongs to (the active
credential, or the one `--cred` names). It SHALL skip the global store and
any store whose ID is already saved under some name. On a terminal without
`--all` it SHALL offer the remaining stores in a multi-select picker
showing each store's display name, ID and roles, then prompt for each
picked store's name, defaulting to its display name lowercased with each
run of characters other than `a-z` and `0-9` replaced by `-`, and leading
and trailing `-` removed. With `--all` it SHALL save every remaining store
under its default name without asking. Without a terminal and without
`--all` it SHALL fail before contacting the store, saying to pass `--all`.
Names SHALL follow `idiot store add`'s rules. A default name already saved
for another store SHALL get `-` and the ID's first four characters, and a
typed name that is taken or invalid SHALL be asked for again. No saved
store SHALL be overwritten. It SHALL report each store saved, and finish
with a count of stores saved and skipped, all on stderr, and SHALL never
pair credentials with stores.

#### Scenario: Import everything
- **WHEN** the account belongs to the global store, `Example Team Store` (`abc123`) and `Example Brand Store` (`def456`), none saved, and the user runs `idiot store import --all`
- **THEN** stores `example-team-store` (`abc123`) and `example-brand-store` (`def456`) are saved, and the global store isn't

#### Scenario: Already saved
- **WHEN** `abc123` is saved as `team` and the user runs `idiot store import --all`
- **THEN** `abc123` is skipped and `team` is unchanged

#### Scenario: Name taken by another store
- **WHEN** `example-team-store` is saved for `zzz999` and the account's `Example Team Store` has ID `abc123`
- **THEN** `idiot store import --all` saves `abc123` as `example-team-store-abc1` and leaves `zzz999` as it was

#### Scenario: Picking and naming
- **WHEN** on a terminal the user picks `Example Brand Store` and types `brand` at the name prompt
- **THEN** only `brand` (`def456`) is saved

#### Scenario: No terminal
- **WHEN** the user runs `idiot store import` with stdin not a terminal
- **THEN** the command fails saying to pass `--all`, and contacts nothing

#### Scenario: No credential
- **WHEN** no credential is active and no `--cred` is given
- **THEN** the command fails saying to log in first, and saves nothing

### Requirement: Fetching an assertion from the store
`idiot assertion fetch <type> [<key>...]` SHALL fetch one assertion from
the store's assertions service and accept these forms:
- `account <account-id>`;
- `account-key <sha3-384>`;
- `store [<store-id>]`, the active store's ID (`UBUNTU_STORE_ID`) when
  none is given, failing before contacting the store when neither is set;
- `model <brand-id>/<model>`;
- `snap-declaration <snap-name>`, the snap-id looked up in the active store
  (`UBUNTU_STORE_ID`, else the global store);
- `snap-revision <snap-name> <revision>`, the revision's SHA3-384 looked up
  in the active store, for the target architecture (`--arch <arch>`, else
  `UBUNTU_STORE_ARCH`, else the host's) where the lookup needs one;
- `snap-revision <snap-name> [<channel>]`, the revision released in that
  channel for the target architecture, `stable` when no channel is given. A
  word of digits only is a revision; anything else is a channel (`<risk>`,
  `<track>/<risk>` or `<track>/<risk>/<branch>`; a risk alone means the
  snap's default track, and a track alone must be written
  `<track>/stable`). A risk with nothing released SHALL follow the next
  more stable risk in its track, as snapd does, and the summary SHALL say
  which channel and revision were used. When nothing is released there for
  that architecture it SHALL fail naming the snap, channel and
  architecture and suggesting `idiot store channels <snap>`;
- `snap-revision <sha3-384>`, given in hex or base64url (a word too long to
  be a snap name);
- `validation-set <account-id>/<name>[=<sequence>]`, the latest sequence
  when none is given;
- for any type the store serves, `<type> <header>=<value>...`: the type's
  primary-key headers, with `series` defaulting to `16` and an optional
  trailing primary-key header (such as `provenance`) to the store's default.
It SHALL fail before contacting the store, naming the expected headers,
when a primary-key header is missing or an unknown header is given, and
SHALL fail on a type it doesn't know, listing the types it does. It SHALL
use the active credential, or the saved credential named by `--cred
<name>` for that call only without changing any environment variable, and
fetch anonymously when there is neither. It SHALL fail with `credentials
'<name>' not found` for an unknown name, before contacting the store. When
the store has no such assertion it SHALL fail saying so, naming the type
and key, and that the store may also be hiding it from this credential; a
snap name the store doesn't show SHALL fail suggesting `idiot store check
<snap>`. It SHALL NOT print, log or pass on a command line any part of a
credential, SHALL NOT change snapd's assertion database, and SHALL be a
host command.

#### Scenario: A snap's declaration by name
- **WHEN** the global store is active and the user runs `idiot assertion fetch snap-declaration hello-world`
- **THEN** the `snap-declaration` with series `16` and snap-id `buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ` is fetched and summarised, showing snap name `hello-world` and publisher `canonical`

#### Scenario: A revision by number
- **WHEN** the user runs `idiot assertion fetch snap-revision hello-world 29`
- **THEN** the `snap-revision` for revision 29's SHA3-384 is fetched and the summary shows revision 29, its size and its SHA3-384

#### Scenario: A revision by channel
- **WHEN** revision 29 of `hello-world` is in `latest/stable` for amd64 and the user runs `idiot assertion fetch snap-revision hello-world stable --arch amd64`
- **THEN** the `snap-revision` for revision 29 is fetched, and the summary says it came from `latest/stable`

#### Scenario: Default channel
- **WHEN** the user runs `idiot assertion fetch snap-revision hello-world`
- **THEN** the revision in `stable` for the target architecture is fetched, as with `stable` given

#### Scenario: Closed risk follows down
- **WHEN** `edge` has nothing released for amd64, `beta` has revision 30, and the user runs `idiot assertion fetch snap-revision hello-world edge --arch amd64`
- **THEN** revision 30's `snap-revision` is fetched, and the summary says `edge` follows `beta`

#### Scenario: Nothing released for the architecture
- **WHEN** a snap has no release in any risk of `latest` for riscv64 and the user runs `idiot assertion fetch snap-revision <snap> --arch riscv64`
- **THEN** the command fails naming the snap, `latest/stable` and riscv64, suggesting `idiot store channels <snap>`, and fetches no assertion

#### Scenario: A revision by hash
- **WHEN** the user runs `idiot assertion fetch snap-revision` with the hex SHA3-384 of a snap the store has
- **THEN** the same `snap-revision` is fetched as with its base64url form

#### Scenario: Latest validation set
- **WHEN** the user runs `idiot assertion fetch validation-set acme/base-set` and the store holds sequences 1 to 3
- **THEN** sequence 3 is fetched

#### Scenario: Any type by its primary key
- **WHEN** the user runs `idiot assertion fetch serial brand-id=acme model=gw serial=1234`
- **THEN** that `serial` assertion is requested by its primary key

#### Scenario: Missing key
- **WHEN** the user runs `idiot assertion fetch model canonical`
- **THEN** the command fails saying a model is fetched by `<brand-id>/<model>`, and contacts nothing

#### Scenario: Unknown type
- **WHEN** the user runs `idiot assertion fetch nope x`
- **THEN** the command fails listing the assertion types it knows, and contacts nothing

#### Scenario: Not in the store
- **WHEN** the user runs `idiot assertion fetch account no-such-account`
- **THEN** the command fails saying the store has no `account` `no-such-account`, or doesn't show it to this credential

#### Scenario: No store active
- **WHEN** no store is active and the user runs `idiot assertion fetch store`
- **THEN** the command fails saying no store is active and to name a store ID, and contacts nothing

#### Scenario: Anonymous fetch
- **WHEN** no credential is active and the user runs `idiot assertion fetch account canonical`
- **THEN** Canonical's `account` assertion is fetched without credentials and summarised

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot assertion fetch account canonical --cred nope` and no credential `nope` is saved
- **THEN** the command fails with `credentials 'nope' not found` and contacts nothing

### Requirement: Fetched assertion output
By default `idiot assertion fetch` SHALL print on stdout the summary and
signature status `idiot assertion show` prints for the fetched stream,
with the same model cross-checks on stderr and the same exit status, and
`--no-verify` SHALL skip the signature check as it does there. With
`--raw` it SHALL print the assertion stream, and nothing else, on stdout:
each assertion byte for byte as the store served it, separated by a blank
line as `snap known` prints them, so it can be piped to `snap ack /dev/stdin`
or `idiot assertion show -`. With `-o <file>` it SHALL write that stream
to the file, replacing it only once the whole stream has been fetched,
print nothing on stdout, and say on stderr how many assertions it wrote
and where. `--raw` and `-o` SHALL NOT both be given. Messages and progress
SHALL go to stderr.

#### Scenario: Human-readable
- **WHEN** the user runs `idiot assertion fetch account canonical`
- **THEN** stdout shows `type: account`, account `canonical`, its signing key, and its signature status, as `idiot assertion show` would for the same assertion

#### Scenario: Raw to stdout
- **WHEN** the user runs `idiot assertion fetch --raw account-key BWDEoaqyr25nF5SNCvEv2v7QnM9QsfCc0PBMYD_i2NGSQ32EF2d4D0hqUel3m8ul | head -1`
- **THEN** the line printed is `type: account-key`

#### Scenario: To a file
- **WHEN** the user runs `idiot assertion fetch --chain -o hello.assert snap-revision hello-world 29`
- **THEN** `hello.assert` holds the stream, stdout is empty, and stderr says how many assertions were written to `hello.assert`

#### Scenario: Failed fetch leaves the file alone
- **WHEN** `hello.assert` exists and `idiot assertion fetch -o hello.assert account no-such-account` fails
- **THEN** `hello.assert` is unchanged

### Requirement: Prerequisite chain
With `--chain`, `idiot assertion fetch` SHALL fetch, besides the assertion
asked for, every assertion snapd needs before it accepts it: its signing
`account-key` and that key's own chain of signing keys, and the
assertions it refers to (an `account-key`'s `account`; a
`snap-declaration`'s publisher `account`; a `snap-revision`'s
`snap-declaration` and developer `account`; a `snap-developer`'s
`snap-declaration` and the accounts it names; a `store`'s operator
`account`), applying the same to each assertion fetched. It SHALL order
the stream so each assertion comes after the ones it needs, list each
assertion once, and leave out what snapd trusts built in (Canonical's
`account` and root `account-key`). The summary or raw output SHALL cover
the whole stream. A prerequisite the store doesn't have SHALL make the
command fail, naming it, and write no file.

#### Scenario: Assertions to side-load a snap
- **WHEN** the user runs `idiot assertion fetch --chain --raw snap-revision hello-world 29 > hello.assert`
- **THEN** `hello.assert` holds Canonical's `store` account-key, then the `snap-declaration` for `hello-world`, then the `snap-revision` for revision 29, each once, and neither Canonical's `account` nor its root key

#### Scenario: Verifies offline
- **WHEN** the user runs `idiot assertion fetch --chain model acme/gw` for a brand whose key this machine's snapd has never seen
- **THEN** the summary shows the brand's `account-key` and `account` before the model, and the model is `verified` without any further network request

#### Scenario: Accepted by snapd
- **WHEN** the user runs `idiot assertion fetch --chain --raw snap-revision hello-world 29 | sudo snap ack /dev/stdin`
- **THEN** snapd accepts every assertion in the stream

### Requirement: Listing a store's members
`idiot store members` SHALL print on stdout the members of the active store,
one per line sorted by username: username, roles (comma-separated) and
display name. `--role <role>` SHALL keep only members with that role,
`--email` SHALL add each member's email, and `--tsv` SHALL print the same
fields tab-separated (username, roles, display name, then email with
`--email`). Emails SHALL NOT be printed without `--email`. The command SHALL
first check that the account is an admin of the active store and that the
credential has `store_admin`, and fail otherwise, saying which is missing.
For a missing permission it SHALL name the saved credentials that have it,
or say how to create one. With the global store active it SHALL fail
saying the global store has no member list. It SHALL follow the credential
rules for store queries (`--cred`, never printing a credential) and change
nothing in the store.

#### Scenario: Members listed
- **WHEN** the account is an admin of the active store and the credential has `store_admin`
- **THEN** stdout lists each member's username, roles and display name, and no email addresses

#### Scenario: Only one role
- **WHEN** the user runs `idiot store members --role admin`
- **THEN** only members with the `admin` role are listed

#### Scenario: Emails on request
- **WHEN** the user runs `idiot store members --email --tsv`
- **THEN** each line is username, roles, display name and email, tab-separated

#### Scenario: Not an admin
- **WHEN** the account has only the `view` role in the active store
- **THEN** the command fails saying the member list needs an admin of the store, and nothing is printed on stdout

#### Scenario: Missing store_admin
- **WHEN** the account is an admin but the credential lacks `store_admin`, and saved credential `ce-admin` has it
- **THEN** the command fails suggesting `idiot store members --cred ce-admin`

#### Scenario: Global store
- **WHEN** no store is active
- **THEN** the command fails saying the global store has no member list

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
