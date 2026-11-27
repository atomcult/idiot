# Design

## Context

idiot-store (`tools/idiot-store`) is idiot's Snap Store client: login
(`/dev/api/acl/` and the login.ubuntu.com discharge), whoami
(`/api/v2/tokens/whoami`) and offline expiry. It reads a credential from
`UBUNTU_STORE_AUTH` or `--file`, never from argv, and never prints one.
sh owns all formatting; the helper prints plain lines.

The calls this change needs, from snapcraft's store client and the
dashboard's published API reference (v1 account and macaroon pages, v2
stores page):

| Call | Host | Who | Permission |
|---|---|---|---|
| `GET /dev/api/account` (`stores` with roles; `snaps` by series with each snap's `store`) | dashboard.snapcraft.io | any account | any |
| `GET /api/v2/stores/{id}/snaps` | dashboard.snapcraft.io | store admin | `store_admin` |
| `GET /v2/snaps/info/{name}` with `Snap-Device-Series: 16`, `Snap-Device-Store: <id>` | api.snapcraft.io | anyone who can see the snap | `package_access` for private snaps |
| the revision's download URL | api.snapcraft.io | as above | `package_access` |

Both admin-only and hidden resources answer **404**, not 403, when the
caller lacks rights, so a 404 alone can't say why.

A credential's permissions are in the root macaroon's first-party caveat
`myapps.developer.ubuntu.com|acl|[...]`, a JSON list (checked on a real
credential). The expiry is read from a caveat beside it already.

## Goals / Non-Goals

**Goals:**
- `store check` names the cause of a failed download from facts it can
  gather, not from the 404 alone.
- Everything works for any account. Admins get more where the API allows.
- Credentials stay where they are: no argv, no stdout, no logs.

**Non-Goals:**
- Writing to the store (adding snaps, members, settings).
- `store members`, `vm install <snap-name>`, image pre-flight checks.
  They come later and build on this.
- Replacing `snapcraft status` for publishers (closed channels, progressive
  releases, branches' expiry). `store channels` shows what snapd sees.
- Stores other than production (staging).

## Decisions

**New idiot-store subcommands, one per question, printing tab-separated
lines.** All read the credential as whoami does (`UBUNTU_STORE_AUTH`, or
`--file`), and all are read-only:
- `permissions` (offline): the credential's permissions, one per line.
- `account`: `store <id> <name> <roles,…>` lines, then
  `snap <name> <store name> <private|public>` lines for the default series.
- `store-snaps --store <id>`: `snap <name> <publisher> <private|public>`
  lines, or exit 2 (rejected) on 404/401/403.
- `info <snap> [--store <id>]`: `channel <track/risk[/branch]> <arch>
  <revision> <version> <released-at>` lines. Exit 3 with "not found" when
  the store doesn't show the snap.
- `probe <snap> --arch <a> --channel <c> [--store <id>]`: `info anon|auth
  <status>` and `download anon|auth <status>` lines. The download request
  doesn't follow redirects and reads no body: the store answers 302 to the
  CDN when it allows the download, and that is the answer we need.

Anonymous requests send no `Authorization` header. Authorised ones send the
header snapd would build: the root and the bound discharge, both serialised
without padding. So the probe tests what snapd would do, and the offline
padding check reports a credential that snapd can't read even before any
request is made.

**sh decides and formats.** `cmd/store/check` gathers the helper's lines
and the offline facts (expiry records, `permissions`, `repair --check`,
pairing via `cred_store_id`), then reports one cause, in this order:

```
no credential active ............. only anonymous was checked (informational)
credential expired ............... renew: idiot auth add
credential padded ................ snapd downloads anonymously: idiot auth login <name>
info auth 404, no role in store .. not a member: ask a store admin
info auth 404, member ............ not in this store (or wrong store active)
info ok, no revision for target .. not released for <arch> in <channel>; lists the ones that are
download auth 404/403 ............ no download permission (role or package_access)
download anon ok/auth ok ......... downloadable (says if it needs the credential)
```

Pairing mismatches (`warn_store_mismatch`) are reported as `!` lines, not
causes. They explain a wrong store but don't stop a download.

**Admin detection before the admin call.** `store snaps` first reads the
roles (`account`) and the credential's permissions (offline). Only an
admin with `store_admin` gets `store-snaps`. Everyone else gets the
`account` snaps filtered to the active store's display name (from the same
`account` response; `Global` for the global store), with a note saying why
the list is partial. The ambiguous 404 never needs interpreting.

**`--cred <name>`** maps to `--file "${IDIOT_AUTH_DIR}/<name>"` for the
helper. The environment is untouched. A shared `creds_with_permission
<perm>` in `creds.sh` runs `permissions --file` over saved credentials to
name alternatives.

**Output shapes.** Data commands print aligned columns on a terminal and
plain tab-separated fields with `--tsv` (no colour). Snap and store names
are ASCII; display names may not be. The helper prints them as-is, and sh
puts display names last so mismatched padding can't misalign the columns.

**Host commands.** All four use the network, so they carry `# idiot: host`
and run from the user's shell under the snap, as `auth status` does.

## Risks / Trade-offs

- **Response shapes beyond the docs.** The `stores` entries in
  `/dev/api/account` and the `/api/v2/stores/{id}/snaps` items aren't fully
  shown in the reference. The first task records real responses (with
  account details replaced) as test fixtures and writes the parsers from
  them.
- **v2 info `fields`.** The fields needed (download URL, revision, version,
  released-at per channel) must be requested explicitly. The exact field
  names are confirmed against a live response in the same task.
- **No store in CI.** Rust tests run against fixtures, bare-host covers
  usage, errors and the offline checks, and the live checks are run by the
  user against `ce-team-test` (admin) and `dsmr6` (viewer).
- **A 302 means allowed today.** If the store started streaming without a
  redirect, the probe would read only the status and drop the connection.
  It is still correct, just less tidy.
