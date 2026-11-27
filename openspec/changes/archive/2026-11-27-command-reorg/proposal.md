# Proposal

## Why

idiot's commands grew one change at a time, and the layout now gets in the
way. Assertion work is spread over four places (`store assertion`,
`inspect assertion`, `model pick` and `assertion`). Each group uses its own
verbs to activate and clear something, and `auth login` doesn't log in to
anything: it activates a saved credential, while `auth add` is what logs in
to the Snap Store. A few commands only make sense if you already know the
tree. Ergonomics and legibility come first, and the tree is still small
enough to reorganise without losing anyone.

## What Changes

- **One home for assertions.** `assertion` gains:
  - `fetch` (was `store assertion`);
  - `show` (was `inspect assertion`);
  - `pick` (was `model pick`);
  - `repos list | add | remove | update` (was `model list | add | remove | update`).

  The `model` group goes. Its name was misleading anyway: it managed
  repositories of models, not models.
- **Fetch by what the user knows.** `assertion fetch` is worth having
  where it saves a lookup. It already takes a snap's name for
  `snap-declaration`, and a name and revision for `snap-revision`. It
  gains `snap-revision <snap> [<channel>]`: the revision released in that
  channel for the target architecture, `stable` when no channel is given,
  following a closed risk down as snapd does.
- **One set of verbs in every group:**
  - `use` picks something already saved: `auth use` replaces `auth login`; `store use` and `profile use` are unchanged.
  - `set` takes a free value: `arch set` and `lp set` are unchanged.
  - `unset` clears it: `auth unset` replaces `auth logout`.
  - `profile` gains `unset`, which clears the active profile and the settings it set.
- **Explicit forms for the overloaded commands.** `vm boot [options] <image>` and `example clone <repo>` are added. The bare forms (`idiot vm <image>`, `idiot example <repo>`) keep working as shortcuts, but help now leads with the explicit form.
- **Fewer near-duplicates:**
  - `vm console-log` becomes `vm logs --console`.
  - `idiot clean` with no subcommand cleans everything, with the same confirmation, `-n` and `-y`. `clean all` goes.
  - `store list --account` replaces `store roles`.
- **Main help headings that match their contents.** "Shell" becomes "Setup & tools", "Development" is split so `kernel` and `example` sit under "Get sources", and `version` is listed.
- **Old names keep working.** Every renamed command still runs as an alias. The alias prints one line on stderr naming the new form, then behaves exactly as the new command. Aliases are hidden from help. A later change can remove them once people have moved over.
- **Help never needs `idiot init`.** The spec now says so for leaf commands too, matching the fix in 064a628.
- `store` is otherwise unchanged. `inspect` keeps `snap`, `image`, `diff`, `state` and `changes`.

Nothing is **BREAKING** while the aliases exist. Scripts calling old names
get the same stdout and exit status, plus one note on stderr.

**stdout and the eval file:** what each command writes to stdout and queues
in the eval file is unchanged, with two additions:
- `store list --account` prints on stdout, as `store roles` did. `store list` without it still prints on stderr.
- `profile unset` queues the unsets for the profile's recorded fields and for `IDIOT_ACTIVE_PROFILE`.

There are no new runtime dependencies, and the snap stages nothing new.

## Capabilities

### New Capabilities
- `command-layout`:
  - the command tree's naming rules (`use`/`set`/`unset`, explicit forms);
  - aliases for renamed commands and the note they print;
  - the main help's headings.

### Modified Capabilities
- `model`: the repository and pick commands move to `idiot assertion repos …` and `idiot assertion pick`.
- `assertion-writing`: the requirements name `assertion fetch` and `assertion show`, where they named `store assertion` and `inspect assertion`.
- `store`:
  - the fetching requirements move to `idiot assertion fetch`, and `snap-revision` can be fetched by channel;
  - `store list --account` replaces `store roles`;
  - references to `auth login` are renamed.
- `inspect`: assertion summary, signature and cross-check requirements move to `idiot assertion show`.
- `auth`: `auth use` and `auth unset` replace `auth login` and `auth logout`, and references are renamed.
- `profile`: `profile unset` is added, and the activation requirement names the new auth verbs.
- `vm`: the serial console log is read with `vm logs --console`.
- `clean`: bare `idiot clean` cleans everything, and `clean all` goes.
- `shell-integration`: dispatch resolves aliases, and help (group or leaf) never needs the wrapper.
- `doctor`, `prompt`, `repo-cloning`: hints and descriptions name the new commands.

## Impact

- **Command tree:**
  - `cmd/model/*` moves under `cmd/assertion/` (`pick`, `repos/`);
  - `cmd/store/assertion` becomes `cmd/assertion/fetch`, and `cmd/inspect/assertion` becomes `cmd/assertion/show`;
  - `cmd/auth/login` and `logout` become `use` and `unset`;
  - `cmd/profile/unset` is new;
  - `cmd/clean/*` becomes one `cmd/clean` file;
  - `cmd/store/roles` folds into `cmd/store/list`;
  - `cmd/vm` gains `boot` and `logs --console`, and `cmd/example` gains `clone`.
- **idiot-store:** the assertion lookup resolves a channel to its revision's SHA3-384, from the channel map it already fetches for the snap-id.
- **Dispatch:** `idiot` gets an alias table and the new help headings.
- **Hints:** user-facing hints that name old commands are updated in `cmd/`, `share/` and `tools/idiot-helper/src/assertion.rs`.
- **Docs:** `README.md`, `skills/using-idiot/` (SKILL.md and its references) and `CLAUDE.md`.
- **Tests:** `tests/bare-host.d/*` move to the new names, and an alias check is added.
- **Users:** anyone with old names in scripts or muscle memory gets a note, not a failure.
