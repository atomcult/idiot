# Design

## Context

See proposal.md (Why). What this builds on:

- **`snap sign`** reads a JSON object on stdin (or a file), signs it with
  a key from the user's snap keyring (`~/.snap/gnupg`), and prints the
  assertion. `-k <name>` picks the key (default `default`), `--chain`
  appends the account and account-key, and `--update-timestamp` resets
  `timestamp`. It takes a `body` pseudo-header. snapd assembles the
  assertion before signing, so `snap sign` itself rejects an invalid one,
  but only once a key is at hand, and with Go error text. Read from
  snapd's `asserts/signtool/sign.go` and `asserts/asserts.go`. `asserts.go`
  also refuses a `format` lower than the one the headers need ("lower than
  min format … covering included features").
- **Scalars as strings.** snapcraft's signing code says "snapd expects a
  json string where all scalars are strings". It marshals every scalar,
  including integers and booleans, as a string before piping to `snap
  sign` (`snapcraft/services/assertions.py`, `marshal_scalars_as_strings`).
  Canonical's published unsigned models (`*.json` in canonical/models) do
  the same.
- **snapd's rules**, from `asserts/model.go`, `asserts/system_user.go`,
  `asserts/validation_set.go` and the type table in `asserts/asserts.go`
  (snapd master):
  - **model** (primary key `series`, `brand-id`, `model`):
    - `authority-id` must equal `brand-id`;
    - `model` matches `^[a-zA-Z0-9](?:-?[a-zA-Z0-9])*$`, with no
      uppercase;
    - with `snaps` (the extended, UC20+ form), `architecture` and `base`
      are mandatory, and `gadget`, `kernel` and `required-snaps` are
      refused;
    - `grade` is `dangerous|signed|secured`, and `storage-safety` is
      `encrypted|prefer-encrypted|prefer-unencrypted`, only `encrypted` on
      `secured`;
    - each snap has a valid name and an `id` (mandatory unless
      `dangerous`);
    - `type` is `app|base|gadget|kernel|core|snapd`, and `presence` is
      `required|optional`, with neither `presence` nor `modes` on
      essential snaps (types snapd, kernel, gadget, and the snap named by
      `base`);
    - `default-channel` must name a track;
    - there is exactly one gadget and one kernel, and no repeated name or
      ID;
    - `validation-sets` entries have `name`, an optional `sequence` > 0,
      and `mode` `enforce|prefer-enforce`;
    - `serial-authority` and `system-user-authority` are lists of account
      IDs (`*` allowed for the latter).
  - **system-user** (primary key `brand-id`, `email`):
    - `email` is RFC 5322;
    - `series` and `models` are lists;
    - `username` matches `^[a-z0-9][-a-z0-9._]*$`;
    - `password` is an optional crypt(3) hash with `$id$` ≥ 6, rounds
      5000…999999999 and salt/hash in `[a-zA-Z0-9./]`;
    - `force-password-change` needs a password;
    - `since` and `until` are RFC 3339, with `until` not before `since`
      and at most a year after it when `models` is empty;
    - `serials` needs format 1 and exactly one model, and `user-presence`
      (only `until-expiration`) needs format 2.
  - **validation-set** (primary key `series`, `account-id`, `name`,
    `sequence`):
    - `authority-id` must equal `account-id`;
    - `name` matches `^[a-z0-9](?:-?[a-z0-9])*$`, and `sequence` ≥ 1;
    - `snaps` is mandatory, each entry with a name and an `id`;
    - `presence` is `required|optional|invalid` (default `required`);
    - `revision` is a positive integer, not with `invalid`;
    - components are checked against the snap's revision;
    - `timestamp` is mandatory.
- **snapcraft's `edit-validation-sets`** opens YAML in an editor, has the
  store build the assertion (`POST /api/v2/validation-sets/build-assertion`,
  which fills IDs, `revision` and `timestamp`), signs it with `snap sign
  -k`, and posts it. idiot does the building locally and stops before
  posting.
- **The store's info call**, `GET api.snapcraft.io/v2/snaps/info/<name>`,
  as `idiot-store info` makes it. **Confirmed live, anonymously**:
  - the top-level `snap-id` and `default-track`;
  - with `fields=type`, a `type` on each channel-map entry (`kernel` for
    `pc-kernel`, `snapd` for `snapd`, `base` for `core24`, `gadget` for
    `pc`);
  - the per-architecture revisions per channel (core24 amd64:
    `latest/stable` 2307, `latest/beta` 2322).

  The snap IDs in the spec's scenarios are the live ones.
- **idiot today:**
  - `idiot-store info` prints `channel` lines (track/risk, arch,
    revision, version, released-at) and no ID;
  - `idiot store snaps --pick` lists the snaps the credential sees in the
    active store;
  - `idiot-store whoami` reports the account `id`;
  - `query_cred` picks the credential;
  - `idiot-helper assertion` verifies signed assertions (used for the
    post-signing verdict);
  - `idiot model pick --unsigned` hands out `*.json` models;
  - `store import` shows how terminal prompts with defaults are done
    (read from `/dev/tty`, a terminal check before any network call).
- **`store-assertions`** (implemented and archived: `openspec/specs/store/spec.md`)
  fetches assertions with `idiot-store assertion <type> <header>=<value>...`
  (behind `idiot store assertion`), printing the stream on stdout and
  exiting 4 when the store has no such assertion. This change uses it
  for:
  - `idiot-store assertion account-key public-key-sha3-384=<sha3-384>`:
    which account a local key is registered to, and its `since`;
  - `idiot-store assertion validation-set account-id=<id> name=<name>`,
    which asks for `?sequence=latest`: the latest sequence (the default
    for a new one) and its revision, and with `sequence=<n>` that
    sequence's revision when editing it.

  Both are anonymous (the assertions API is public). A not-found (exit 4)
  is an answer: no such validation set (sequence 1) or an unregistered
  key. Only a network or store error (exit 3) leaves the answer unknown:
  the sequence prompt then defaults to 1 with a note, as the spec allows,
  and `sign` warns that it couldn't check the key.

## Goals / Non-Goals

**Goals:**
- A correct draft in one pass for the common cases (a UC24 model for pc
  or pi, a validation set pinning a few snaps, a brand system-user), with
  snapd's errors found before signing, in idiot's words.
- The draft stays plain JSON the user can edit or keep in a model
  repository. idiot adds no headers of its own.
- Keys stay in snap's keyring and passwords in the user's head.

**Non-Goals:**
- A full editor UI for every header (components, `modes`,
  `serial-authority`, `body`…). The editor step covers those, and `check`
  validates them.
- Classic and pre-UC20 models (no `snaps` header). `check` validates
  them, but `new` writes only the extended form.
- Re-implementing `snap sign`'s cryptography, or verifying with anything
  other than the existing `idiot-helper assertion`.

## Decisions

**A new `idiot assertion` group, not `idiot model`.** Three types share
one flow (headers, snaps, editor, check, sign). `model` is about
repositories of finished models. `inspect assertion` stays where it is.
store-assertions put its user-facing fetch in the store group (`idiot
store assertion`), so there is no overlap. Alternatives considered:
- `idiot model new` plus `idiot system-user …`: scatters one flow.
- `idiot sign`: names only the last step.

**JSON in idiot-helper (`draft`), not sh.** The host has no python3, and
jq is only in the `debug` component. sh can't escape arbitrary strings
into JSON (quotes, backslashes, non-ASCII in display names and SSH key
comments) or read an existing JSON file back for `edit`, and porting
snapd's rules in awk would be unreadable and fragile. idiot-helper
already parses assertions for `inspect assertion`. It stays offline and
credential-free: it never opens a socket and is given only a password
hash, never a password. Three subcommands, all on stdin/stdout:
- `draft write <type>` reads tab-separated records and prints pretty
  JSON (2-space indent, headers in a fixed order: `type`, `authority-id`,
  the primary keys, then the rest alphabetically, with `snaps` last):
  - `h <header> <value>` sets a scalar;
  - `l <header> <value>` appends to a list;
  - `s <name>` starts a snap entry;
  - `f <field> <value>` sets a field of the current entry;
  - `v` and `f` lines do the same for a model's validation-set entries.

  Values containing a tab or newline are refused, as no header idiot
  writes needs one.
- `draft read <file|->` prints the same records for a JSON draft, so `edit`
  reads defaults with awk. It also prints `t <type>`, and fails on a
  signed assertion or another type. Fields it doesn't model (components,
  `modes`) come back as `x <path> <json>` and are re-emitted unchanged.
