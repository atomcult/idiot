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
stderr, or a hint when none are saved.

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
`idiot store check`, `idiot store channels`, `idiot store roles` and
`idiot store snaps` SHALL use the active credential, or the saved
credential named by `--cred <name>` for that call only, without changing
any environment variable. They SHALL fail with `credentials '<name>' not
found` for an unknown name. A query that needs a permission the credential
lacks SHALL say which permission is missing and name the saved credentials
that have it (read offline from each credential), or say that none does
and how to create one (`idiot auth add <name> --permissions …`). No query
SHALL print, log or pass on a command line any part of a credential, and
all SHALL be host commands.

#### Scenario: Another credential for one call
- **WHEN** credential `work` is active and the user runs `idiot store snaps --cred ce-admin`
- **THEN** the query uses `ce-admin`, and afterwards `work` is still the active credential

#### Scenario: Missing permission named
- **WHEN** the active credential lacks `store_admin`, saved credential `ce-admin` has it, and the user runs `idiot store snaps` as an admin of the active store
- **THEN** stderr says the full list needs `store_admin` and suggests `idiot store snaps --cred ce-admin`

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot store roles --cred nope` and no credential `nope` is saved
- **THEN** the command fails with `credentials 'nope' not found` and contacts nothing

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
- **THEN** the report has a `✗` line saying snapd would download anonymously, and suggests `idiot auth login <name>`

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

### Requirement: Store roles
`idiot store roles` SHALL print on stdout each store the account belongs
to: its saved name when one has its ID, its ID, its display name, and the
account's roles (`admin`, `review`, `view`, `access`), marking the active
store. `--tsv` SHALL print the same as tab-separated fields. When the
account belongs to no store other than the global one, it SHALL say so on
stderr.

#### Scenario: Roles listed
- **WHEN** the account is an admin of `ce-team-test` and a viewer of `dsmr6`, and `dsmr6` is active
- **THEN** stdout lists both with their roles, `dsmr6` marked active

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
