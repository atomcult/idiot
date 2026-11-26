# Proposal

## Why

Credentials made by `idiot auth add` often don't work with snapd's image
tooling (`ubuntu-image`, `snap prepare-image`, `snap download`). Downloads
of private or brand-store snaps fail with a 404 even though
`snapcraft whoami` and `idiot auth status` show the right account.

The cause is in idiot-store. It doesn't keep the macaroon strings the Snap
Store and Ubuntu One send. It parses them and writes them out again with
the `macaroon` crate, whose V1 format pads its base64 with `=`. snapd
decodes macaroons as unpadded URL-safe base64 and rejects the padding. When
it can't authorise a request, it only logs that at debug level and sends the
request anonymously, so the store hides the snap. snapcraft and idiot-store
accept padding, so nothing else notices. A macaroon is padded whenever its
length isn't a multiple of 3 bytes, so most credentials idiot creates are
affected. This breaks the auth spec's promise that idiot credentials are in
the format `snapcraft export-login` writes. It was found reproducing a 404
on `ce-dummy` from the `ce-team-test` store, and confirmed by a saved
credential whose root macaroon ends in `=`.

## What Changes

- **New credentials:** `idiot auth add` saves the macaroons exactly as the
  store and Ubuntu One sent them, as `snapcraft export-login` does, and
  never writes base64 padding.
- **Existing credentials** are repaired in place when they are activated
  (`idiot auth login`, `idiot profile use`) or exported (`idiot auth
  export`). Removing padding is lossless: it carries no data. The repair
  says so on stderr, once per credential.
- **`idiot doctor`** reports `!` for each saved credential that still has a
  padded macaroon, saying snapd can't use it and that activating it repairs
  it. A shell that activated a credential before its repair still holds the
  padded value; doctor's existing "active credential differs from the
  saved one" check already catches that and says to run `idiot auth login`.
- **idiot-store** gains `repair <file>`, which rewrites a credential file
  in place (staying private, mode 0600). With `--check` it changes nothing.
  It prints only a status word on stdout (`ok`, `repaired`, or with
  `--check` `padded`) and never the credential.

Output: `auth login`, `profile use` and `auth export` may print one extra
line on stderr when they repair a credential. What they queue in the eval
file is unchanged in kind (the same exports), but the value is now the
repaired credential. `auth export` stdout is the repaired credential.
There are no new runtime dependencies and nothing new for the snap to stage:
idiot-store is already in the snap. `auth login` and `profile use` stay
confined, since the repair only touches idiot's own files with a bundled
binary.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `auth`: credential format compatibility now covers snapd's image tooling
  (`UBUNTU_STORE_AUTH`) and forbids padding; activating or exporting a
  credential repairs a padded one.
- `doctor`: the credential checks flag credentials with padded macaroons.

## Impact

- `tools/idiot-store`: `store/login.rs` keeps the raw macaroon strings;
  `store/credential.rs` encodes without padding and gains the repair; new
  `cmd/repair.rs` and a `Repair` subcommand in `main.rs`; unit tests.
- `share/creds.sh`: `emit_cred_login` repairs before exporting (skipped,
  with a warning, when idiot-store is missing); `cmd/auth/export` repairs
  first.
- `cmd/doctor`: the padded-credential check (through idiot-store
  `repair --check`, which is offline).
- `tests/bare-host.d/`: a padded credential is repaired on login and
  flagged by doctor beforehand.
- README troubleshooting note and the using-idiot skill (`references/`).