- `draft check <file|->` prints `error <where> <message>` and `warning
  <where> <message>` lines (exit 1 on any error), ported from snapd's
  checks above. The Go error text is reworded in idiot's style, with
  `<where>` like `snaps[pc-kernel].default-channel`. Unit tests use
  snapd's own test vectors for the rules, rewritten as JSON drafts.

Alternatives: bundling jq into the core component (still leaves the rules
in sh, and adds a tool to stage); YAML in the editor as snapcraft does
(needs a YAML parser, and the JSON is what users keep).

**Store data from `idiot-store info`, extended slightly.** `INFO_FIELDS`
gains `type`. The output gains a first `snap\t<name>\t<snap-id>` line and
a seventh, `type`, field on each `channel` line. Existing callers match
`$1 == "channel"` and read fields 2 to 6, so they are unaffected (checked:
`cmd/store/channels`, `cmd/store/check`). One info call per snap gives the
ID, type, channels and revisions, as the credential sees the active store,
which matters for private snaps in brand stores. A new
`snap-id`-only subcommand was rejected as duplicating the same request.

**Prompts in sh, on `/dev/tty`.** Prompts work as `store import`'s do. A
default is shown in brackets and Enter accepts it. The terminal check
(stdin and stderr) comes first, before any network call. Snap names come
from `fzf --print-query` over `idiot store snaps --tsv` (when it fails or
is empty, typing alone). Channels come from fzf over the `channel` lines
for the target architecture, preselecting the default. `--set`, `--add`
and `--snap` answers are taken first and never asked again.
`share/assertion.sh` holds the per-type header lists, the defaults and
the snap loop, shared by `new` and `edit`.

