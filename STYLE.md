# Style guide

This document describes the conventions used throughout this codebase. All
new commands and helpers should follow them so that the project reads as one
coherent thing.

---

## Shell dialect

Everything is POSIX `sh`. No bashisms — no `[[`, no `$(( ))` arrays, no
process substitution, no `local` outside a function. The shebang is always:

```sh
#!/usr/bin/env sh
```

---

## File headers

Every script begins with a two-line header:

```sh
#!/usr/bin/env sh
# cmd/store/add - save a Dedicated Snap Store ID under a human-readable name
```

The comment on line 2 uses the actual file path relative to the project root,
followed by a dash and a one-line description in sentence-fragment style
(lowercase, no period).

Leaf commands that accept arguments add a `# Usage:` line on line 3:

```sh
#!/usr/bin/env sh
# cmd/store/add - save a Dedicated Snap Store ID under a human-readable name
# Usage: idiot store add <name> <store-id>
```

Commands that write eval-able output add an `# Outputs:` line after that:

```sh
#!/usr/bin/env sh
# cmd/arch/set - set the target architecture for store operations
# Usage: idiot arch set [<architecture>]
# Outputs: eval-able export statement
```

When more context is essential (e.g. `cmd/init`), additional comment lines
follow the header block before the first blank line.

---

## Sourcing `lib/common.sh`

Every command sources `lib/common.sh` immediately after the header, using a
path relative to its own location:

```sh
# One level deep (cmd/<command>):
. "$(dirname "${0}")/../lib/common.sh"

# Two levels deep (cmd/<group>/<subcommand>):
. "$(dirname "${0}")/../../lib/common.sh"
```

---

## Structure: the `main()` pattern

Every script wraps its logic in `main()` and calls it at the end of the file:

```sh
main() {
    ...
}
main "$@"
```

Private helper functions defined outside `main()` are prefixed with `_`:

```sh
_row() { ... }
_pick_from() { ... }

main() { ... }
main "$@"
```

---

## Help handling

The first `case` branch in `main()` always handles help. Single-action
commands use the one-liner form:

```sh
main() {
    case "${1:-}" in
        -h|--help|help) help_heading "Usage:" "idiot store add <name> <store-id>"; exit 0 ;;
    esac
    ...
}
```

Commands with richer help use the block form:

```sh
main() {
    case "${1:-}" in
        -h|--help|help)
            help_heading "Usage:" "idiot run <binary> [args...]"
            echo >&2
            say "Prose description here."
            echo >&2
            help_heading "Examples:"
            say "  idiot run sfdisk --json /dev/sda"
            exit 0 ;;
    esac
    ...
}
```

`help` files for command groups handle the empty-argument case as well:

```sh
main() {
    help_heading "Usage:" "idiot arch <subcommand>"
    echo >&2
    help_heading "Subcommands:"
    help_command "set"    "Set the target architecture"
    help_command "unset"  "Clear the target architecture"
    help_command "status" "Show the active architecture"
    echo >&2
    help_heading "Environment:"
    help_env "UBUNTU_STORE_ARCH" "Target architecture override"
}
main "$@"
```

### Help formatting functions

All help output goes through the helpers in `lib/common.sh`:

| Function | Purpose | Column width |
|---|---|---|
| `help_heading "Label:" ["text"]` | Bold blue section heading | — |
| `help_command "name" "desc"` | Cyan command name + description | 14 chars |
| `help_option  "--flag" "desc"` | Bold option name + description | 22 chars |
| `help_env     "VAR" "desc"` | Bold env var name + description | 22 chars |

Blank lines between sections use `echo >&2`, never `say ""`.

---

## Output discipline

All user-visible text goes to **stderr** so it reaches the terminal directly
without being eval'd by the shell wrapper.

| Situation | How to output |
|---|---|
| Message to the user | `say "text"` |
| Blank line | `echo >&2` |
| Error + exit | `die "message"` |
| Eval-able env change | `printf 'export VAR=%s\n' "$(shell_quote "${val}")"` to **stdout** |
| Eval-able env unset | `emit_unset VARNAME` to **stdout** |

Never use bare `printf ... >&2` for user messages — use `say`. The only
legitimate `printf` to stderr calls are inside helpers in `lib/common.sh`
itself.

---

## Eval-able output

Commands that modify the shell environment write to **stdout** only. The shell
wrapper (`idiot()` function installed by `idiot init`) evals this output so
changes take effect in the current shell session.

**Setting a variable:**

```sh
printf 'export UBUNTU_STORE_ID=%s\n' "$(shell_quote "${store_id}")"
```

Always use `shell_quote` for values — credentials, paths, and names can
contain characters that would break unquoted eval.

**Unsetting a variable:**

```sh
emit_unset UBUNTU_STORE_ID
```

`emit_unset` in `lib/common.sh` emits `unset VAR` for bash/zsh and
`set --erase VAR` for fish, keyed on `$IDIOT_SHELL`.

