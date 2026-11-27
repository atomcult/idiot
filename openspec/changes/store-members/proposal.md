# Proposal

## Why

"Why can't Lorenzo download this?" usually comes down to who is in a brand
store and with which role. Today that means opening the dashboard. The
dashboard's store API serves the list (`GET /api/v2/stores/{id}/users`) to
store admins with a `store_admin` credential, the same rights
`idiot store snaps` already checks for its full list.

## What Changes

- **`idiot store members [--tsv] [--email] [--role <role>] [--cred <name>]`**
  lists the active store's members on stdout: username, roles (`admin`,
  `review`, `view`, `access`) and display name, sorted by username.
  - `--role <role>` shows only members with that role.
  - `--email` adds each member's email. Emails are other people's personal
    data, so they're off by default.
  - `--tsv` gives tab-separated fields: username, roles, display name, and
    email when asked.
- **Needs an admin with `store_admin`**, checked before the call as
  `store snaps` does: the account's role from `/dev/api/account` and the
  credential's permissions offline. Without them it fails, saying which one
  is missing and naming saved credentials that have `store_admin` (or how
  to create one). The global store has no member list, and the command says
  so.
- `--cred <name>` works as on the other store queries.

Out of scope:
- adding, removing or changing members, and invites (the API's write
  calls). Changing who has access stays on the dashboard, where it can be
  reviewed.
- `store check`: it doesn't consult the member list; its membership line
  already uses the account's own roles.

Output: the list goes to stdout and messages to stderr. Nothing goes to the
eval file. It is a host command. There are no new runtime dependencies,
only an idiot-store query, which the snap already builds.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `store`: adds listing a brand store's members.

## Impact

- `tools/idiot-store`: `store-members --store <id>` (`member <username>
  <roles> <display name> <email>` lines, or a `refused …` line like
  `store-snaps`), using the same authorised request and in-memory discharge
  refresh.
- `cmd/store/members` (new), `cmd/store/help`.
- Tests: a synthetic fixture shaped like the documented response; a
  bare-host check for usage and the offline refusals.
- README, the using-idiot skill reference, and the store spec.
