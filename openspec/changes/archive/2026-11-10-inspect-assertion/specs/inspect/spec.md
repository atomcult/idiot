# Spec Delta

## ADDED Requirements

### Requirement: Assertion summary
`idiot inspect assertion [--remote] [--no-verify] <file|->` SHALL read an
assertion stream from the file, or from stdin when given `-`, and print on
stdout, for each assertion in the order they appear:
- its type, its primary key headers, `authority-id`, the brand or account
  it is about, `sign-key-sha3-384`, and its `timestamp` and `revision`
  when present;
- its remaining headers, a multi-line header summarised by its number of
  entries unless the type's summary below shows it;
- for `model`: each snap with its type, default channel, presence and id,
  and each validation set with its mode (a pre-UC20 model: its kernel,
  gadget and required snaps);
- for `account-key`: the public key's algorithm, size and SHA3-384, and
  its `since`/`until` validity;
- for `validation-set`: each snap with its presence and revision;
- for `system-user`: its models, `since`/`until` and the number of SSH
  keys, showing only whether a password is set and never the password
  hash;
- for `snap-declaration` and `snap-revision`: the snap's id, name,
  publisher or developer, and for a revision its number, size and
  SHA3-384;
- for any other type: its headers and its body's size.
It SHALL color that output only when stdout is a terminal and `NO_COLOR`
is unset. It SHALL fail on a missing file, on input with no assertion in
it, and on an assertion it can't parse (naming its position in the
stream). JSON input SHALL fail saying it is an unsigned model, not a
signed assertion. It SHALL need python3, from the host or the `core`
component.

#### Scenario: A signed model
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.model`
- **THEN** stdout shows `type: model`, brand `canonical`, model `ubuntu-core-24-amd64`, grade `signed`, its signing key, and a table of its snaps (pc, pc-kernel, core24, snapd, console-conf…) with type, default channel, presence and id

#### Scenario: From stdin
- **WHEN** the user runs `snap known account-key account-id=canonical | idiot inspect assertion -`
- **THEN** every account-key in the stream is summarised in turn, each with its name, key size and SHA3-384

#### Scenario: System user password
- **WHEN** the user inspects a system-user assertion that carries a password hash
- **THEN** the output says a password is set and contains no part of the hash

#### Scenario: Unsigned model
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.json`
- **THEN** the command fails saying the file is an unsigned model, not a signed assertion

#### Scenario: Not an assertion
- **WHEN** the user runs `idiot inspect assertion README.md`
- **THEN** the command fails saying no assertion was found

### Requirement: Assertion signatures
Unless given `--no-verify`, `idiot inspect assertion` SHALL report for
each assertion one of:
- `verified`, naming the trusted key the chain ends at, when its
  signature and each signature up the chain of account-keys verify and
  the chain ends at a key in this machine's snapd assertion database or
  at Canonical's root key, which idiot bundles together with Canonical's
  `models` key (itself checked against the root);
- `invalid`, saying why, when a signature in the chain doesn't match, a
  signing account-key's `account-id` isn't the signed assertion's
  `authority-id`, or the assertion's timestamp is outside the key's
  `since`/`until`;
- `unverified`, naming the missing key, when an account-key in the chain
  isn't in the stream, the local database, or (with `--remote`) the
  store.
It SHALL look up account-keys in the stream first, then in the local
database, then among the bundled keys, and in the store only with
`--remote`; without `--remote` it
SHALL make no network request. It SHALL NOT change the local assertion
database and SHALL NOT need root or store credentials. When `snap` isn't
available it SHALL use only the stream and the bundled keys, and say so
on stderr. The command SHALL exit 1, after printing every assertion,
when any assertion is `invalid`, and 0 otherwise.

#### Scenario: Canonical model, offline
- **WHEN** the user runs `idiot inspect assertion ubuntu-core-24-amd64.model` without `--remote`, on a classic host whose snapd has never seen Canonical's `models` key
- **THEN** the model is `verified` through the bundled `canonical/models` key, ending at `canonical/root`, and no network request is made

#### Scenario: Brand key not known locally
- **WHEN** the user inspects a model signed by a brand key this machine's snapd has never seen, without `--remote`
- **THEN** the model is `unverified`, naming the key's SHA3-384 and suggesting `--remote`, and the command exits zero

#### Scenario: Fetching the key
- **WHEN** the user runs the same command with `--remote` and the store has the brand's account-key
- **THEN** the key is fetched, its own signature checked up to `canonical/root`, and the model is `verified`

#### Scenario: Tampered assertion
- **WHEN** one header of a signed model has been edited after signing
- **THEN** the model is `invalid` (the signature doesn't match), every other assertion in the stream is still reported, and the command exits 1

#### Scenario: Skipping verification
- **WHEN** the user runs `idiot inspect assertion --no-verify pc.model`
- **THEN** the summary is printed with no signature status, `snap` isn't run, and the command exits zero

### Requirement: Assertion cross-checks
For a `model` assertion, `idiot inspect assertion` SHALL warn on stderr
when:
- its `store` header differs from the active store (`UBUNTU_STORE_ID`),
  or it names a store and no store is active, or a store is active and it
  names none; the warning SHALL name the saved store (`idiot store`) whose
  ID is the model's `store`, when there is one;
- a model with the same `brand-id` and `model` exists in a cloned model
  repository (`idiot model`) with a different revision or different
  content, naming that file and both revisions;
- its `authority-id` differs from its `brand-id`.
When a saved model is identical it SHALL say so on stderr, naming the
file. These checks SHALL NOT change the exit status.

#### Scenario: Wrong store active
- **WHEN** `UBUNTU_STORE_ID` is `acme-store-id` and the user inspects a model whose `store` header is `other-store-id`, saved with `idiot store add other other-store-id`
- **THEN** stderr warns the model's store is `other-store-id` (saved as `other`) while `acme-store-id` is active, and the command still exits zero

#### Scenario: Saved model is newer
- **WHEN** the user inspects revision 2 of a model and the canonical clone holds revision 3 of the same brand and model
- **THEN** stderr warns that the saved model differs, naming its file and revision 3

#### Scenario: Matches a saved model
- **WHEN** the user inspects a file copied with `idiot model pick --signed`
- **THEN** stderr says it matches the saved model, naming the file
