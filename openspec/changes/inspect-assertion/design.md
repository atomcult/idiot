# Design

## Context

Assertions are text: `key: value` headers (lists and maps as indented
continuation lines), an optional body whose size is the `body-length`
header, then a blank line and the signature. A stream is assertions one
after another (`snap known` prints several; `.assert` bundles carry the
account-keys a model needs). The signature is base64 of a format byte
(`0x01`) and an OpenPGP v4 signature packet (RSA, SHA-512) over
everything before the blank line that precedes it. An `account-key`'s body
is base64 of `0x01` and an OpenPGP v4 public-key packet, and its
`public-key-sha3-384` is the unpadded URL-safe base64 SHA3-384 of those
decoded bytes.

A stdlib-only python3 prototype (in the planning scratch space, ~50 lines:
packet parsing, `pow(s, e, n)`, `hashlib.sha512`/`sha3_384`) verified the
local `generic-classic` model against the `generic/models` key, and that
key's computed SHA3-384 matched its header; it also verified the
canonical clone's `ubuntu-core-24-amd64.model` against `canonical/models`
(`9tydnLa6…`) and rejected a copy with `grade` edited. Canonical's `root`
key signs itself and `canonical/store`, `generic/models` etc.; `snap
known` on this machine lists them (snapd's built-in trusted roots are
included), but **not** `canonical/models`, which signs every Canonical
model: a classic host's snapd never needs it, so it had to be fetched
with `snap known --remote`.

What idiot already has: `share/uefi-vars.py`, a stdlib python3 reader run
by `cmd/vm` with its listing on stdout (the precedent for this command);
`share/stores.sh` (`IDIOT_STORES_DIR`, one file per saved store holding
its ID; the active store is `UBUNTU_STORE_ID`); `share/models.sh`
(`IDIOT_MODELS_DIR/<repo>` clones, `*.model` files); `tools/idiot-store`,
a Rust client for store login/whoami that only talks to the store with a
macaroon. See proposal.md for why; the inspect spec delta for what is
printed.

## Goals / Non-Goals

**Goals:**
- Read and verify any assertion stream offline, without root and without
  changing snapd's database.
- Keep sh in charge of UX and idiot state; one stdlib python3 file does
  the parsing and cryptography.

**Non-Goals:**
- Signing, or producing assertions (`snap sign`, `snapcraft`).
- Revocation beyond the account-key's `until` header, and checking an
  assertion against the store's current revision of it.
- Checking a `snap-revision` against a snap file's digest (a possible
  `--snap <file>` later), or a `serial` against a device.
- Interpreting `snap-declaration` plug/slot rules beyond naming them.
- Non-RSA keys: snapd only uses RSA today; anything else reports
  `unverified` (unsupported key algorithm), not `invalid`.

## Decisions

### Parsing and verification in python3 (stdlib), not the Rust helper or `snap`
- **`snap ack`** verifies, but needs root, adds the assertion (and keys)
  to the system database, and stops at the first failure with snapd's
  message: wrong for an inspection command. `snap` has no verify-only
  command, and `snap known` only reads what is already in the database.
- **The Rust helper** would need `rsa`, `sha2`, `sha3` and an OpenPGP
  packet parser as new crates, and would stop being the store-only leaf
  tool it was introduced as. Nothing here needs store credentials (the
  one network step, fetching public account-keys, is `snap known
  --remote`, unauthenticated), which was the reason Rust was chosen for
  login.
- **python3 stdlib** does it: RSA PKCS#1 v1.5 verification is a modular
  exponentiation and a byte comparison, SHA-512 and SHA3-384 are in
  `hashlib`, and the prototype works. python3 is already required by
  `inspect image` (host or `core` component). Verification handles only
  public data, so a non-constant-time implementation is fine; the padded
  block is compared in full against the expected encoding
  (`00 01 FF… 00 || DigestInfo(SHA-512) || digest`), never by suffix.
Alternative considered: `gpg --verify` after re-wrapping the packets.
Rejected: gpg needs a keyring import (state outside idiot) and isn't
reliably installed.

### `share/assertion.py`
Usage: `python3 assertion.py [--remote] [--no-verify] [--snap <path>]
[--trusted <file>] [--facts <file>] <file|->`, standard library only,
header comment as in `uefi-vars.py`.
- Parses the stream into assertions (headers, body, signature, and the
  signed content bytes).
- Prints the report to stdout, coloring with the `OUT_*` values passed in
  the environment (as `cmd/inspect/image` passes `CYAN` etc. to its
  python), which `stdout_colors` sets from `[ -t 1 ]`/`NO_COLOR`.
- Verification: for an assertion signed by key K, find K's account-key
  in (1) the stream, (2) `snap known account-key public-key-sha3-384=K`
  via `--snap`, (3) with `--remote`, `snap known --remote account-key
  public-key-sha3-384=K`, else (4) the bundled keys (`--trusted`).
  Check the key's computed SHA3-384 equals K, its `account-id` is the
  assertion's `authority-id`, the assertion's `timestamp` is within
  `since`/`until`, and the signature. A key from the local database or
  the bundled root ends the chain (snapd verified what it stored); a key
  from the stream, the store or the bundle (other than the root) is
  itself verified the same way, at most
  four levels deep, keys cached per run. Results: `verified (<account>/<name>)`,
  `unverified: <reason>`, `invalid: <reason>`.