**Defaults per type** are in the spec (Model essentials, Defaults from the
environment). The kernel and gadget track is the base's number (`core24`
→ `24`), as in Canonical's UC24 models. Defaults exist only for amd64
(`pc`/`pc-kernel`) and arm64/armhf (`pi`/`pi-kernel`), because those are
the only gadgets Canonical publishes generally. Anything else is asked.

**Editor loop.** The draft is written to a 0600 temp file in a private
`mktemp -d` directory. The editor runs on `/dev/tty`. After each save,
`draft check` runs. On errors, the problems are listed and the user is
asked `edit again? [Y/n]`. "n" writes nothing and exits 1. Only a passing
draft is moved to the destination (`ln`-style no-clobber: write to a temp
file beside the destination, then `mv` only if the destination doesn't
exist yet, checked just before). Warnings are shown and don't block.

**Passwords.** `stty -echo` around two `read`s from `/dev/tty`, with a
trap restoring echo. The password goes `printf '%s\n' "${pw}" | openssl
passwd -6 -stdin`. printf is a shell builtin, so the password is never in
any process's argv, and it is unset right after. openssl's output (the
hash) goes into the draft records. The prompt says that SSH keys are the
usual choice and a password is optional. `--set password=` takes only a
hash, checked by `draft check`. System-user drafts, temp files and signed
files are created under `umask 077`.
Alternatives considered:
- hashing in idiot-helper (would put the password in idiot-helper's
  memory, against its no-credentials rule, and add a crypt dependency);
- `mkpasswd` (whois package, less often installed than openssl).

**Signing.** `snap keys --json` gives key names and SHA3-384s. For each
key, `idiot-store assertion account-key` gets the account-key (anonymous: the
assertions API is public) to learn its `account-id` and `since`. The
picker shows `name`, the account it is registered to (or `not
registered`), and preselects the one matching `authority-id`.
`snap sign -k <name> [--chain] <draft>` runs on the terminal, so gpg's
passphrase prompt reaches the user. Its stdout goes to a temp file, and
on success moves to the destination as above. Then `idiot-helper
assertion --remote` (with the same `--snap`/`--trusted` arguments `inspect
assertion` builds) runs on the result, its summary discarded and its
verdict line reported on stderr. A timestamp before the key's `since` is
warned about (snapd would reject the assertion on devices), suggesting
`edit`. idiot never touches `~/.snap/gnupg`.

**Host commands.** `new`, `edit` and `sign` use the network, the user's
files and host tools. `check` reads a user file, so it is host too
(CLAUDE.md: confined only for idiot's own state). Under the snap, `snap
sign` must run on the host anyway: the keyring is outside confinement.

**Credentials.** The store credential is used exactly as the other store
queries use it: `query_cred`, then `--file` or the environment, for
`whoami` and `info`. Nothing new reads it, and no output or draft contains
any part of it. Account-key and validation-set fetches go through
`idiot-store assertion`, anonymously: no credential is passed to it.

## Risks / Trade-offs

- [Ported rules drift from snapd's] → `check` is advice. `snap sign`
  remains the authority and its error is shown verbatim if it disagrees.
  The rules cite the snapd files they come from, and the tests carry
  snapd's vectors.
- [A validation set's single revision spans architectures] → the writer
  uses the target architecture and warns when other architectures have
  another revision in that channel. Users building for several
  architectures pick per-architecture sets or edit by hand.
- [`store snaps` is partial for non-admins] → the picker accepts any typed
  name, and the lookup is per snap, so partial lists only limit
  suggestions.
- [openssl missing on a minimal host] → only passwords need it. The error
  says so and suggests SSH keys.
- [Editor not on the terminal under some wrappers] → `--no-edit`, and
  `check` on its own.
- [The store is unreachable] → `new` and `edit` still work offline but
  for snap lookups (which fail, naming the snap); the sequence defaults
  to 1 with a note, and `sign` warns that it couldn't check the key.

## Migration Plan

New commands only. Rolling back removes them. The `info` additions are
backwards-compatible for every current caller.

## Open Questions

- Assumption: a validation set is not uploaded. Should `sign` offer to
  post it with `snapcraft edit-validation-sets`-like semantics later
  (`POST /api/v2/validation-sets`, a store write)? Deferred to a later
  change.
- Assumption: the default grade is `signed` and the default base `core24`.
  The user may prefer `dangerous` for day-to-day dev models.
- Assumption: drafts go to the current directory. Offering to save into a
  user model repository (`IDIOT_MODELS_DIR/<repo>`, then commit) could be
  added later without changing these commands.
- `--snap <name>=invalid` reuses the channel slot for a validation-set
  presence. If a real channel named `invalid` ever matters, use `--set`
  instead, or set `presence` in the editor.
