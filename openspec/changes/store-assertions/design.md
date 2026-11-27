# Design

## Context

- idiot-store (`tools/idiot-store`) is the Snap Store client: it reads a
  credential from `UBUNTU_STORE_AUTH` or `--file`, sends the header snapd
  builds, refreshes the Ubuntu One discharge in memory on
  `macaroon-needs-refresh`, and prints plain lines for sh to format. Its
  `info` subcommand already calls `GET /v2/snaps/info/<name>` with
  `Snap-Device-Series: 16` and `Snap-Device-Store`.
- idiot-helper's `assertion` module parses, summarises and verifies an
  assertion stream offline (`idiot inspect assertion`). It looks up
  missing account-keys in the stream, then snapd's database, then the
  bundled Canonical keys, and with `--remote` through `snap known
  --remote`. It has no network code and must keep none.
- `idiot store` queries share `query_cred` (share/creds.sh) for `--cred`,
  and `store_helper` for the idiot-store path.

### The store's assertions service

Confirmed live with unauthenticated GETs to api.snapcraft.io (2026-11-27):

- `GET /v2/assertions/<type>/<primary key values in order>` with `Accept:
  application/x.ubuntu.assertion` returns the assertion (content type
  `application/x.ubuntu.assertion`, body ending in one newline). Fetched:
  `account/canonical`, `account-key/<sha3-384>` (Canonical's `root` and
  `store` keys), `snap-declaration/16/<snap-id>` (core, hello-world),
  `snap-revision/<base64url sha3-384>` (hello-world r29),
  `model/16/canonical/ubuntu-core-24-amd64`. `?max-format=N` is accepted.
  `Accept: application/json` returns `{"headers": {...}}` without the
  signature, so idiot asks for the assertion media type.
- Errors are JSON `error-list`s: `404 not-found` ("no assertion with type
  … and key {…}"), `400 invalid-request` naming the expected key
  ("should be \"{series}/{brand-id}/{model}\""), `400` "invalid assertion
  type". An unknown path shape (e.g. a query string instead of a key)
  is an HTML 404.
- Sequence-forming types: `validation-set/16/<account>/<name>?sequence=latest`
  is understood (its 404 names a "sequence key"); with a sequence the key
  is `…/<name>/<sequence>`.
- The service knows every type in snapd's registry: probing each with a
  bogus key gave a key-shape answer, not "invalid assertion type", for
  account, account-key, model, serial, base-declaration, snap-declaration,
  snap-build, snap-revision, snap-developer, system-user, validation,
  validation-set, store, preseed, snap-resource-revision,
  snap-resource-pair, confdb-schema, cluster, repair.
- `snap-revision`'s key is `{snap-sha3-384}[/{provenance}]` (the store
  fills in `global-upload`); `snap-developer`'s is `{snap-id}/{publisher-id}`.
- `GET /v2/snaps/info/<name>` returns the top-level `snap-id`, and per
  channel `download.sha3-384` (hex) with `revision`.
- Canonical's `account` is signed by the root key (`-CvQ…`), and snap
  declarations and revisions by the `store` key (`BWDE…`), which the root
  key signs.

From snapd's source (store/store_asserts.go, asserts/*.go, master), not
tested live:
- snapd sends `max-format` per type: `snap-declaration` 7, `system-user`
  2, `account-key` 1, others 0; and the user's macaroon when logged in.
- Prerequisites: `account-key` → `account`; `snap-declaration` →
  publisher `account`; `snap-revision` → `snap-declaration` (series 16) and
  developer `account`; `snap-developer` → `snap-declaration`, publisher and
  developer `account`s; `store` → operator `account`. Plus, for every
  assertion, the `account-key` named by `sign-key-sha3-384`. snapd trusts
  Canonical's `account` and root `account-key` built in.
- A revision that isn't in the channel map: snapd's `snap download
  --revision` asks `POST /v2/snaps/refresh` with a `download` action
  naming the revision, and reads `download.sha3-384` from the answer. Not
  called during drafting (only GETs were allowed); confirmed in task 1.

Assumed: a brand store's assertions (models, serials, system-users,
private snaps' declarations) may need the credential, as private snaps'
info does; and the assertions service answers 404 rather than 403 when it
hides something, as the rest of the store does.

## Goals / Non-Goals

**Goals:**
- One idiot-store call that `vm-install-from-store` and `assertion-writer`
  can use to get a ready stream (`--chain`) without parsing anything.
- Users name snaps by name and revisions by number; ids and hashes are
  looked up for them.
- The human-readable view is exactly `idiot inspect assertion`'s.

**Non-Goals:**
- Writing to snapd (`snap ack`), or to the store.
- Searching assertions by non-key headers.
- Changing idiot-helper (no network there) or `inspect assertion`.

## Decisions

