# Design

## Context

- **Drafts today.** `idiot assertion new/edit` builds a draft from
  tab-separated records:
  - `idiot-helper draft read` turns a JSON draft into records (`t`, `h`,
    `l`, `s`, `v`, `f`, `x`);
  - `share/assertion.sh` keeps those records in `${work}/records` and
    prompts with them as defaults;
  - `draft write` builds the JSON.

  `parse_draft` in `tools/idiot-helper/src/draft.rs` refuses input that
  starts with `type:` ("a signed assertion, not a JSON draft"). The
  `signed_and_foreign_input` test checks that.
- **The parser already exists.** `tools/idiot-helper/src/assertion.rs`
  (used by `idiot inspect assertion`) decodes signed assertions and
  streams: `decode_stream` gives each assertion's headers as `V::S`,
  `V::L` or `V::M`, its body and its signature. That is the shape of the
  JSON `snap sign` takes: every scalar a string, lists and maps nested.
- **The fetch already exists.** `cmd/store/assertion` (store-assertions)
  maps `model <brand>/<model>` and `validation-set
  <account>/<name>[=<seq>]` to primary keys, uses the active store and
  credential (`--cred`), and with `--raw` prints the assertion as served.
  Its not-found messages are already worded for users.
- **Terminal detection.** The writer decides it has a terminal from stdin
  and stderr (`as_init`: `[ -t 0 ] && [ -t 2 ]`, and `/dev/tty` opens).
  Reading the source from stdin therefore always means "no terminal".
- **Assertion bodies.** snapd's model, system-user and validation-set
  assertions have no body (asserts/model.go, system_user.go,
  validation_set.go), so a signed one carries no `body-length` beyond
  `0`.

## Goals / Non-Goals

**Goals:**
- `edit` starts from a signed assertion (a file, `-`, or the store) as
  well as from a draft. It never writes to the signed file.
- `new --from` / `--from-store` uses any assertion of the same type as a
  template, without carrying over its identity.
- Reuse the existing parser and fetch. Add no network code to
  idiot-helper, and no new runtime dependency.

**Non-Goals:**
- Verifying the source's signature. `idiot inspect assertion` and
  `idiot store assertion` already do that, and a template only supplies
  defaults.
- Fetching system-users from the store. The store doesn't serve them by
  a key the user knows; they come from a file.
- Assertion types other than the three the writer supports.
- Converting a template's snaps to another store. Snap IDs are kept, and
  `--refresh` looks them up again in the active store when wanted.

## Decisions

1. **Parse signed input in idiot-helper `draft read`, not in sh.** Signed
   headers can be nested lists and maps (`snaps`, `validation-sets`,
   `system-user-authority`) in snapd's indentation-based format, and POSIX
   sh has no robust way to parse that. `assertion::decode_stream` already
   does it, offline, and is tested against Canonical's models.
   - `parse_draft` tries JSON first. Input starting with `type:` goes
     through `decode_stream`.
   - The converter turns `V::S`/`V::L`/`V::M` into `Value::String`,
     `Array` and `Object` (keeping header order for the `x` records), and
     drops `sign-key-sha3-384` and `body-length`.
   - The existing record emitter then runs unchanged.
   - A non-empty body fails ("has a body, which a <type> can't have").
     Accepting it silently could hide a corrupted file.
   - *Alternative:* a new `draft import` subcommand. Rejected: `read`
     already means "records for this input", and one entry point keeps
     `edit` simple.
2. **A source record.** For signed input, `draft read` prints
   `o\tsigned\t<type>\t<primary keys joined by />\t<revision>` right after
   `t`; for a draft it prints `o\tdraft\t…`.
   - `draft write` ignores `o` records, so `read | write` still round-trips.
   - sh uses the record to say where it started, to choose the output
     name, and, for templates, to know nothing more.
