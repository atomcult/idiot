# idiot

Shell environment manager for Snap Store development. The shell wrapper
function (from `idiot init`) runs each command with `IDIOT_EVAL_FILE`
pointing at a private temp file; commands append `export`/`unset` statements
there and the wrapper sources it, so environment changes take effect in the
current shell. stdout is ordinary data output (never eval'd); messages,
progress and errors go to stderr.

## Commands

```sh
make hooks      # configure git to use .githooks/ (run once after cloning)
make check      # shfmt format-check + shellcheck (run before every commit)
make fmt        # rewrite files to match shfmt style
make install    # install to $PREFIX (default: ~/.local)
make uninstall
make purge      # uninstall + delete user data dirs
```

## Spec-driven changes (OpenSpec)

Non-trivial changes go through OpenSpec: propose with the `openspec-propose`
skill, implement with `openspec-apply-change`, then `openspec-archive-change`
(explore ideas first with `openspec-explore`). Changes live in
`openspec/changes/<name>/`; accepted behaviour in `openspec/specs/`. Project
context and artifact rules are in `openspec/config.yaml`. Check with
`openspec validate --all`.

## Git workflow

- **Always commit after making changes.** Do not leave the working tree dirty.
- **Use Conventional Commits** for all commit messages:
  `<type>(<scope>): <description>`
  Common types: `feat`, `fix`, `docs`, `refactor`, `chore`, `style`, `test`
  Examples:
  ```
  feat(init): add iot() alias for all shells
  fix(logo): use octal escapes for POSIX printf compatibility
  docs: trim README to selling points
  chore: add MIT LICENSE
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
| Export an env var | `emit_export VAR "${val}"` → eval file (quoted for you) |
| Unset an env var | `emit_unset VARNAME` → eval file (fish-aware) |
| Data the user asked for | plain `printf`/`cat` → stdout |

Commands that change the environment call `require_wrapper` right after
their help handling, before any side effect: it refuses to run under an
outdated wrapper (one without `IDIOT_EVAL_FILE`).

**Never** write `export`/`unset` statements to stdout, and never write
messages there: stdout is for data the command was asked to print
(`auth export`, `version --short`, child tools under `idiot run`), so it
must stay clean for pipes.

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
require_wrapper                 # die unless IDIOT_EVAL_FILE is usable
emit_export VARNAME "${val}"    # queue a quoted export in the eval file
emit_unset VARNAME              # queue unset (bash/zsh) or set --erase (fish)
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
