# Design

## Context

The motivation and the new layout are in proposal.md.

- **Dispatch.** `idiot` maps words onto files under `cmd/`: a directory is a group, and `dispatch_subcmd` goes up to two levels deep. Under the snap, `run_cmd` hands each host command (`# idiot: host`) to the user's shell as an `idiot <words…>` line.
- **Counting.** `_counted` counts every invocation by its first words, and skips the expiry warning for `auth login`.
- **Single-file commands.** `vm` and `example` are single files that parse their own subcommands. `clean` is a group of three files that share `share/clean.sh`. `store roles` is a host command, because it calls the store, while `store list` runs confined.
- **Help.** Help is printed by each command file. Group help is a `help` file in the group's directory.

## Goals / Non-Goals

**Goals:**
- Old names behave exactly like the new ones, including under the snap's handoff, apart from one note on stderr.
- One place lists the renames, so removing the aliases later is a single edit.
- Files move with `git mv`, so `git log --follow` keeps each command's history.

**Non-Goals:**
- Removing the aliases. That is a later change, after people have moved over.
- Shell completions. idiot has none today.
- Renaming spec capabilities. For example, `model` keeps its spec name and its requirements just name the new commands. Specs are organised by subject, not by command path.
- Changing any command's options or output, beyond the folds the specs list.

## Decisions

### An alias table in the dispatcher
`idiot` gets a small alias table in a new `share/aliases.sh`. Each line maps old words to new words:

```
store assertion     -> assertion fetch
inspect assertion   -> assertion show
model pick          -> assertion pick
model               -> assertion repos
auth login          -> auth use
auth logout         -> auth unset
clean all           -> clean
store roles         -> store list --account
```

Before dispatch, `main` tries the longest match on the first two words, then the first word. On a match it:
1. prints one note: `idiot: note: 'idiot model pick' is now 'idiot assertion pick'`;
2. rewrites the argument list;
3. dispatches as usual.

Because the rewrite happens before `_counted` and `run_cmd`:
- counting uses the new name;
- the snap handoff line carries the new words, so the host side runs the new command and prints no second note;
- help for an old name shows the new command's help.

`model` maps to `assertion repos`, so `idiot model update` becomes `idiot assertion repos update`. The two-word `model pick` entry is matched first, so it becomes `assertion pick`, not `assertion repos pick`.

*Alternatives:*
- *Symlinks or stub files under `cmd/`.* These would scatter the renames over many files and give no single place for the note.
- *Making each moved file accept its old name.* Files don't know how they were called.

### `vm console-log` stays inside `cmd/vm`
Its old argument shape differs from the new one: the name is positional in `vm console-log [<name>]`, but is passed with `--name` in `vm logs --console`. A plain word rewrite can't express that. So:
- `cmd/vm` keeps a hidden `console-log)` case, which prints the same note and calls the console printer with the positional name.
- `do_logs` gains `--console`, which takes `-f`, `--previous` and `--name`, and refuses a snap name or `-n`.

The note is printed by the same `alias_note` helper from `share/aliases.sh`, so the wording matches.

### `vm boot` and `example clone` are cases in their own files
`cmd/vm` main gets `boot) shift; do_run "$@"`, and `cmd/example` gets `clone)`. Both are first-class subcommands that print no note. The bare forms are unchanged. Help leads with `idiot vm boot [options] [<image>]`, and lists `idiot vm [options] <image>` as a shortcut.

### `clean` becomes one file
`cmd/clean/{all,containers,snaps}` merge into a single `cmd/clean` that takes `[containers|snaps] [-n] [-y]`. The three files differed only in which `clean_list_*` and `clean_remove_*` helpers they called. `clean all` is resolved by the alias table to `clean`.

*Alternative:* a "default subcommand" hook in `dispatch_subcmd`. That would add a dispatch concept for one group.

