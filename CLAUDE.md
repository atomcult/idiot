# idiot

Shell environment manager for Snap Store development. The shell wrapper
function (from `idiot init`) runs each command with `IDIOT_EVAL_FILE`
pointing at a private temp file; commands append `export`/`unset` statements
there and the wrapper sources it, so environment changes take effect in the
current shell. stdout is ordinary data output (never eval'd); messages,
progress and errors go to stderr.

## Confined and host commands

The snap is strictly confined with no interfaces. Confined, idiot only
changes its own state (plus the bundled fzf). **A command runs confined only
if it touches nothing but idiot's state and needs no tool but fzf;
everything else is a host command** and carries a `# idiot: host` line in
its header. Under the snap the dispatcher hands a host command (unless it
is asked for help) to the user's shell with `emit_handoff`: the eval file
gets one quoted invocation of `/snap/<instance>/current/lib/idiot/idiot`,
which runs outside confinement. Outside a snap every command runs directly.

- `idiot init` (from the snap) exports `IDIOT_SNAP`, `IDIOT_DATA_HOME` and
  `IDIOT_CACHE_DIR`: that is the host side's environment. Never `SNAP`.
- **Never** export PATH or LD_LIBRARY_PATH into the user's shell. A bundled
  tool gets whatever it needs only in its own process (`tool`,
  `snap_app`, `require`'s fallback); host tools run with the user's
  environment.
- Bundled tools live in the snap's optional components (`core vm tpm
  network hardware boot debug`, `IDIOT_COMPONENTS`), each mounted at
  `/snap/<instance>/components/<rev>/<name>`. The snap build writes an
  index per component to `share/components/<name>` (one tool per line),
  so a missing tool's error names its component even when it isn't
  installed.
- Host commands can't set variables (they run after the eval file is
  read). A command that needs to both use host tools and export must be
  split.

## Commands

```sh
make hooks      # configure git to use .githooks/ (run once after cloning)
make check      # shfmt + shellcheck, cargo fmt --check + clippy (before every commit)
make fmt        # rewrite files to match shfmt / rustfmt style
make install    # install to $PREFIX (default: ~/.local); builds idiot-store (needs cargo)
make install-sh # the shell tree only (the snap builds idiot-store in its own part)
make uninstall
make purge      # uninstall + delete user data dirs
snap/local/check-components <snap>   # after snapcraft pack: promised tools in each index
tests/bare-host [--boot download --kvm] <snap> [<comp>...]
                # the snap + components in a bare local: LXD container: every
                # tool runs, data/helpers/wrappers work, idiot run, refresh,
                # (--boot) Ubuntu Core boots from vm+tpm; checks in tests/bare-host.d/
```

Test containers go on `local:` (or `--remote`): the default LXD remote on
Lauren's laptop is her NAS.

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
                 #   dispatch_subcmd, tool/tool_path/snap_app, git(), shell_quote,
                 #   require_wrapper, emit_export, emit_unset
  arch.sh        # uname_to_snap_arch(), snap_arch_to_qemu()
  clean.sh       # clean_list_*/clean_remove_*, clean_show, clean_confirm
  creds.sh       # IDIOT_AUTH_DIR, validate_cred_name, ensure_auth_dir,
                 #   require_cred/no_cred, whoami_cache_path, store_helper,
                 #   STORE_PERMISSIONS, map_permission, is_store_permission
  examples.sh    # IDIOT_EXAMPLES_DIR, resolve_example_url
  fzf.sh         # file_preview_cmd, fzf_pick_from_dir
  lp.sh          # IDIOT_LP_USER_FILE, lp_user_value, require_lp_user
  models.sh      # IDIOT_MODELS_DIR, require_models_repo
  stores.sh      # IDIOT_STORES_DIR, require_store/no_store
  vm-firmware.sh # vm_firmware (the UEFI images idiot vm boots; also prunes
                 #   the snap's vm component), vm_firmware_secboot/_package
  uefi-vars.py   # run, not sourced: lists an OVMF/AAVMF variable store
                 #   (idiot vm vars); python3 standard library only
  examples/      # bundled static example alias files
  kernels/       # bundled static kernel data files
tools/idiot-store/ # Rust helper: Snap Store login and whoami (no snapcraft
                 #   needed for `idiot auth`); installed to lib/idiot/libexec/.
                 #   A leaf tool: sh owns the UX, it only talks to the store.
                 #   Credentials never in argv or stdout. Exit 0/1 usage/2 auth/3 store.
snap/local/      # build-time scripts for the snap: patchelf-host, pull-tracked,
                 #   component-build (finish a tool component's part),
                 #   wrap/<tool> (setup a bundled tool needs, see component-build),
                 #   check-components <snap> (promised tools are in the indexes)
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
- No `set -e` in commands or modules (handle failures with `|| die`); the
  `idiot` dispatcher alone has `set -e`
- Always double-quote expansions: `"${VAR}"`, not `$VAR`
- Use `${VAR:-}` for optional variables to avoid unbound-variable errors
- Indentation is shfmt's call (`make fmt`): 4 spaces in `share/*.sh`
  (`.editorconfig`), tabs in `idiot` and `cmd/*` (no extension, shfmt's
  default)
- `die` inside `$(…)` only exits the command substitution: write
  `var=$(helper_that_may_die) || exit 1`

## Key helpers (from share/common.sh)

```sh
say "message"                   # printf to stderr
die "message"                   # say error + exit 1
require cmd ["install hint"]    # die unless cmd is on PATH or bundled (host first);
                                #   a bundled identifier-named cmd becomes a function
tool cmd [args]                 # run cmd (host's, else bundled); any name, e.g.
                                #   qemu-system-x86_64, which require can't wrap
tool_path cmd                   # print the path tool would run, or fail
missing_tool cmd ["hint"]       # die naming the component that has cmd
include module.sh               # source share/<module.sh> via IDIOT_ROOT
shell_quote "${val}"            # POSIX single-quote escape for eval-safe output
require_wrapper                 # die unless IDIOT_EVAL_FILE is usable
emit_export VARNAME "${val}"    # queue a quoted export in the eval file
emit_unset VARNAME              # queue unset (bash/zsh) or set --erase (fish)
snap_app cmd [args]             # run a bundled tool, else the host's (idiot run)
snap_app_path cmd               # print the path snap_app would run, or die
emit_handoff word...            # queue a host-side run of idiot <word...>
dispatch_subcmd name dir [args] # exec cmd/<group>/<subcommand>
```

Help formatters (all output to stderr via `say`):
```sh
help_heading "Label:" ["text"]    # bold blue
help_command "name" "desc"        # cyan, 14-char name column
help_option  "--flag" "desc"      # bold, 22-char name column
help_env     "VAR" "desc"         # bold, 22-char name column
```

## State paths

```
IDIOT_DATA_HOME          IDIOT_AUTH_DIR, IDIOT_STORES_DIR, IDIOT_MODELS_DIR,
                         IDIOT_EXAMPLES_DIR, IDIOT_LP_USER_FILE live here
                         source: ~/.local/share/idiot  snap: ~/snap/idiot/common
IDIOT_CACHE_DIR          whoami cache, example mirrors
                         source: ~/.cache/idiot        snap: ~/snap/idiot/common/cache
~/.local/lib/idiot/      installed cmd/ and share/ (IDIOT_ROOT when installed)
```

`share/common.sh` sets `IDIOT_DATA_HOME`/`IDIOT_CACHE_DIR`; the per-item
variables follow the pattern:
```sh
IDIOT_AUTH_DIR="${IDIOT_AUTH_DIR:-${IDIOT_DATA_HOME}/auth}"
```

## The `git()` wrapper

`share/common.sh` defines `git()` to wrap `command git` with DIM/RESET on
**stderr**. The colors are explicitly on stderr so `git archive | tar` and
similar pipelines are never corrupted by escape sequences on stdout.

## Style guide

`STYLE.md` in the repo root is the canonical reference for naming, formatting,
color usage, error message phrasing, and everything else not covered here.