**The fetch lives in idiot-store, as `idiot-store assertion`.** sh can't
fetch with the credential (it never sees it in a form curl could send
safely, and curl isn't a dependency), and the chain needs the refreshed
discharge to survive across requests, which only one process can hold.
Alternative considered: a sh loop over single fetches — rejected, as each
process would refresh the discharge again and sh would have to parse
headers. idiot-helper is out: no network code there by rule.

The interface, for idiot's own commands and the other changes:

```
idiot-store assertion [--file FILE] [--store ID] [--arch ARCH] [--chain]
                      <type> <header>=<value>...
```

- Credential: `--file`, else `UBUNTU_STORE_AUTH`, else anonymous (as
  `info`). Never in argv, never printed.
- `--store`: sent as `Snap-Device-Store` on every request (as snapd does),
  and used for the snap-id/sha3-384 lookups. Default: none (global).
- Headers: the type's primary-key headers in any order. `series` defaults
  to `16`; an optional trailing key (`provenance`) is left to the store.
  `validation-set` without `sequence` asks for `?sequence=latest`.
  Two lookups are accepted in place of key headers:
  `snap-name=<name>` for `snap-id` (any type with a `snap-id` key), and
  `snap-name=<name> snap-revision=<n>` for `snap-sha3-384` (`--arch`
  needed only if the revision isn't in the channel map). `snap-sha3-384`
  may be hex (96 digits, converted) or base64url.
- `--chain`: the prerequisites (Context), fetched depth-first, each
  assertion printed after everything it needs, deduplicated by type and
  key, stopping at Canonical's `account` and root key
  (`-CvQKAwRQ5h3Ffn10FILJoEZUXOv6km9FwA80-Rcj-f-6jadQ89VRswHNiEB9Lxk`).
- stdout: the stream, each body as served, separated by a blank line.
  Nothing is printed until every assertion has been fetched, so a failure
  never leaves half a stream on stdout.
- `max-format`: snapd's table above, so the store serves the revision
  snapd would get.
- Exit: 0 ok; 1 usage (unknown type, missing or unknown header: the
  message names the type's key); 2 credential rejected; 3 network or store
  error; **4 not found** (store 404 `not-found`, or a snap name the info
  lookup doesn't show), the message naming the type and key. 4 is new and
  only `assertion` uses it, so callers can tell "absent or hidden" from
  "store down" without parsing stderr.

The types table (name, primary key, optional trailing key, sequence-
forming) is copied from snapd's `asserts.go`; a type outside it fails
offline with the list. Following the chain needs only top-level `name:
value` headers (before the first blank line) plus the `developer-id`s in a
`snap-developer`'s `developers` list, so idiot-store gets a small reader
for those rather than idiot-helper's full parser (a shared crate would be
the alternative; not worth it for a dozen lines, and the summariser stays
the single place that interprets assertions for users).

**`cmd/store/assertion` (sh) maps shortcuts to headers and formats.** The
shortcuts in the spec become header words (`model acme/gw` → `brand-id=acme
model=gw`; `validation-set a/n=3` → `account-id=a name=n sequence=3`;
`snap-revision <hex|b64>` → `snap-sha3-384=`; `snap-revision <snap> <n>` →
`snap-name=… snap-revision=…`; `store` → `store=${UBUNTU_STORE_ID}`). A
first key word containing `=` selects the generic form. It passes
`--file` from `query_cred`, `--store` from `UBUNTU_STORE_ID`, and `--arch`
from `--arch`/`UBUNTU_STORE_ARCH`/the host. The stream goes to a private
temp file, then:
- default: `cmd/inspect/assertion` on it (so summary, signature status,
  model cross-checks and exit status are inspect's, not a copy);
  `--no-verify` passes through. Without `--chain`, keys the stream lacks
  are looked up as inspect does (local snapd, bundled keys); a store key
  missing there shows `unverified`, and idiot adds a hint to use `--chain`.
- `--raw`: `cat` to stdout.
- `-o <file>`: copied next to the target and renamed over it, then
  `wrote <n> assertions to <file>` on stderr (n counted from `type:` lines
  at the start of each assertion). Mode follows the umask: assertions are
  public, signed data, not credentials.
Exit-status mapping: 4 → "the store has no <type> <key>, or doesn't show
it to this credential" (for a snap-name lookup: suggest `idiot store check
<snap>`); 2 → the usual rejected-credential message; 3 → the store's
message.

**Credentials.** The credential reaches idiot-store only via `--file` or
the environment, as for every other query. Fetched assertions contain no
secrets except a `system-user`'s password hash, which the summariser never
shows; `--raw`/`-o` print it because that is the assertion the user asked
for (it is public to anyone who can fetch it).

**Host command.** It uses the network, so `# idiot: host`, as the other
store queries.

## Risks / Trade-offs

- [The refresh download action may need auth or the right architecture
  for an unreleased revision] → try the channel map first (covers released
  revisions, anonymous for public snaps); for the rest, use `--arch` and
  the credential, and on 404 say the revision may be for another
  architecture or unreleased-and-hidden. Confirmed in task 1.
- [Brand-store assertions may behave differently (auth, `Snap-Device-Store`)]
  → covered by the live check with the user against `ce-team-test`.
- [snapd raises `max-format`s over time] → the table is one constant,
  updated with snapd; an old value only means the store may serve an older
  revision of a declaration, which is what that snapd would see anyway.
- [Exit 4 is a new code for idiot-store] → documented in main.rs's exit
  table; only `assertion` uses it, so existing callers are unaffected.
- [Chain fetches are sequential] → a snap-revision chain is three requests
  after dedup; a model with a brand key, four. Acceptable.

## Open Questions

- Should `idiot inspect assertion --remote` fetch missing keys with
  `idiot-store assertion account-key …` instead of `snap known --remote`,
  so it works without snapd? It would keep idiot-helper network-free (sh
  would fetch, then re-run the summary). Left for a follow-up; it doesn't
  change this change's interface.
- Should `snap-revision <file.snap>` hash a local snap (idiot-helper
  already computes SHA3-384) to fetch its assertions? Small to add later.
- `vm-install-from-store` may want the revision's sha3-384 and the
  download URL from one lookup; if so, `idiot-store info` could print the
  hash too. That is for that change to decide.
