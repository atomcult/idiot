# Design

## Context

See proposal.md for motivation. Where expiry is known today:

- **At creation**: `cmd/auth/add` turns `--expires` into an RFC 3339 time
  and passes it to `idiot-store login`, which sends it to the store's ACL
  endpoint (`Restrictions.expires`). The store bakes it into the root
  macaroon; the shell prints `expires <date>` once and keeps nothing.
- **In the credential**: the credential is
  `base64({"t":"u1-macaroon","v":{"r":<root>,"d":<discharge>}})`
  (`src/store/credential.rs`). The root macaroon's first-party caveats are
  expected to include `dashboard.snapcraft.io|expires|<time>`, and the SSO
  discharge's may include `login.ubuntu.com|expires|<time>` (unverified,
  see Assumptions). The `macaroon` crate already in use exposes
  `first_party_caveats()` and each caveat's `predicate()`; nothing reads
  them yet.
- **From the store**: `idiot-store whoami` reports `expires:` (the store's
  value with a `Z` appended); `auth status` caches that report in
  `IDIOT_CACHE_DIR/whoami/<name>`. This needs the network once per
  credential and is a host command.
- **Imported credentials** (`auth import`, a `snapcraft export-login`
  file): only the caveats and the store know.

Constraints: the helper runs host-side only (the snap's `idiot-store`
part is patched to core24's loader for the host and is used only by host
commands), so confined commands (`auth login`, `auth list`, `status`)
cannot run it. They can read idiot's own state. Credentials are never
passed in argv or printed (helper contract: env or file in, `0600` files
out).

## Goals / Non-Goals

**Goals:**
- Expiry known offline for every credential, including imported ones and,
  after one host-side run, credentials saved before this change.
- Visible in `status`, `auth list`, `auth status` and at login; a warning
  shortly before expiry that does not nag.

**Non-Goals:**
- Renewing or re-issuing credentials (discharge refresh). The warning
  points at `idiot auth add`.
- Verifying signatures or checking the credential against the store
  beyond what `auth status` already does.
- Expiry in the prompt segment (`shell-prompt-segment`); it can add a
  field reading `IDIOT_CRED_EXPIRES` later.

## Decisions

### Read caveats in the Rust helper, not in sh
`idiot-store expiry [--file <file>]` decodes the credential the way
`credential::decode` already does, deserialises both macaroons, and
prints, one per line, `root <time>` / `discharge <time>` for every
first-party caveat whose predicate is `<location>|expires|<time>`. A time
without a zone gets `Z` appended (the store's times are UTC, as
`whoami.rs` already assumes). No caveat → no output, exit 0. Not a
credential → `invalid credential format`, exit 2 (as `whoami`). It makes
no network request and never prints anything but those lines; caveat
predicates other than `expires` are not printed either, since some (the
account caveat) identify the user.

Why Rust: the macaroons inside are V1 binary packets in base64url; sh
would need `base64 -d` plus python3 to walk the packets, python3 is only
bundled in the optional `core` component, and every intermediate (decoded
JSON, packets) would contain signatures and pass through pipes or temp
files. The helper already has the decoder and its tests; this adds about
40 lines and no crates. Time arithmetic stays in sh (`date -u -d`), so the
helper needs no date library: sh converts each printed time to Unix time
and keeps the smallest.

Alternative: rely on `whoami` alone. Rejected as the primary source: it
needs the network, and it is what fails once the credential has expired.
It stays the fallback when the caveats carry no expiry.

