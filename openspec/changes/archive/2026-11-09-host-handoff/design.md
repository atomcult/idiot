# Design

## Context

The wrapper from `eval-output-channel` gives each call a private eval file
(`IDIOT_EVAL_FILE`) and sources it afterwards. Under strict confinement
with no plugs the snap can write only `$SNAP_USER_DATA`,
`$SNAP_USER_COMMON` and `/run/user/$UID/snap.<instance>/`, can't open the
network, and can't execute host binaries. The user's shell, which sources
the eval file, has none of those limits and can read the snap's files at
`/snap/<instance>/current/`, a symlink that follows refreshes.

What each command needs today:

| Runs confined | Runs on the host |
|---|---|
| `arch *`, `lp *`, `store *`, `status`, `version`, `init` | `vm`, `run`, `inspect *`, `clean *`, `example`, `kernel` |
| `auth list/login/logout/remove` | `auth add`, `auth status` (network), `auth import/export` (user's files) |
| `model add/list/remove` | `model update`, `model pick` (git) |

## Goals / Non-Goals

**Goals:** a strict snap with no interfaces that can still do everything;
`idiot init` sets up the environment the host side relies on, while PATH
and LD_LIBRARY_PATH are only ever set as needed, per process; source
installs behave exactly as now.

**Non-Goals:** sandboxing the host side (it runs as the user, like a
source install); running host commands without the shell wrapper.

## Decisions

### The rule
A command runs confined only if it touches nothing but idiot's own state
and needs no tool but fzf. Everything else is a host command. When in
doubt, host: running host-side loses nothing a source install has.

### Marker, not a list
Each host command's script carries `# idiot: host` in its header. The
dispatcher, when `$SNAP` is set (i.e. it is running confined), checks the
resolved command file with `grep -qx '# idiot: host'` and hands off.
Alternative: a list in the dispatcher. Rejected: it drifts from the
scripts, and the marker documents the command where it's read.

Help (`help`, `-h`, `--help` as the first argument) isn't handed off: it
needs nothing from the host.

### What `init` exports
Generated from the snap (`$SNAP` set), the integration also exports:

- `IDIOT_SNAP=/snap/<instance>/current`
- `IDIOT_DATA_HOME=$SNAP_USER_COMMON`, `IDIOT_CACHE_DIR=$SNAP_USER_COMMON/cache`

These are stable across refreshes (`current`, `common`), so a shell
initialised once stays right. They are how the host side gets its
environment: no per-call plumbing. Inside confinement snapd passes them
through, and they agree with what the confined side computes itself.
A source install's integration exports only `IDIOT_SHELL`, as today.

PATH and LD_LIBRARY_PATH are not exported, ever: putting the snap's
directories there would shadow the host's tools and load core24's
libraries into every program the user runs.

### The handoff line
`emit_handoff <command> [args…]` (common.sh) requires the wrapper, writes
one line to the eval file and the dispatcher then exits 0:

```sh
'/snap/idiot/current/lib/idiot/idiot' 'vm' '--arch' 'arm64'
```

- It runs as a child of the user's shell, with the shell's environment
  (including what `init` exported). The same line works in bash, zsh and
  fish; every word goes through `shell_quote`, whose `'…'\''…'` form is
  also valid fish.
- The host side knows it is the host side because `$SNAP` is unset and
  `IDIOT_SNAP` is set. If `IDIOT_SNAP` is missing (a shell initialised
  before this change), it refuses and says to re-run `idiot init`.
- `IDIOT_EVAL_FILE` and `IDIOT_SHELL` are inherited, so a host command
  that sets variables (none today) can still emit exports; they are
  evaluated by the next call, so host commands don't set variables. A
  host command that ever needs to must be split.
- The path uses `current`, never `$SNAP` (one revision; gone after two
  refreshes).
- `$SNAP` is deliberately never exported to the user's shell: snapcraft
  and other tools change behaviour when it is set. Host-side code tests
  `IDIOT_SNAP`.

### Exit status
The wrapper returns the command's status when it is non-zero, and
otherwise the status of evaluating the file. For ordinary exports that is
0; for a handoff it is the host command's status.

### Read, delete, then evaluate
bash/zsh: `_c="$(cat "$f")"; rm -f "$f"; eval "$_c"`; fish:
`set -l _c (cat $f | string collect); rm -f $f; eval $_c`. A handed-off
command can run for hours (a VM); if Ctrl-C aborted the function during
it, the old order left the file behind. The file now exists only while the
confined command runs, which already traps INT.

### Eval file location
`idiot init` knows whether it runs from the snap and bakes the directory
in: `${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/snap.<instance>` under the
snap (the wrapper creates it with `mkdir -p -m 700` if snapd hasn't yet),
`${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}` otherwise. The confined process sees
the same path as its own `XDG_RUNTIME_DIR`, which the default AppArmor
template lets it write.

### Bundled tools, scoped
`snap_app <tool> [args…]`, host-side with `IDIOT_SNAP` set, looks the tool
up in `$IDIOT_SNAP/usr/bin`, `$IDIOT_SNAP/bin`, then the `core` component
(`fix-snap-packaging` pins down the path) and runs it with the PATH and
library setup it needs **for that process only**. Host tools are called by
name with the user's environment.

How a bundled binary finds core24's libraries is settled while packaging
(`fix-snap-packaging`): binaries patched by snapcraft's `enable-patchelf`
(interpreter and RPATH pointing into `/snap/core24/current` and the snap)
need nothing at run time, so nothing can leak into tools they start (fzf
starts `sh` for previews; an inherited LD_LIBRARY_PATH would load core24's
libc into the host's sh). `snap_app` falls back to `env
LD_LIBRARY_PATH=…` for that one tool only if patching isn't possible.

Host commands call tools by name after `require`. On the host side of the
snap, `require` turns a tool the host lacks but the snap bundles (fzf on a
bare host, debugfs/mcopy/python3 from `core` on Ubuntu Core) into a shell
function that runs the bundled copy through `snap_app`, so callers don't
change and the user's PATH is never touched. The host's own copy wins when
it exists.

`idiot run --shell` execs `$SHELL` with the bundled tool directories
prepended to PATH (the user asked for exactly that), and LD_LIBRARY_PATH
only if bundled binaries need it (see above); it is a child shell, so the
caller is unaffected.

### State directories
`IDIOT_DATA_HOME` replaces the scattered `${XDG_DATA_HOME:-…}/idiot`
defaults in `share/*.sh` (auth, stores, models, model-repos, lp-user). It
defaults to `$SNAP_USER_COMMON` in the confined snap, comes from `init`'s
export on the host side, and defaults to `${XDG_DATA_HOME:-~/.local/share}/idiot`
for a source install; `IDIOT_CACHE_DIR` likewise. The per-item overrides
(`IDIOT_AUTH_DIR`, …) keep working.

### Pickers that preview account status
`auth login` and `auth remove` stay confined (login exports), but their
fzf preview ran `auth status`, which contacts the store. Under the snap the
preview runs `auth status --cached`: cached details, or a note that they
haven't been fetched. Source installs keep the live preview.

## Risks / Trade-offs

- [`IDIOT_DATA_HOME` exported into the shell] → a source install run in the
  same shell would use the snap's state. Having both installed at once is
  not a supported setup; the variable is visible in `env`, so it's easy
  to spot.

- [A host command forgets its marker] → it runs confined and fails on its
  first tool or network call with a confinement error. Task 3.x greps
  every command for tool calls; CLAUDE.md states the rule.
- [Host commands can't set variables] → none do today; documented above.
- [`env` resolution] → the line calls `env` by name from the user's PATH,
  as any shell command would; the program it runs is an absolute path.
- [Outdated wrappers] → caught by the existing `require_wrapper` check
  (eval file missing) for confined env-setting commands, and the handoff
  calls `require_wrapper` too, so an old wrapper never half-runs a host
  command.

## Migration Plan

Re-run `idiot init` in open shells (the error says so). Source-install data
stays where it is.
