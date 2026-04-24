# idiot

Shell environment manager for Snap Store development. Commands output
eval-able `export`/`unset` statements to stdout; the shell wrapper function
captures and evals them so environment changes take effect in the current
shell. All user-visible output (progress, errors) goes to stderr.

## Commands

```sh
make check      # shfmt format-check + shellcheck (run before every commit)
make fmt        # rewrite files to match shfmt style
make install    # install to $PREFIX (default: ~/.local)
make uninstall
make purge      # uninstall + delete user data dirs
```

## Project layout

```
idiot            # main dispatcher; sets IDIOT_ROOT, sources share/common.sh
cmd/             # one file per leaf command; dirs for subcommand groups
share/           # sourced helper modules (never executed directly)
  common.sh      # universal: colors, say/die/require/include, help formatters,
                 #   dispatch_subcmd, snap_app, git(), emit_unset, shell_quote
  arch.sh        # uname_to_snap_arch(), snap_arch_to_qemu()
  creds.sh       # IDIOT_AUTH_DIR, require_cred/no_cred, whoami_cache_path
  examples.sh    # IDIOT_EXAMPLES_DIR, resolve_example_url
  fzf.sh         # file_preview_cmd, fzf_pick_from_dir
  lp.sh          # IDIOT_LP_USER_FILE, lp_user_value, require_lp_user
  models.sh      # IDIOT_MODELS_DIR, require_models_repo
  stores.sh      # IDIOT_STORES_DIR, require_store/no_store
  examples/      # bundled static example alias files
  kernels/       # bundled static kernel data files
```

## Every cmd/ file follows this structure

```sh
#!/usr/bin/env sh
# cmd/path/to/file - one-line description

. "$(dirname "${0}")/../../share/common.sh"   # adjust depth as needed
include creds.sh                               # source only the modules needed
include fzf.sh

_helper() { ... }   # private helpers above main(), prefixed _

main() {
    case "${1:-}" in
    -h | --help | help)
        help_heading "Usage:" "idiot <cmd> [args]"
        exit 0
        ;;
    esac
    # ...
}
main "$@"
```

`include module.sh` uses `IDIOT_ROOT` (always set by the wrapper before any
cmd file runs). The first `common.sh` source must still use `$(dirname "${0}")`
because `include` itself is not available yet.

## Output discipline

| What | How |
|---|---|
| User messages | `say "text"` → stderr |
| Blank lines | `echo >&2` |
| Errors + exit | `die "message"` |
| Export an env var | `printf 'export VAR=%s\n' "$(shell_quote "${val}")"` → stdout |
| Unset an env var | `emit_unset VARNAME` → stdout (fish-aware) |

**Never** write display output to stdout. The shell wrapper evals everything
on stdout; stray output breaks the user's shell session.

**Always** use `shell_quote` when building export statements. Bare variable
expansion in printf format strings breaks on values with spaces or special
characters.

## Shell dialect

POSIX `sh` only — no bashisms. Specifically:
- No `[[`, `local`, `$'...'`, process substitution `<(...)`, or `(( ))`
- No `set -e` at the top of files
- Always double-quote expansions: `"${VAR}"`, not `$VAR`
- Use `${VAR:-}` for optional variables to avoid unbound-variable errors
- 4-space indent; enforced by shfmt (`.editorconfig` carries the config)

## Key helpers (from share/common.sh)

```sh
say "message"                   # printf to stderr
die "message"                   # say error + exit 1
require cmd ["install hint"]    # die unless cmd is in PATH
include module.sh               # source share/<module.sh> via IDIOT_ROOT
shell_quote "${val}"            # POSIX single-quote escape for eval-safe output
emit_unset VARNAME              # unset (bash/zsh) or set --erase (fish)
snap_app cmd [args]             # run cmd, resolving into $SNAP/bin if in snap
dispatch_subcmd name dir [args] # exec cmd/<group>/<subcommand>
```

Help formatters (all output to stderr via `say`):
```sh
help_heading "Label:" ["text"]    # bold blue
help_command "name" "desc"        # cyan, 14-char name column
help_option  "--flag" "desc"      # bold, 22-char name column
help_env     "VAR" "desc"         # bold, 22-char name column
```

## XDG paths

```
~/.local/share/idiot/    IDIOT_AUTH_DIR, IDIOT_STORES_DIR, IDIOT_MODELS_DIR,
                         IDIOT_EXAMPLES_DIR, IDIOT_LP_USER_FILE
~/.cache/idiot/          IDIOT_CACHE_DIR (whoami cache, example mirrors)
~/.local/lib/idiot/      installed cmd/ and share/ (IDIOT_ROOT when installed)
```

All path variables follow the pattern:
```sh
IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${XDG_DATA_HOME:-${HOME}/.local/share}/idiot/auth}"
```

## The `git()` wrapper

`share/common.sh` defines `git()` to wrap `command git` with DIM/RESET on
**stderr**. The colors are explicitly on stderr so `git archive | tar` and
similar pipelines are never corrupted by escape sequences on stdout.

## Style guide

`STYLE.md` in the repo root is the canonical reference for naming, formatting,
color usage, error message phrasing, and everything else not covered here.