Commands that only display information (no env changes) produce no stdout
output at all.

---

## Error messages

`die` messages are lowercase, no trailing period. Describe what went wrong;
add a hint after ` — ` when it's not obvious what to do next:

```sh
die "credentials '${name}' not found"
die "destination '${dest}' already exists"
die "no credentials found — use 'idiot auth import' to add credentials"
die "unsupported shell: ${shell} (supported: bash, zsh, fish)"
```

---

## Dependency checks

Use `require` for mandatory external commands, not inline `command -v` tests:

```sh
require git
require fzf "install fzf for interactive selection"
require unsquashfs "install squashfs-tools"
```

The second argument is an install hint. Omit it only when the command name is
self-explanatory. Use `command -v` directly only for **optional** tools where
the code has a fallback path rather than dying.

---

## Variable quoting and defaults

Always double-quote variable expansions. Use `${VAR:-}` (empty default) for
variables that might be unset to avoid `set -e` / `set -u` failures:

```sh
[ -n "${IDIOT_ACTIVE_CRED:-}" ] || ...
name="${1:-}"
```

Never use unquoted `$VAR` except inside `[ ]` numeric comparisons where the
value is known safe.

---

## Colors

Color variables are defined in `lib/common.sh` and are empty strings when
output is not a TTY or `NO_COLOR` is set. Never hardcode ANSI escape sequences
— always use the variables.

| Variable | Use for |
|---|---|
| `${BOLD}` | Active values, important names |
| `${DIM}` | Default/inactive values, hints, annotations |
| `${CYAN}` | Command/label names in help output |
| `${RED}` | Errors (via `die` — don't use directly) |
| `${BOLD}${BLUE}` | Section headings (via `help_heading` — don't use directly) |
| `${RESET}` | Always close every color open |

Active (explicitly set) values are bold; default/fallback values are dim:

```sh
[ -n "${UBUNTU_STORE_ARCH:-}" ] \
    && say "${BOLD}${UBUNTU_STORE_ARCH}${RESET}" \
    || say "${DIM}amd64${RESET}"   # host default — not explicitly set
```

---

## Interactive pickers (fzf)

When an argument is optional and fzf is available, offer an interactive picker.
Guard the picker with `require fzf "..."`. Use `|| exit 0` after the fzf call
so that pressing Escape cancels cleanly:

```sh
name="${1:-}"
[ -n "${name}" ] || {
    require fzf "install fzf for interactive selection"
    name=$(printf '%s\n' "${entries}" | fzf --prompt="store> ") || exit 0
    [ -n "${name}" ] || exit 0
}
```

Prompt format is `"noun> "` — lowercase, matches the thing being selected,
space before `>`.

---

## Directory layout

```
idiot               Main entry point and dispatcher
cmd/                Command implementations
  <command>         Leaf command (executable file)
  <group>/          Command group (directory)
    help            Required: usage text for the group
    <subcommand>    Leaf subcommand (executable file)
lib/                Shared libraries (sourced, not executed)
  common.sh         Core helpers — every command sources this
share/              Bundled read-only data (keyed by data type)
```

Every command group directory **must** have a `help` executable. The
`dispatch_subcmd` router in `lib/common.sh` calls it when the user runs
`idiot <group>` with no arguments or with `help`/`-h`/`--help`.

---

## The dispatcher and `IDIOT_SHELL`

Most commands require the shell wrapper to be active (`IDIOT_SHELL` set).
The check lives in `dispatch_subcmd` and in the main `idiot` dispatcher.

Commands that work without shell initialisation — currently `version`, `init`,
and `status` — are special-cased **before** the `IDIOT_SHELL` check in the
main `idiot` script. A command belongs in this set only if it is purely
display-only and cannot modify the shell environment.

---

## `lib/common.sh` helpers reference

| Helper | Signature | Notes |
|---|---|---|
| `say` | `say "text"` | Print to stderr |
| `die` | `die "message"` | Print error to stderr and exit 1 |
| `require` | `require cmd ["hint"]` | Die if `cmd` not in PATH |
| `shell_quote` | `shell_quote "${val}"` | POSIX single-quote escaping for eval |
| `emit_unset` | `emit_unset VARNAME` | Write shell-appropriate unset to stdout |
| `snap_app` | `snap_app cmd [args...]` | Run snap-bundled or system binary |
| `file_preview_cmd` | `file_preview_cmd VARNAME` | Return fzf `--preview` string for a directory |
| `help_heading` | `help_heading "Label:" ["text"]` | Section heading |
| `help_command` | `help_command "name" "desc"` | Command row (14-char column) |
| `help_option` | `help_option "--flag" "desc"` | Option row (22-char column) |
| `help_env` | `help_env "VAR" "desc"` | Env var row (22-char column) |
| `git` | `git [args...]` | `command git` wrapped with DIM colour |
