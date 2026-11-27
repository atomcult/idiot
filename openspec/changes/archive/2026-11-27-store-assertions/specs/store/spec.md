## ADDED Requirements

### Requirement: Fetching an assertion from the store
`idiot store assertion <type> [<key>...]` SHALL fetch one assertion from
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
- `snap-revision <sha3-384>`, given in hex or base64url;
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
- **WHEN** the global store is active and the user runs `idiot store assertion snap-declaration hello-world`
- **THEN** the `snap-declaration` with series `16` and snap-id `buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ` is fetched and summarised, showing snap name `hello-world` and publisher `canonical`

#### Scenario: A revision by number
- **WHEN** the user runs `idiot store assertion snap-revision hello-world 29`
- **THEN** the `snap-revision` for revision 29's SHA3-384 is fetched and the summary shows revision 29, its size and its SHA3-384

#### Scenario: A revision by hash
- **WHEN** the user runs `idiot store assertion snap-revision` with the hex SHA3-384 of a snap the store has
- **THEN** the same `snap-revision` is fetched as with its base64url form

#### Scenario: Latest validation set
- **WHEN** the user runs `idiot store assertion validation-set acme/base-set` and the store holds sequences 1 to 3
- **THEN** sequence 3 is fetched

#### Scenario: Any type by its primary key
- **WHEN** the user runs `idiot store assertion serial brand-id=acme model=gw serial=1234`
- **THEN** that `serial` assertion is requested by its primary key

#### Scenario: Missing key
- **WHEN** the user runs `idiot store assertion model canonical`
- **THEN** the command fails saying a model is fetched by `<brand-id>/<model>`, and contacts nothing

#### Scenario: Unknown type
- **WHEN** the user runs `idiot store assertion nope x`
- **THEN** the command fails listing the assertion types it knows, and contacts nothing

#### Scenario: Not in the store
- **WHEN** the user runs `idiot store assertion account no-such-account`
- **THEN** the command fails saying the store has no `account` `no-such-account`, or doesn't show it to this credential

#### Scenario: No store active
- **WHEN** no store is active and the user runs `idiot store assertion store`
- **THEN** the command fails saying no store is active and to name a store ID, and contacts nothing

#### Scenario: Anonymous fetch
- **WHEN** no credential is active and the user runs `idiot store assertion account canonical`
- **THEN** Canonical's `account` assertion is fetched without credentials and summarised

#### Scenario: Unknown credential
- **WHEN** the user runs `idiot store assertion account canonical --cred nope` and no credential `nope` is saved
- **THEN** the command fails with `credentials 'nope' not found` and contacts nothing

### Requirement: Fetched assertion output
By default `idiot store assertion` SHALL print on stdout the summary and
signature status `idiot inspect assertion` prints for the fetched stream,
with the same model cross-checks on stderr and the same exit status, and
`--no-verify` SHALL skip the signature check as it does there. With
`--raw` it SHALL print the assertion stream, and nothing else, on stdout:
each assertion byte for byte as the store served it, separated by a blank
line as `snap known` prints them, so it can be piped to `snap ack /dev/stdin`
or `idiot inspect assertion -`. With `-o <file>` it SHALL write that stream
to the file, replacing it only once the whole stream has been fetched,
print nothing on stdout, and say on stderr how many assertions it wrote
and where. `--raw` and `-o` SHALL NOT both be given. Messages and progress
SHALL go to stderr.

#### Scenario: Human-readable
- **WHEN** the user runs `idiot store assertion account canonical`
- **THEN** stdout shows `type: account`, account `canonical`, its signing key, and its signature status, as `idiot inspect assertion` would for the same assertion

#### Scenario: Raw to stdout
- **WHEN** the user runs `idiot store assertion --raw account-key BWDEoaqyr25nF5SNCvEv2v7QnM9QsfCc0PBMYD_i2NGSQ32EF2d4D0hqUel3m8ul | head -1`
- **THEN** the line printed is `type: account-key`

#### Scenario: To a file
- **WHEN** the user runs `idiot store assertion --chain -o hello.assert snap-revision hello-world 29`
- **THEN** `hello.assert` holds the stream, stdout is empty, and stderr says how many assertions were written to `hello.assert`

#### Scenario: Failed fetch leaves the file alone
- **WHEN** `hello.assert` exists and `idiot store assertion -o hello.assert account no-such-account` fails
- **THEN** `hello.assert` is unchanged

### Requirement: Prerequisite chain
With `--chain`, `idiot store assertion` SHALL fetch, besides the assertion
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
- **WHEN** the user runs `idiot store assertion --chain --raw snap-revision hello-world 29 > hello.assert`
- **THEN** `hello.assert` holds Canonical's `store` account-key, then the `snap-declaration` for `hello-world`, then the `snap-revision` for revision 29, each once, and neither Canonical's `account` nor its root key

#### Scenario: Verifies offline
- **WHEN** the user runs `idiot store assertion --chain model acme/gw` for a brand whose key this machine's snapd has never seen
- **THEN** the summary shows the brand's `account-key` and `account` before the model, and the model is `verified` without any further network request

#### Scenario: Accepted by snapd
- **WHEN** the user runs `idiot store assertion --chain --raw snap-revision hello-world 29 | sudo snap ack /dev/stdin`
- **THEN** snapd accepts every assertion in the stream
