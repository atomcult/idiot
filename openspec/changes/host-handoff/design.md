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
the user's environment never altered by idiot beyond the variables a
command exists to set (`UBUNTU_STORE_*`, `SNAPCRAFT_*`, `IDIOT_*`, …);
source installs behave exactly as now.

**Non-Goals:** sandboxing the host side (it runs as the user, like a
source install); running host commands without the shell wrapper.

## Decisions

### The rule
A command runs confined only if it touches nothing but idiot's own state
and needs no tool but fzf. Everything else is a host command. When in
doubt, host: running host-side loses nothing a source install has.

### Marker, not a list
Each host command's script carries `# idiot: host` in its header. The
dispatcher, when `$SNAP` is set and `IDIOT_HOST` is not, checks the
resolved command file with `grep -qx '# idiot: host'` and hands off.
Alternative: a list in the dispatcher. Rejected: it drifts from the
scripts, and the marker documents the command where it's read.

Help (`help`, `-h`, `--help` as the first argument) isn't handed off: it
needs nothing from the host.

### The handoff line
`emit_handoff <command> [args…]` (common.sh) requires the wrapper, writes
one line to the eval file and the dispatcher then exits 0:

```sh
env IDIOT_HOST=1 IDIOT_SNAP='/snap/idiot/current' IDIOT_ROOT='/snap/idiot/current/lib/idiot' IDIOT_DATA_HOME='…' IDIOT_CACHE_DIR='…' '/snap/idiot/current/lib/idiot/idiot' 'vm' '--arch' 'arm64'
```

- `env` runs the program as a child of the user's shell with those
  variables in its environment only: the same line works in bash, zsh and
  fish, and nothing is set in the shell itself.
- Every word goes through `shell_quote`, whose `'…'\''…'` form is also
  valid fish.
- `IDIOT_EVAL_FILE` and `IDIOT_SHELL` are inherited, so a host command
  that sets variables (none today) can still emit exports; they are
  evaluated by the next call, so host commands don't set variables. A
  host command that ever needs to must be split.
- The path uses `current`, never `$SNAP` (one revision; gone after two
  refreshes).
- `$SNAP` is deliberately *not* passed: snapcraft and other tools change
  behaviour when it is set. Host-side code tests `IDIOT_SNAP`.

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

`idiot run --shell` execs `$SHELL` with the bundled tool directories
prepended to PATH, and nothing else; it is a child shell, so the caller is
unaffected.

### State directories
`IDIOT_DATA_HOME` replaces the scattered `${XDG_DATA_HOME:-…}/idiot`
defaults in `share/*.sh` (auth, stores, models, model-repos, lp-user). It
defaults to `$SNAP_USER_COMMON` in the confined snap, to the handed-off
value on the host side, and to `${XDG_DATA_HOME:-~/.local/share}/idiot`
for a source install; `IDIOT_CACHE_DIR` likewise. The per-item overrides
(`IDIOT_AUTH_DIR`, …) keep working.

## Risks / Trade-offs

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