### The record: `IDIOT_AUTH_DIR/.expires.<name>`
One line, the expiry as Unix time (sh compares integers; display converts
with `date -u -d @<t>`). In the auth directory (0700, files 0600 via
`ensure_auth_dir`'s umask) because it describes the credential and must
survive a cache wipe: under the snap only host commands can recompute it.
The leading dot keeps it out of `auth list` (`"${IDIOT_AUTH_DIR}"/*`) and
pickers (`fzf_pick_from_dir` skips `.*`), matching the `.store.<name>`
record `add-env-profiles` plans. Credential names cannot start with `.`
(`validate_cred_name`), so no collision.

Writers (helpers in `share/creds.sh`: `expiry_record_path`,
`record_expiry <name>`, `read_expiry <name>`):
- `auth add`: after the rename, `record_expiry` (helper `expiry --file`);
  if that yields nothing, the `--expires` value it asked for.
- `auth import`: `record_expiry`; on failure a dim note
  `expiry unknown` and the import still succeeds.
- `auth status` (host): if the active named credential has no record,
  `record_expiry` from the file; if the caveats have no expiry, the
  `expires:` line of the whoami report it just fetched or read from cache.
- `auth login`: if there is no record and `SNAP` is unset (source
  install, helper runnable), `record_expiry`. Under the snap a missing
  record stays missing and login says
  `expiry unknown — run: idiot auth status` (dim, once).
- `auth remove`: deletes the record and the warning stamp.

### `IDIOT_CRED_EXPIRES` on login
`auth login` exports the recorded Unix time as `IDIOT_CRED_EXPIRES` (or
unsets it), `auth logout` unsets it. The per-command warning check and
`status` then need no file read in the common case. `status` falls back to
the record file when `IDIOT_CRED_EXPIRES` is unset (a shell logged in
before this change).

### Warning: in the dispatcher, once per credential per day
`idiot` calls `cred_expiry_warn` (in `creds.sh`) where it calls
`_increment_count`, i.e. for counted commands only (not help, version,
status, prompt or init). It returns at once unless `IDIOT_ACTIVE_CRED` and
`IDIOT_CRED_EXPIRES` are both set, so shells without an active
credential pay nothing. Otherwise one `date -u '+%s %Y-%m-%d'`; if the
credential has expired or has less than `IDIOT_CRED_WARN_DAYS` × 86400
seconds left, and `IDIOT_CACHE_DIR/expiry-warned/<name>` does not hold
today's date, it prints the warning and writes the date there.

Under the snap a host command passes through the dispatcher twice
(confined, then host-side after the handoff); both share
`IDIOT_CACHE_DIR`, so the confined pass writes the stamp and the host pass
stays quiet. `auth login` prints its own notice whenever it activates an
expiring credential and writes the same stamp, so the dispatcher does not
repeat it that day.

Messages (stderr, `say`):
- `idiot: credential 'work' expires in 3d — renew with: idiot auth add`
  (yellow)
- `idiot: credential 'work' expired 2d ago — create a new one with: idiot auth add`
  (red)

Alternatives considered:
- *At `idiot init`*: init runs from the rc file of every new terminal,
  where usually no credential is active yet, and its stdout is being
  eval'd; a warning there is noise in every new shell. Rejected.
- *Once per shell* (a shell variable set through the eval file): exported
  variables leak into child shells and every new tmux pane warns again;
  it would also make an otherwise read-only command queue eval output.
  Once per day per credential, tracked in the cache, is quieter and needs
  no eval output.
- *Every command*: spam. Rejected.

### Thresholds and phrasing
`IDIOT_CRED_WARN_DAYS`: default 7; `0` turns off the early warning (the
expired warning stays); anything that is not a non-negative integer is
treated as 7. Phrase (`expiry_phrase <unix> <now>` in `creds.sh`): ≥ 1 day
left → `expires in <N>d` (whole days, rounded down); ≥ 1 hour →
`expires in <N>h`; less → `expires in <1h`; past → `expired` (`expired
<N>d ago` in the warning). Colours: dim normally, yellow inside the
window, red when expired (via `${YELLOW}`/`${RED}`, never raw escapes).

### Helper tests
In `src/store/credential.rs` (or a new `src/store/expiry.rs`), using the
existing `macaroon()` test fixture:
- root with `dashboard.snapcraft.io|expires|2027-11-09T18:00:00.000` and a
  discharge with `login.ubuntu.com|expires|2027-05-01T00:00:00Z` → both
  lines, the first with `Z` appended;
- other first-party caveats (`…|account|…`, `…|valid_since|…`) and
  third-party caveats are not printed;
- no expiry caveat → empty output;
- garbage / wrong `t` → `InvalidCredential` (exit 2);
- the output never contains the credential string, either serialised
  macaroon, or the hex/base64 of any signature.
`cargo test` stays offline; `make check` runs clippy over them.

## Risks / Trade-offs

- [The caveat format differs from the assumption] → task 1.1 checks a real
  credential first (printing only predicate *keys* and expiry values);
  the matcher is one function, and `whoami`'s value is the fallback.
- [Store-side revocation or a shorter server-side limit] → the record says
  when the credential *would* expire; `auth status` still shows the
  store's answer, and a rejected credential is still reported as such.
- [Clock skew] → expiry is compared with local `date`; a skewed clock moves
  the warning by the skew, nothing more.
- [Snap credentials saved before this change have no record] → expiry
  unknown until `idiot auth status` runs host-side for them; login says
  so, and `idiot doctor` (`add-doctor`) lists credentials without one.
- [Merge with `add-env-profiles`] → both modify the auth `Activating a
  credential` and `Removing a credential` requirements and the same
  commands; the later change merges the requirement texts.

## Migration Plan

Additive. Existing credentials get a record the next time
`auth status` (or, from source, `auth login`) runs for them. Rollback:
remove the subcommand and shell changes; stray `.expires.*` files are
ignored by older versions (dotfiles are skipped everywhere).

## Assumptions

- Store root macaroons carry `dashboard.snapcraft.io|expires|<ISO time>`
  first-party caveats and SSO discharges may carry
  `login.ubuntu.com|expires|<ISO time>`; to be confirmed in task 1.1.
- The effective expiry is the earliest of all expiry caveats.
- Warning window default of 7 days, set by `IDIOT_CRED_WARN_DAYS`.
- "Once per day per credential, across shells" is acceptable instead of
  "once per shell".
- `auth login` still activates an expired credential (with a red notice)
  rather than refusing; refusing would block inspecting it with
  `auth status`.