- `--facts` writes one tab-separated line per model for the shell's
  cross-checks: index, `brand-id`, `model`, `authority-id`, `store`,
  `revision` (default 0), and the SHA3-384 of the assertion's bytes.
- Exit 0, 1 when any assertion is `invalid`, 2 on unreadable input or a
  parse error (message on stderr via `sys.exit("…")`, prefixed the way
  `die` prefixes, without color).
- System-user `password` is printed as `password: set`; the value is
  never written anywhere.

### `cmd/inspect/assertion` (sh) owns state and cross-checks
- `# idiot: host` (reads an arbitrary user file; runs `snap`). `-` is
  passed through: under the snap the confined stage doesn't read stdin, so
  the handed-off host command gets the user's pipe (verified in tasks).
- `require python3`; `snap` is optional (`command -v snap`): without it,
  says on stderr that only the stream and Canonical's root key are used.
- JSON input (first non-space byte `{`) fails before python: "unsigned
  model (JSON), not a signed assertion — sign it with snap sign".
- Runs `assertion.py` with `--facts` into a `mktemp` file, then for each
  model:
  - store: compare with `UBUNTU_STORE_ID`; find the saved store whose
    file content is the model's `store` (`grep -lx` over
    `IDIOT_STORES_DIR`) to name it;
  - saved models: scan `IDIOT_MODELS_DIR/*/` `*.model` files for the same
    `brand-id`/`model` headers (awk, first match per file); `cmp` for
    identical, else compare `revision`;
  - `authority-id` ≠ `brand-id`: note it.
  All of this is `say "warning: …"` on stderr; the exit status is the
  python's (2 mapped to 1).

### Bundled Canonical keys
`share/assertions/canonical.account-keys`: the public `account-key`
assertions for `canonical/root` (`-CvQKAwRQ5h3…`, the anchor) and
`canonical/models` (`9tydnLa6…`, verified against the root at each run
like any other non-anchor key), copied from `snap known --remote`. The
root is the anchor when `snap` is missing (a source install on a host
without snapd); the models key makes every Canonical model verify offline
on a classic host. Both are public: no credential handling. If Canonical
rotates the models key, models signed with the new one fall back to
`unverified` offline (or `--remote`) until the bundle is refreshed; the
bundled root only anchors Canonical-rooted chains.

### Output layout
```
type:          model
brand / model: canonical / ubuntu-core-24-amd64
authority-id:  canonical
sign-key:      9tydnLa6MTJ-jaQTFUXEwHl1yRx7ZS4K5cyFDhYDcPzhS7uyEkDxdUjg9g08BtNn
timestamp:     2024-…   revision: 0
grade: signed  base: core24  architecture: amd64
snaps:
  pc          gadget  24/stable  required  UqFziVZDHLSyO3TqSWgNBoAdHbLI4dAH
  pc-kernel   kernel  24/stable  required  pYVQrBcKmBa0mZ4CCN7ExT6jH8rY1hza
  …
signature:     verified (canonical/root)
```
Assertions in a stream are separated by a blank line.

### stdout colors
`stdout_colors` in `share/common.sh` as specified in the
`inspect-image-diff` design (`OUT_*`, `-t 1` and `NO_COLOR` only);
whichever change lands second reuses it.

## Risks / Trade-offs

- [Hand-written crypto has a bug that reports `verified` wrongly] →
  Tests against real chains (Canonical models, a brand model with
  `--remote`), a tampered header, a tampered signature byte, a key used
  outside `since`/`until`, a key whose `account-id` isn't the authority;
  full-block padding comparison. The report is advisory (idiot doesn't
  install or trust anything on its basis), and says what it checked.
- [Local database contains keys the user `snap ack`ed] → snapd verified
  them on ack; treating them as anchors is what snapd itself does.
- [`snap known --remote` is slow or offline] → only with `--remote`; a
  failure leaves the assertion `unverified`, naming the reason.
- [Header format corner cases (maps in lists, `body-length: 0`, CRLF)] →
  Parser follows snapd's format (2-space indented continuation lines);
  test with model (snaps list of maps), system-user (`models` list),
  validation-set and account-key streams from `snap known`.
- [Scanning every `*.model` in the clones is slow for big repos] → one
  awk over the file list, reading only the first header lines; the
  canonical clone is a few hundred files.

## Assumptions

- The report goes to stdout (the data asked for, like `vm vars`), even
  though `inspect image` and `inspect snap` report on stderr.
- `unverified` exits 0; only `invalid` exits 1 (scripts that need
  `verified` can grep for it). A `--strict` flag can come later.
- Anchors are snapd's local database plus the bundled Canonical root
  (with the models key bundled for offline use); no idiot-managed trust
  store.
- Store cross-checks compare against `UBUNTU_STORE_ID` only (what
  `idiot store use` sets), not snapd's device store.
- `--remote` uses `snap known --remote` (which goes via snapd when it
  can); no direct HTTP client is added.
