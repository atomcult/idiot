# Proposal

## Why

Writing a model, system-user or validation-set assertion today means
copying a JSON file from somewhere, then looking up each snap's ID (and,
for a validation set, the revision in a channel) by hand with `snap info`
or the dashboard. Then come snapd's rules, which are easy to get wrong:
essential snaps without `presence`, a `default-channel` with no track,
scalars that must be strings, the 365-day limit on a system-user that
names no models, and `format` bumps. You only find out at `snap sign`, or
when the image fails to build. idiot already knows the active store, the
target architecture and the account behind the credential, and its store
client can look snaps up, so it can fill in most of an assertion and check
the rest before anything is signed.

## What Changes

New `idiot assertion` group (all host commands: they use the network, the
user's files, `snap` or an editor):

- **`idiot assertion new <model|system-user|validation-set>`** writes an
  unsigned assertion as JSON, the form `snap sign` takes (every scalar a
  string). It asks for each header on the terminal, showing a default:
  - the account behind the credential (`authority-id`, `brand-id`,
    `account-id`);
  - the target architecture;
  - the active store (a model's `store`);
  - a model's essential snaps for its base, architecture and grade;
  - a system-user's `since`, and its `until` a year later;
  - the next validation-set sequence.

  For each snap it offers the store's snaps in a picker (or a typed name),
  then the snap's channels for the target architecture. It fills in the
  snap's `id` and, for a model, its `type` and `default-channel`, or, for
  a validation set, the channel's `revision`.

  `--set <header>=<value>`, `--add <list-header>=<value>` and `--snap
  <name>[=<channel>]` give the same answers without prompts. Without a
  terminal, every required value must come from them.

  The draft then opens in the user's editor. On save it is checked against
  snapd's rules for its type, and reopened with the problems listed until
  it passes or the user gives up.
- **`idiot assertion edit <file.json>`** reads an existing unsigned
  assertion (for example from `idiot model pick --unsigned`) and goes
  through the same steps, the file's values as defaults. It offers to bump
  the model's `revision`, or the validation set's `revision` or
  `sequence`. With `--refresh` it looks each snap's ID up again, and a
  validation set's revisions from the channel the user names.
- **`idiot assertion check <file.json>`** runs those rules alone and
  prints the problems (offline; exit 1 on an error).
- **`idiot assertion sign <file.json> [-k <key>] [--chain] [-o <file>|-o -]`**
  signs with `snap sign`, the user's key. idiot never reads or stores the
  key and only passes its name to `snap sign`. Without `-k`, it picks from
  `snap keys`. It first checks, through `store-assertions`' account-key
  fetch, that the key is registered to the assertion's `authority-id`
  (warning if it isn't or can't tell). Afterwards it checks the signed
  result's signature as `idiot inspect assertion` does. With no key at
  all, it says how to create and register one (`snap create-key`,
  `snapcraft register-key`).
- **System-user passwords** are optional. When asked for, they are read
  twice from the terminal without echo and hashed with the host's
  `openssl passwd -6`, the plain password going only to its stdin. Only
  the hash is written, and system-user files are written mode 0600. SSH
  public keys are offered from `~/.ssh/*.pub`.
- **`idiot-store info`** also reports the snap's ID and each revision's
  type (two read-only fields from the call it already makes).
  `store channels` output is unchanged.

Out of scope:
- uploading a validation set to the store (`snapcraft
  edit-validation-sets` does that);
- registering keys;
- serial, account and other assertion types;
- signing with anything but `snap sign`;
- fetching assertions, which `store-assertions` (change 2) owns.

Output: `new` and `edit` write the JSON to a file (default named after
the assertion), or to stdout with `-o -`. `sign` writes the signed
assertion the same way. `check` prints problems on stderr. Prompts,
progress and the signature verdict go to stderr. Nothing goes into the
eval file.

New runtime dependencies, none staged by the snap:
- `snap` on the host for `sign` (already used by `inspect assertion`);
- an editor: `VISUAL`, `EDITOR`, then `editor`, `vi`; `--no-edit` skips
  it;
- `openssl` on the host, only to hash a system-user password.

## Capabilities

### New Capabilities

- `assertion-writing`: writing, checking and signing model, system-user
  and validation-set assertions, with snap IDs and revisions from the
  store.

### Modified Capabilities

None. `store channels` and `inspect assertion` behave as before.

## Impact

- `cmd/assertion/` (new group): `help`, `new`, `edit`, `check`, `sign`.
  `share/assertion.sh` (new): defaults per type, the snap picker and the
  editor loop shared by `new` and `edit`.
- `tools/idiot-helper`: a new offline `draft` subcommand (`write`, `read`,
  `check`) that turns line records into JSON and back, and ports snapd's
  header rules for the three types. It has no network code and handles no
  credentials, and it is only ever given a password's hash.
- `tools/idiot-store`: `info` gains a `snap <name> <snap-id>` line and a
  type field on each `channel` line, from `fields=…,type`.
- Depends on `store-assertions` for fetching account-keys and the latest
  validation set. Reuses `idiot store snaps --pick` and `idiot-store info`,
  `query_cred`, `uname_to_snap_arch` and `idiot-helper assertion`.
- Tests: Rust unit tests for `draft` against snapd's rules and for the
  changed `info` parser; `tests/bare-host.d/assertion-writer` for usage,
  `check`, offline drafts and signing with a throwaway key made in the
  container.
- README, the using-idiot skill reference, and the new spec.
