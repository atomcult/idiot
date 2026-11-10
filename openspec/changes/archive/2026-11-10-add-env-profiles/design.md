# Design

## Context

What the four settings are today (observed in the code):

| Setting | Set by | Unset by | State |
|---|---|---|---|
| credential | `auth login <name>`: `require_cred`, then exports `UBUNTU_STORE_AUTH`, `SNAPCRAFT_STORE_CREDENTIALS` (file contents) and `IDIOT_ACTIVE_CRED` | `auth logout`: unsets all three | `IDIOT_AUTH_DIR/<name>` (0600 in a 0700 dir) |
| store | `store use <name>`: `require_store`, exports `UBUNTU_STORE_ID` from the file | `store unset` | `IDIOT_STORES_DIR/<name>` holds the ID |
| arch | `arch set <arch>`: validates against a list inlined in `cmd/arch/set`, exports `UBUNTU_STORE_ARCH` | `arch unset` | env only |
| lp | `lp set <user>`: writes `IDIOT_LP_USER_FILE` and exports `IDIOT_LP_USER` | `lp unset`: deletes the file, unsets | env, falling back to the file (`lp_user_value`) |

None of the eight commands has `# idiot: host`: each only reads or writes
files under `IDIOT_DATA_HOME` and queues statements in the eval file, and
the only tool any of them needs is fzf (pickers). A command composed only
of what they do is therefore confined by CLAUDE.md's rule, which is what
lets `profile use` run without a host handoff.

What records a credential's store today: nothing.
- The credential is snapcraft's `export-login` format, a base64 root
  macaroon plus discharge; `idiot-store login` requests it with
  permissions, packages, channels and expiry only
  (`tools/idiot-store/src/store/login.rs` `Restrictions`).
- `idiot-store whoami` (dashboard `tokens/whoami`) reports email,
  username, id, permissions, channels and expiry; the cache under
  `IDIOT_CACHE_DIR/whoami/<name>` is that report verbatim.
- Store credentials are account-scoped: the same macaroon is used against
  any store the account has a role in, and whoami doesn't depend on
  `UBUNTU_STORE_ID`. So "last verified against" can't be learned from the
  store either; a successful `auth status` says nothing about which store
  the account can use.

The wrapper evaluates the eval file even when the command exits non-zero
(shell-integration spec, "exit status preserved"), so a command that queues
some statements and then dies leaves a half-applied change.

## Goals / Non-Goals

**Goals:**
- One switch for all four settings, all-or-nothing.
- A single implementation of each setting's export/unset, shared by the
  per-setting commands and `profile use`.
- A store mismatch guard that is cheap (no network), quiet when it knows
  nothing, and always tells the user how to fix it.

**Non-Goals:**
- Asking the store which brand stores an account can access, or
  validating a credential against a store. That needs a store API call and
  host handoff, and still wouldn't say which store the user *meant*.
- Blocking or refusing a mismatched switch: the warning is advisory.
- Profiles that hold anything but the four settings (model, vm, kernel
  choices), or profile inheritance/defaults.
- Changing `tools/idiot-store` or adding any non-shell component: this is
  all POSIX sh over small text files.

## Decisions

### Profile file: `key: value` lines, parsed, never sourced
`IDIOT_PROFILES_DIR/<name>`, e.g.

```
auth: ci
store: staging
arch: arm64
lp:
```

A key that is absent is not part of the profile (left as is on `use`); a
key with an empty value clears the setting. Read with `while IFS= read -r`
and `case`, never `.`/`eval`, so a hand-edited file can't run code. An
unknown key or a line without `:` makes the profile invalid (fail, change
nothing) rather than being skipped, so a typo like `stor:` can't silently
turn into "leave the store alone". `key: value` matches the whoami cache's
format.

*Alternatives:* `KEY=value` sourced as shell (rejected: executes the file);
one file per field in a directory per profile, like stores (rejected: four
files for four words, and harder to read in the picker preview).

### Absent vs. empty: both, explicitly
The request was to decide between "a missing field means leave as is" and
"means unset it". Both are needed and the file can express both without a
sentinel value:
- Unsetting is a real target state: "personal" = personal credential on
  the global store, and a profile that can't say "global" would leave a
  brand store from the previous profile active, which is exactly the
  mismatch this change guards against. So `save` records cleared settings
  as empty by default.
