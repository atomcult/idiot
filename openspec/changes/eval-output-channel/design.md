# Design

## Context

Today `cmd/init` defines `idiot()` as `_out="$(idiot "$@")"; eval "$_out"`
(fish: `set -l _out (idiot $argv)` then `eval` per line). Env-setting commands
`printf 'export …'` to stdout; `emit_unset` prints `unset`/`set --erase`.
Interactive pickers already work because fzf draws on `/dev/tty`. See
proposal.md for why stdout has to be freed.

## Goals / Non-Goals

**Goals:**
- One channel for shell code that never carries anything else.
- Identical behaviour in bash, zsh and fish.
- Old wrappers fail loudly instead of silently dropping exports.

**Non-Goals:**
- Changing what any command exports, or the export format itself.
- Supporting shells other than bash, zsh and fish.

## Decisions

### A per-call temp file, not a spare file descriptor
The wrapper runs `IDIOT_EVAL_FILE="$(mktemp)" idiot …`, then sources the file
and deletes it. Commands append their statements to `"${IDIOT_EVAL_FILE}"`.

Alternatives considered:
- **fd 3** (`{ out="$(idiot 3>&1 1>&4)"; } 4>&1`): works in bash, but in
  fish 3.7 a block-level `4>&1` is not honoured (tested: the child's stdout
  write fails with "I/O error"). It is also fragile: an fd 3 inherited from
  a parent process (seen in practice in a tool runner) would make an old or
  mis-set wrapper write exports somewhere arbitrary, and "is fd 3 open?" can't
  tell the wrapper's fd from a stray one.
- **Keep stdout, add a sentinel prefix** (only eval lines starting with
  `__IDIOT__`): still mixes data with code; any tool output beginning with the
  sentinel would execute.

The file is the only option that is shell-agnostic, keeps stdout and stderr
fully untouched, and can be validated (it must exist and be owned by us).

### File location and permissions
`mktemp` under `${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}` (tmpfs, per-user, 0700 on
systemd systems); `mktemp` creates files 0600. The file may briefly hold a
credential (`auth login` exports it), so it is deleted by the wrapper right
after sourcing, on every exit path (bash/zsh: delete after eval regardless of
status; fish: same, in the function body). Commands never print it.

### Writing statements
Two helpers in `share/common.sh` replace direct `printf 'export …'`:
`emit_export VAR value` (uses `shell_quote`) and the existing `emit_unset`,
both appending to `"${IDIOT_EVAL_FILE}"`. CLAUDE.md's output table points at
them. `require_wrapper` (called by env-setting commands before doing anything)
dies with the reload hint when `IDIOT_EVAL_FILE` is unset or not a writable
regular file.

Fish sources the file with `source`, which accepts `export VAR='…'` (fish's
`export` wrapper) and `set --erase`, so the format is unchanged.

### Detection of the outdated wrapper
The old wrapper sets `IDIOT_SHELL` but not `IDIOT_EVAL_FILE`. Checking for
the file in env-setting commands only (not in printing commands) keeps
`idiot auth export`, `idiot run`, etc. usable from an old shell.

## Risks / Trade-offs

- [Credential briefly on disk] → tmpfs location, 0600, deleted immediately;
  same exposure as the existing credential files, which already live on disk.
- [Interrupted wrapper leaves a file] → tested: when the command dies from
  SIGINT, bash and fish abort the wrapper function before its cleanup; when
  the command catches INT and exits 130, the function carries on. So
  `share/common.sh` sets `trap 'exit 130' INT` for every command, and the
  wrapper also deletes a leftover file from an earlier call (tracked in a
  global `_idiot_ef`) before starting a new one. A SIGKILL'd command or
  parent shell can still leave a 0600 file in the runtime dir until the next
  call or logout.
- [Users in long-lived shells] → the outdated-wrapper error tells them the
  one command to run.

## Migration Plan

Ship wrapper and commands together. Existing shells hit the outdated-wrapper
error once and re-run `idiot init`; rc files need no change since they call
`idiot init` on every new shell. Rollback is reverting the commit; old
wrappers keep working with old commands.
