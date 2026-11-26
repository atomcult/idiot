# Design

## Context

A credential is `base64({"t":"u1-macaroon","v":{"r":<root>,"d":<discharge>}})`,
with each macaroon in V1 serialisation (URL-safe base64 of the binary
macaroon). `Login::start` (`tools/idiot-store/src/store/login.rs`) parses
the ACL endpoint's `macaroon` into a `macaroon::Macaroon` (to find the SSO
caveat ID), and `Login::finish` parses SSO's `discharge_macaroon` the same
way. `credential::encode` then serialises both with `Format::V1`. In
macaroon 0.3.0 that is `base64::encode_config(.., URL_SAFE)` (base64
0.13), and `URL_SAFE` pads.

snapd's `auth.MacaroonDeserialize` uses `base64.RawURLEncoding`, which
rejects `=`. `store.go` treats a failed `Authorize` as a debug log and sends
the request without authorisation, so a private snap's download is a 404.
pymacaroons (snapcraft) and the Rust crate accept padding either way, so
idiot's `whoami`, `expiry` and `auth_header` all keep working.

This was confirmed on a real install: the snap-side credential written by
idiot-store has a padded root macaroon, while an older one from
`snapcraft export-login` has none.

## Goals / Non-Goals

**Goals:**
- New credentials are byte-for-byte what the store and Ubuntu One sent, as
  with `snapcraft export-login`.
- Existing padded credentials get fixed without the user logging in again,
  at the moment they would be used.
- Nothing prints, logs or passes a credential on a command line in the
  process.

**Non-Goals:**
- Patching or replacing the `macaroon` crate. idiot only needs it for
  parsing (SSO caveat ID, expiry caveats, binding the discharge for
  `whoami`), and parsing already accepts both forms.
- Repairing credentials idiot didn't write in other places (snapcraft's
  keyring, an exported file elsewhere).
- Making snapd report the failed authorisation.

## Decisions

**Keep the raw strings.** `Login` keeps the ACL response's `macaroon`
string and SSO's `discharge_macaroon` string, and `encode` takes strings
instead of `Macaroon`s. The parsed root is still used to find the caveat ID.
As a guard, `encode` strips any trailing `=` from both strings, so even a
padded string from the server can't produce a credential snapd rejects.
Unit test: a padded input produces an unpadded credential that still
decodes to the same macaroons.

**Repair in idiot-store, not sh.** It needs base64 and JSON on a
credential, which sh would have to put in a pipeline or variable. One more
helper subcommand, `idiot-store repair [--check] <file>`, keeps the
existing rules: no credential in argv or on stdout, and status on stdout
only (`ok`, `repaired`, `padded`). It rewrites atomically: a temporary file
in the same directory, created 0600, then renamed over the original. Exit
codes follow the existing scheme: 0 on success, 1 for usage, and the local
failure code for an unreadable file or something that isn't a `u1-macaroon`
credential (that one is left untouched). Only trailing `=` on `r` and `d` is
removed. The JSON is re-encoded with the same two keys, so the credential is
otherwise identical.

**Repair where a credential is used.** `emit_cred_login` (used by
`auth login` and `profile use`) and `cmd/auth/export` run the repair first,
then read the file. These are the paths by which a credential reaches
snapd. A missing helper means a warning (`could not check credential
'<name>'`), and the credential is used as saved: never a failure, since
logging in must keep working. `auth login` and `profile use` are confined;
the repair runs the snap's own idiot-store on idiot's own files, with no
network, so they stay confined. `auth import` stays a plain copy (the spec
says so), and the file is repaired on first use.

**Doctor stays read-only.** It runs `repair --check` for each saved
credential (the helper is already located for its idiot-store check) and
reports `!` with `idiot auth login <name>` as the fix. Other shells still
holding the padded value are caught by the existing "differs from the
saved one" check.

## Risks / Trade-offs

- **The store or SSO might someday send padded strings.** The `encode`
  guard strips them.
- **A repaired file under a running shell.** The shell still holds the
  padded value until `idiot auth login` runs again. Doctor reports it, and
  it only matters to snapd tooling, which was already failing.
- **A cost on every login.** One extra helper process per `auth login` or
  `profile use`, a few milliseconds. This could be avoided by checking with
  `grep` first, but that would mean decoding the credential in sh, which
  the design avoids.
- **Downgrades.** An older idiot reads repaired credentials fine: the crate
  accepts both forms.
