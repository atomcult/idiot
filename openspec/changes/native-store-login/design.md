# Design

## Context

idiot-v2 (`~/work/idiot-v2`, crate `idiot-store`, ~550 lines of Rust)
implements:
- `store::login`: POST `dashboard.snapcraft.io/dev/api/acl/` →
  root macaroon; extract the `login.ubuntu.com` third-party caveat; POST
  `login.ubuntu.com/api/v2/tokens/discharge` with email/password/caveat
  (+ `otp`), mapping `TWOFACTOR_REQUIRED` to a typed error; return
  `base64({"t":"u1-macaroon","v":{"r":root,"d":discharge}})`, the format
  snapcraft's `export-login` writes.
- `store::whoami`: decode that, bind the discharge to the root, GET
  `dashboard.snapcraft.io/api/v2/tokens/whoami` with
  `Authorization: Macaroon root=…, discharge=…`.

Gaps found in review: permissions/expiry hard-coded (expiry
`2026-02-22`, now past), user agent impersonates snapcraft 8.6.3, `whoami`
pads its `key:` columns (idiot's status parser expects `key: value`),
`parse_credential` unused, async tokio for two sequential requests. Its
directory also holds an untracked live credential (`creds`, mode 0664) that
must not travel with the code.

## Goals / Non-Goals

**Goals:** drop snapcraft from `idiot auth`; scoped credentials; keep the
credential format; keep the POSIX sh core in charge.

**Non-Goals:**
- Rewriting idiot in Rust, or moving any non-store logic into the helper.
- Store operations beyond login/whoami (push, release, brand-store
  admin). The helper is shaped so they can be added later.
- Macaroon refresh or revocation.

## Decisions

### A Rust helper rather than sh + python3
Chosen with Lauren. The flow needs macaroon (de)serialisation and discharge
binding (HMAC-SHA256 over macaroon signatures): v2 has this working and
tested against the real store; redoing it in Python would mean hand-rolling
the macaroon V1 format with no library, and python3 is only bundled in the
optional `core` component. The helper is a leaf tool: sh owns UX (pickers,
names, files, caching, messages), the binary only talks to the store.

### Layout and lookup
- Crate at `tools/idiot-store/` (Cargo workspace of one), binary
  `idiot-store`.
- `make install` builds it with `cargo build --release --locked` and
  installs it to `$(LIBDIR)/libexec/idiot-store`; the snap's rust part
  stages it at the same relative path, so `$IDIOT_ROOT/libexec/idiot-store`
  works for both.
- `share/creds.sh` gets `store_helper` which resolves that path, falling
  back to `idiot-store` on PATH for a development checkout, and dies with
  "build it with: make install" otherwise.
- `make install` fails with a clear message when cargo is missing (the
  helper is required, not optional).

### Helper interface (stable, sh-friendly)
- `idiot-store login --output <file> [--permissions a,b] [--snaps x,y]
  [--channels c,d] [--expires <rfc3339>] [--description <text>]`
  - Prompts on `/dev/tty`: email (echoed), password and OTP (not echoed).
  - Creates `<file>` with `O_CREAT|O_EXCL`, mode 0600, and writes the
    credential there. Never writes it to stdout or stderr.
  - Exit codes: 0 ok, 2 authentication failed (store message on stderr),
    3 network/store error, 1 usage.
- `idiot-store whoami` reads the credential from `UBUNTU_STORE_AUTH` (or
  `--file`) and prints exactly snapcraft's lines (`email: …`,
  `username: …`, `id: …`, `permissions: a, b`, `channels: …`,
  `expires: …`) to stdout; 2 on a rejected credential.
- Credentials are never passed in argv (visible in `ps`): environment or
  file only.

### Shell side
- `auth add` parses the new options; validates name, permissions (against
  the list `auth status` already maps), and expiry before prompting;
  converts `Nd` to RFC 3339 with `date -u -d "+N days"`; `--permissions`
  with no value opens `fzf --multi` over the permission list. It runs the
  helper with `--output` pointing at a `mktemp` path in the auth dir (same
  filesystem, 0600), then renames into place.
- `auth status` swaps `snapcraft whoami` for `"$(store_helper)" whoami`;
  parsing, caching and the expired-credential path (no cache on failure)
  stay as they are.
- `--description` defaults to `idiot@<hostname>`, which is what the store
  dashboard shows for the credential.

### Rust changes while porting
- `reqwest` blocking client with `rustls-tls`, no tokio: two sequential
  requests don't need an async runtime, and rustls avoids linking the
  host's libssl (matters for the classic snap and older hosts).
- User agent `idiot-store/<version>` instead of impersonating snapcraft.
  If the store rejects unknown agents (task 2.3 checks), fall back to a
  snapcraft-style agent and record why in a comment.
- Expiry, permissions, packages and channels come from arguments; typed
  `StoreError` kept; `clap` derive kept.
- Unit tests for credential encode/decode, caveat extraction, and 2FA/error
  JSON parsing (`two_factor_required`, `extract_error_message`) using
  recorded response bodies, so `cargo test` needs no network.

### Snap
A `idiot-store` part with `plugin: rust`, `source: tools/idiot-store`,
organised into `lib/idiot/libexec/`. Being a classic snap, the binary
gets `build-attributes: [enable-patchelf]` so it uses core24's loader
and libc instead of the host's.

## Risks / Trade-offs

- [Unofficial use of store endpoints] → they are the ones snapcraft
  (craft-store) uses; isolate URLs as constants and keep the compatibility
  scenario (snapcraft accepts idiot credentials) as the regression check.
- [Rust toolchain needed for `make install`] → accepted; clear error, and
  the snap needs nothing on the host.
- [Credential briefly in a temp file] → created 0600 in the 0700 auth dir
  and renamed atomically; deleted on failure via the existing trap.
- [Store rejects the non-snapcraft user agent] → checked live in task 2.3;
  documented fallback.

## Migration Plan

Existing credentials (made by snapcraft) keep working unchanged. After
this lands, `snapcraft` can be uninstalled as far as idiot is concerned.
Rollback: revert; the credential files are the same format.

## Open Questions

- Should the permission picker also offer non-`package_*` permissions
  (`store_admin`, `account_admin`, …)? Default list follows snapcraft;
  the picker can be widened later without changing the specs.