### `store list --account` is confined unless asked for the account
`cmd/store/list` stays confined, because a plain `store list` reads only idiot's state.
- With `--account` under the snap, it hands itself to the host with `emit_handoff store list "$@"` and exits.
- On the host (or outside a snap), it runs what `cmd/store/roles` ran: that code moves into `cmd/store/list`, behind `--account`.

This is the first command that is confined for one form and a host command for another. CLAUDE.md's confined/host section gets one line describing the pattern.

*Alternative:* mark `store list` as `# idiot: host`. Every `store list` would then need the wrapper and a handoff, for the sake of a rarely used flag.

### Moves under `assertion`
These are all `git mv`:
- `cmd/store/assertion` → `cmd/assertion/fetch`
- `cmd/inspect/assertion` → `cmd/assertion/show`
- `cmd/model/pick` → `cmd/assertion/pick`
- `cmd/model/{list,add,remove,update,help}` → `cmd/assertion/repos/…`

After the move, update each file's `. "$(dirname "${0}")/…/share/common.sh"` depth and its usage strings. `assertion/repos/help` takes over the repository listing that `model help` printed.

### `snap-revision` by channel
`cmd/assertion/fetch` maps the snap-revision words as follows:

| Words | Passed to idiot-store |
|---|---|
| `<snap>` alone (40 characters or fewer, the longest a snap name can be) | `snap-name=<snap>` and `snap-channel=stable` |
| `<snap> <digits>` | `snap-name=` and `snap-revision=`, as today |
| `<snap> <anything else>` | `snap-name=` and `snap-channel=<channel>` |
| a longer word alone | a SHA3-384, as today |

idiot-store's `Request` gains `snap_channel`. `resolve` already fetches the snap's info, including its channel map, to find the snap-id, so it looks the channel up in that same reply:
1. A risk alone takes the snap's `default-track`, else `latest`.
2. It takes the entry for that track and risk at the request's architecture. When there is none, it tries each more stable risk in the same track (edge → beta → candidate → stable), which is what snapd does with a closed channel.
3. It fills `snap-sha3-384` from that entry's `download.sha3-384`.

The channel and revision it settled on are printed on stderr, because sh owns the UX. This extends the lookup that `snap-revision=` already uses, adding no second endpoint. `snap-channel` is a lookup word, not an assertion header: like `snap-name`, it never reaches the assertions service.

*Alternative:* resolve the channel in sh, through `store channels --tsv`. That costs a second info request and parsing in sh, for what idiot-store already holds.

### `auth use` and `auth unset`
`cmd/auth/login` and `logout` are renamed with `git mv`. `_counted`'s expiry-warning skip checks `auth use`. `profile use` and `profile unset` keep calling the creds.sh emitters, which are unchanged.

### `profile unset`
`cmd/profile/unset` reads the active profile with `read_profile`. For each field it records, it emits the unsets that `profile use` emits for an empty field. It then unsets `IDIOT_ACTIVE_PROFILE`. It runs confined, like the rest of the group.

### Main help
`idiot_help` gets the headings from the command-layout spec. A bare-host check compares the commands listed in help with the ones under `cmd/`, so a new command can't go unlisted.

## Risks / Trade-offs

- **[Risk]** A script parses stderr and trips over the note. → stdout and the exit status are unchanged, and the note is a single line with a fixed prefix.
- **[Risk]** Hints and docs keep naming old commands. → Every old name is grepped for in `cmd/`, `share/`, `tools/`, the README and the skill. bare-host runs the new names, plus one check per alias.
- **[Risk]** Moving files breaks their relative `common.sh` source. → `make check` runs shellcheck `-x` on every file, and bare-host runs each moved command.
- **[Trade-off]** The `model` spec now describes `assertion repos` commands. That is accepted: the spec is about model repositories, whatever the command path.

## Migration Plan

Nothing to migrate: no saved state changes. The aliases ship with the new names, and README and the skill teach only the new names. A later change removes `share/aliases.sh` and the `console-log` case.
