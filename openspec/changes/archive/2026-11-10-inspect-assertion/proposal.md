# Proposal

## Why

Model, serial, snap-declaration and system-user assertions are what Ubuntu
Core images and devices are built and judged on, yet reading one means
squinting at raw headers, and checking it is signed by the key it claims
means `snap ack` (root, and it changes the system's assertion database)
or nothing. A model picked with `idiot model pick` can also silently
disagree with the store the shell is using (`idiot store use`), which only
shows up when an image build or a device registration fails.

## What Changes

- New `idiot inspect assertion [--remote] [--no-verify] <file|->`: reads
  an assertion stream (a file, or stdin with `-`) and prints, for each
  assertion in it, on **stdout**:
  - its type and primary key, `authority-id`, the brand or account,
    `sign-key-sha3-384`, `timestamp`/`revision`, and its other headers;
  - a type-specific summary: for `model`, a table of its snaps (name,
    type, default channel, presence, id) and its validation sets; for
    `account-key`, the key's algorithm, size, validity and SHA3-384; for
    `validation-set`, its snaps; for `system-user`, its models and
    validity, never its password hash; for `snap-declaration` and
    `snap-revision`, the snap's id, name, revision, size and digest;
    any other type gets its headers and its body's size;
  - whether its signature verifies, up to a key snapd already trusts
    (this machine's assertion database, read with `snap known`, or
    Canonical's root key, bundled), using the account-keys in the stream
    itself, those in the local database, Canonical's `models` key
    (bundled, as a classic host's snapd doesn't have it) and, with
    `--remote`, the store's (`snap known --remote`, no credentials).
    `verified`, `unverified` (a key in the chain can't be found) or
    `invalid` (a signature doesn't match, or the key isn't the
    authority's or wasn't valid at the assertion's timestamp).
- Cross-checks, warned about on stderr: a model whose `store` header
  isn't the active store (`UBUNTU_STORE_ID`), naming the saved store
  (`idiot store`) that has that ID, if any; a model whose brand and name
  match a saved model (`idiot model` clones) with a different revision or
  content, naming that file; a model whose `authority-id` isn't its
  `brand-id`.
- An unsigned model (JSON, from `idiot model pick --unsigned`) fails
  saying it isn't a signed assertion.
- Exit status: 0 when every assertion verified or couldn't be checked;
  1 after printing when any is `invalid`, and on unreadable input.
- `share/common.sh` gains `stdout_colors` (shared with the
  `inspect-image-diff` change).

Output channels: the new command prints its report on stdout and
warnings on stderr. No existing command's output changes. Nothing is
queued in the eval file. New runtime dependencies: none required;
python3 (already required by `inspect image`, from the host or the `core`
component) does the parsing and RSA verification with its standard
library, and the host's `snap` is used when present (the snap needs to
stage nothing new: a host command runs with the host's `snap`). New
bundled data: Canonical's `root` and `models` account-keys (public).
`inspect assertion` is a host command: it reads the user's files and
runs `snap known`.
Network access only with `--remote`.

## Capabilities

### New Capabilities

### Modified Capabilities
- `inspect`: new requirements for summarising an assertion stream,
  verifying its signatures, and cross-checking a model against idiot's
  stores and saved models.

## Impact

New `cmd/inspect/assertion`, `share/assertion.py` (parser, summary,
verifier; standard library only, like `share/uefi-vars.py`),
`share/assertions/canonical.account-keys`; `cmd/inspect/help`;
`share/common.sh` (`stdout_colors`); README's inspect section.
