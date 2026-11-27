# Design

## Context

add-store-commands gave idiot-store authorised dashboard queries (header
as snapd builds it, in-memory discharge refresh) and `store-snaps`, whose
refusals come back as `refused permission <p>` / `refused not-admin`
lines. `cmd/store/snaps` checks the role (`account`) and the credential's
permissions (`permissions`, offline) before the admin call, and uses
`creds_with_permission` to name alternatives.

`GET /api/v2/stores/{id}/users` (dashboard v2 stores reference) returns
`{"store": {...}, "users": [{"displayname", "email", "id", "roles",
"username"}, ...]}`, admin-only with `store_admin`. Like the snaps call, it
answers 404 when the caller lacks rights and 403 `macaroon-permission-required`
for a missing permission. That shape is from the docs and not yet seen
live.

## Goals / Non-Goals

**Goals:** a read-only member list for admins, with the same pre-checks and
messages as `store snaps`.

**Non-Goals:** changing members or invites; showing account IDs; members of
the global store.

## Decisions

- **One query, `store-members --store <id>`,** printing `member <username>
  <roles,…> <display name> <email>` lines. The email is always in the
  helper's output, and sh drops it unless `--email` was given, so the
  helper doesn't need to know about privacy flags. The account ID is
  dropped by the helper: idiot has no use for it.
- **Shared pre-check.** `cmd/store/snaps` and `cmd/store/members` need the
  same "admin of the active store with `store_admin`" test. It moves into
  `share/stores.sh` as `store_admin_check <store-id>`. That function prints
  `ok`, `role` or `permission`, and names the alternatives through
  `creds_with_permission`. `store snaps` keeps its behaviour (a partial
  list); `store members` fails instead, since there is no partial member
  list.
- **Emails off by default.** Lists get pasted into tickets and chats.
  Making the user ask for emails keeps other people's addresses out of the
  transcript by default.

## Risks / Trade-offs

- **The response shape is from the docs only.** The live check (with the
  user, once they have a `store_admin` credential) confirms it, and the
  parser ignores unknown fields.
- **Moving the pre-check touches `store snaps`.** Its existing bare-host
  and local checks guard it.