- Leaving as is is also real: arch usually follows the build target, not
  the account, so a "staging" profile shouldn't reset it. `save --fields
  auth,store` records only those.
- A sentinel like `store: none` was rejected because `none` is a valid
  store and credential name.

### References by name, resolved at `use`
Profiles hold credential and store *names*. `use` resolves them then, so a
re-imported credential or a store re-added with a new ID is picked up, and
no secret or ID is duplicated. `save` turns `UBUNTU_STORE_ID` back into a
name with the reverse lookup `cmd/status` already does (moved into
`store_name_for_id` in `share/stores.sh`); when no saved store matches, or
the credential was activated without a name (`IDIOT_ACTIVE_CRED` unset but
`SNAPCRAFT_STORE_CREDENTIALS` set), the field is left out with a warning
("save it first: idiot store add <name> <id>" / "activate it by name:
idiot auth login"), rather than failing the save or recording a raw value.

`auth remove`/`store remove` don't touch profiles; a profile left pointing
at a removed name fails cleanly on `use` and its entry in `profile list`
says which reference is missing.

### Validate everything, then emit
`profile use` runs in two phases: parse the file and check every field
(credential exists and name is valid, store exists and name is valid, arch
is in the list, lp is non-empty if present), collecting nothing into the
eval file; only then queue the statements. Because the wrapper sources the
eval file even on failure, this ordering is what makes a broken profile
change nothing. Names read from the file go through the same validation as
the commands that create them (`validate_cred_name`, the store-name
pattern), so `auth: ../../x` is rejected rather than read.

### Shared emitters instead of calling the commands
`profile use` could run `cmd/auth/login <name>` etc. as children (they
inherit `IDIOT_EVAL_FILE` and would append to it). Rejected: each child
validates and emits in one go, so a later child's failure leaves earlier
ones applied; `lp set` writes the saved file, which a profile must not; and
a child's picker would open if a name came through empty. Instead:
- `share/creds.sh`: `emit_cred_login <name>`, `emit_cred_logout`;
- `share/stores.sh`: `store_id <name>`, `store_name_for_id <id>`,
  `emit_store_use <name>`, `emit_store_unset`;
- `share/arch.sh`: `STORE_ARCHES` and `is_store_arch`, replacing the list
  inlined in `cmd/arch/set` (also used for its error message);
- lp needs only `emit_export`/`emit_unset IDIOT_LP_USER`.
`auth login`/`logout`, `store use`/`unset` and `arch set` are rewired onto
these with no behaviour change, so `profile use` and the per-setting
commands can't diverge.

### `profile use` does not touch the saved Launchpad user
`lp set` persists the username for new shells; `profile use` only exports
or unsets `IDIOT_LP_USER`. Switching profiles in one terminal shouldn't
change what every new terminal starts with. Consequence, accepted: a
profile that records `lp` empty unsets the variable, and the effective
user then falls back to the saved file, if any. `save` records the
*effective* user (`lp_user_value`), so a profile saved in a fresh shell
pins the saved user explicitly.

### Active profile: an exported name, drift computed on demand
`use` exports `IDIOT_ACTIVE_PROFILE=<name>`. `profile status` and the
`idiot status` row re-read the profile and compare each recorded field to
the current environment (`IDIOT_ACTIVE_CRED`, `UBUNTU_STORE_ID` vs. the
store's current ID, `UBUNTU_STORE_ARCH`, `IDIOT_LP_USER`). `lp` compares
the variable, not the file fallback, since that's what `use` set. No
`profile unset`: the per-setting commands already clear settings, and a
stale `IDIOT_ACTIVE_PROFILE` is harmless because drift is shown.
*Alternative:* match the current settings against every profile to infer
the active one (rejected: ambiguous when profiles overlap, and slower as
the list grows).

### Credential/store pairing: idiot's own record
Since neither the credential nor the store says which store a credential
is for (see Context), idiot records it:
- Where: `IDIOT_AUTH_DIR/.store.<name>`, one line `store: <id>` (empty
  value: global). Next to the credential so it moves with
  `IDIOT_AUTH_DIR` and the data-dir backup, private like it (0600 under
  `ensure_auth_dir`'s umask), and invisible to `auth list` (`*` glob) and
  `fzf_pick_from_dir` (`! -name '.*'`). Credential names can't start with
  `.`, so `.store.*` can't collide; `auth add` already keeps its temp file
  as `.new.<name>.$$` there. Holds a store ID only, never credential
  material.
- Written by `auth add` from `UBUNTU_STORE_ID` at creation (host side; the
  handed-off command runs in the user's shell, so the variable is there),
  and by the new confined `auth bind [<name>]`. Deleted by `auth remove`.
- Not written by `auth import` (the active store at import time says
  nothing about the file), `auth login` (people log in before *or* after
  `store use`; recording at login would re-pair on every login and then
  warn about the order the user happened to type things in), or
  `auth status` (whoami doesn't involve the store).
- Compared by store ID, displayed by saved store name where one matches.
- Absent record = unknown = never warns, so existing credentials keep
  their behaviour until the user binds them.

*Alternatives:* a field in the whoami cache (rejected: a cache, deleted by
`--refresh`); a separate `IDIOT_DATA_HOME/cred-stores/` dir (rejected:
doesn't follow a custom `IDIOT_AUTH_DIR`); asking the store for the
account's stores (rejected under Non-Goals).

### One mismatch helper, called after the change
`warn_store_mismatch <cred> <store-id>` in `share/creds.sh` (needs
`stores.sh` for names; callers include both). Callers pass the settings as
they will be after their own change: `store use` passes the new ID,
`auth login` the new name with the current `UBUNTU_STORE_ID`, `profile use`
the resolved pair (with the current value for a field the profile leaves
out), `profile save` what it recorded. It prints one `say` line in
`YELLOW`, e.g.

```
warning: credentials 'ci' belong to store 'staging', but prod is active
         switch: idiot store use staging · re-pair: idiot auth bind
