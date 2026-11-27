## ADDED Requirements

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
