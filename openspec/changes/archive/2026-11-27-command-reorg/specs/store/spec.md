## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: Store roles
**Reason**: Folded into `idiot store list --account`, so saved stores and the account's stores are listed by one command.
**Migration**: Run `idiot store list --account` (`--tsv` as before). `idiot store roles` keeps working as an alias that says so.