```

and returns 0 whatever happens (no `die`, missing files tolerated), so it
can't alter an exit status. In `idiot status` it goes below the rows,
before the XP bar.

### All new commands confined
`profile *` and `auth bind` read/write only `IDIOT_PROFILES_DIR`,
`IDIOT_AUTH_DIR` and `IDIOT_STORES_DIR` and use only fzf, so none gets
`# idiot: host`. The picker preview is `file_preview_cmd` (cat, or
batcat on a source install), safe because profiles hold names only. Under
the snap, `IDIOT_PROFILES_DIR` resolves to `~/snap/idiot/common/profiles`
from `IDIOT_DATA_HOME` like the other state dirs. `auth add` stays a host
command; the pairing write it gains is to the same `IDIOT_AUTH_DIR` it
already writes.

### Credentials stay where they were
`profile use` puts credential contents in the eval file exactly as
`auth login` does today (shell-quoted export values, private eval file,
never evaluated as code); profiles, pairing records, warnings and
`profile list`/`status` output contain only names and store IDs.

## Risks / Trade-offs

- [`auth add` pairs with whatever store happened to be active, possibly by
  accident] → the confirmation names the store; `auth bind` re-pairs.
- [False warnings for accounts legitimately used across stores] → bind to
  the store used most; warnings are one stderr line and never block. If it
  proves noisy, a later change can add a "global/any" pairing (see Open
  Questions).
- [The rewiring of `auth login`/`store use`/`arch set` could change their
  behaviour] → tasks re-verify each existing spec scenario for them.
- [A profile names a credential that was later re-created for a different
  account] → by design (references by name); `profile list` and the
  picker preview show names, and `idiot status` shows the account email.
- [`IDIOT_ACTIVE_PROFILE` is a new variable in the user's shell] → only set
  by `profile use`; nothing else reads it but `status`/`profile status`.

## Migration Plan

Additive. Existing credentials have no pairing (no warnings) until `auth
bind`; no profile exists until `profile save`. Rollback is removing the
commands; leftover `.store.*` and `profiles/` files are inert (`make purge`
already deletes the data dir).

## Open Questions

Assumptions made for minor details; none changes the specs' shape:
- `profile list`/`status` go to stderr like `store list`/`status`; if a
  scriptable form is wanted later, a `--short` on stdout (as `version
  --short`) can be added.
- Whether to offer a "belongs to any store" pairing (`auth bind --any`)
  for cross-store accounts. Deferred until the warning has been lived with.
- Exact wording/colour of the warning and of the `profile list` table
  (active marker, how "left out" vs. "cleared" fields are shown: proposed
  as omitted vs. dimmed `none`) is settled during implementation.