3. **Picking from a stream.** `draft read` uses the one assertion of type
   model, system-user or validation-set in the stream, and skips
   `account`, `account-key`, `snap-declaration` and the rest. With none,
   or with two or more, it fails listing the types it saw ("holds account,
   account-key: no model, system-user or validation-set"). This covers
   `idiot store assertion --chain --raw`, `snap known model`, and a
   device's `snap model --assertion`.
4. **CLI shape.**
   - **`edit`:** `edit <file|->` keeps its one positional argument, and
     `edit <model|validation-set> <key>` takes two, the same words as
     `idiot store assertion`. Two positionals whose first is one of those
     types means a fetch. One positional is always a path, so a file
     named `model` still works.
   - **`new`:** takes `--from <file|->` and `--from-store <key>`, because
     `new`'s positional is already the type.
   - *Alternatives:* a prefix such as `store:model/a/b`, which is awkward
     and collides with paths; or `--from-store` on `edit` too, which
     would need the type again and duplicate the positional form.
5. **Fetch by running `cmd/store/assertion --raw`.** That reuses its key
   shortcuts, `--cred`, active-store handling and messages (exit 4 means
   not found).
   - There is a precedent: `cmd/store/assertion` already runs
     `cmd/inspect/assertion` the same way.
   - The output goes to `${work}/source`, which the cleanup trap removes.
   - Credentials still go only to idiot-store, through `query_cred`'s
     file, and are never in argv or printed.
6. **Output for signed sources.** Without `-o`, a signed source (file,
   stdin or store) is written like `new`'s output: the default name from
   the edited values, in the current directory, never overwriting.
   - The signed file stays as it is. Overwriting a `.model` with JSON
     would lose the published assertion.
   - Drafts keep today's write-back.
7. **Template mode** (`new --from`) loads the template's records, then
   removes identity before prompting, so the existing "no default"
   prompts and the non-terminal "missing header" failure apply unchanged:
   - **model:** `authority-id`, `brand-id`, `model`, `display-name`,
     `revision`, `timestamp` (a template's display name names the other
     model);
   - **validation-set:** `authority-id`, `account-id`, `name`,
     `sequence`, `revision`, `timestamp`;
   - **system-user:** `authority-id`, `brand-id`, `username`, `email`,
     `name`, `password`, `since`, `until`, `revision`, `timestamp`.

   `serials`, `models` and the rest stay as defaults. `--set`, `--add`
   and `--snap` are applied after the template, as `edit` already
   applies them after the file, so they win.
8. **No signature check.** A template is only defaults, and `sign` makes
   a new signature. Checking would need the full chain (a network fetch
   or `--chain`) for no gain to the draft. stderr says "signed" or
   "draft" and where the input came from, so the user knows.
9. **stdin sources have no prompts.** Because terminal detection is on
   stdin, `edit -` and `new --from -` behave as without a terminal:
   `--no-edit` is implied, and required values come from the source or
   `--set`. For an interactive edit, the help points to a file or the
   store form.

## Risks / Trade-offs

- **Unknown headers.** A signed assertion from a newer snapd may carry
  headers idiot doesn't model. They pass through as `x` records and
  `draft check` judges them as today, so nothing new can slip past
  `check`.
- **A signed system-user holds a password hash.** It is kept in an `edit`
  (it's the same user) and dropped in a template. Both paths already run
  under `umask 077`, and the hash is never printed.
- **A brand-store snap ID in a template from another store.** An ID that
  isn't the active store's is kept. `draft check` can't see that, but
  `--refresh` (or `--snap`) fixes it, and the README says so.
- **Two positionals in `edit`** could surprise someone who passes two
  files. That is an error today; it then becomes "unknown assertion type
  `<first>`" unless the first word is `model` or `validation-set`, and
  the usage line shows both forms.

## Open Questions

- Should `idiot model pick` (signed `.model` files from model repos) get
  an "edit this" action now that `edit` reads signed models? This change
  leaves it out; `idiot assertion edit "$(idiot model pick)"` works.
