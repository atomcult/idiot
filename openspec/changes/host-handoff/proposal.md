# Proposal

## Why

Strict confinement with no interfaces is the right shape for the idiot snap
(see `fix-snap-packaging`), but only for commands that do nothing except
change idiot's own state: saving a credential name, picking a store,
setting the target architecture. Everything else (qemu, lxc, git, the
network, disk images, files the user names) can't run inside a snap with no
plugs, and shouldn't need to: idiot already hands work back to the user's
shell through the eval file. That same channel can hand a command over to a
host-side copy of idiot, run from the snap's files, outside confinement.

`idiot init` is how the host side gets its environment: it exports the
variables idiot's host-side code needs (where the snap is, where its state
lives) into the user's shell. What it must not do is put the snap's tools
on PATH or its libraries on LD_LIBRARY_PATH for the whole shell; those are
set as needed, only in the process that runs a bundled tool.

## What Changes

- Commands are split into **confined** (state only, plus the bundled fzf)
  and **host** (anything else). A host command marks itself with a
  `# idiot: host` line; the dispatcher reads that marker.
- Under the snap, the confined entry point (`/snap/bin/idiot`, which the
  shell wrapper calls) answers a host command by writing one quoted
  invocation of `/snap/<instance>/current/lib/idiot/idiot` to the eval file
  and exiting; the wrapper runs it in the user's shell. Help for a host
  command still runs confined. Outside a snap nothing changes: every
  command runs directly.
- Under the snap, `idiot init` also exports `IDIOT_SNAP`
  (`/snap/<instance>/current`), `IDIOT_DATA_HOME` and `IDIOT_CACHE_DIR`
  (the snap's common directory), so host-side code finds the snap and the
  same state as the confined side. The handoff line itself sets nothing.
  Neither ever sets PATH or LD_LIBRARY_PATH.
- Host-side, `snap_app` runs a bundled tool (from the snap or its `core`
  component) with PATH/LD_LIBRARY_PATH set as needed for that one process;
  host tools (qemu, lxc, git, snap, ssh) run with the user's PATH and
  libraries.
  `idiot run --shell` gives a child shell where bundled tools come first on
  PATH; the calling shell is unchanged.
- The wrapper:
  - creates the eval file where the confined command can write it:
    `$XDG_RUNTIME_DIR/snap.<instance>/` under the snap (created 0700 if
    missing), as today otherwise;
  - reads the file, deletes it, then evaluates what it read, so cleanup no
    longer depends on the command handling Ctrl-C;
  - returns the command's status, or, when the command exited zero, the
    status of what it handed off.
- **BREAKING** for shells that loaded the current integration: they must
  re-run `idiot init` (the outdated-wrapper check already says so).
- stdout: unchanged. A handed-off command's stdout is the host-side
  command's stdout, still never evaluated.
- No new dependencies.

## Capabilities

### New Capabilities

### Modified Capabilities
- `auth`: under the snap the login/remove picker previews show cached status only (`auth status --cached`), since they run confined.
- `shell-integration`: eval file location and evaluation order, the exit status rule for handoffs, the confined/host command split, the variables `init` exports under the snap, and PATH/LD_LIBRARY_PATH set only per process, as needed.

## Impact

- `cmd/init`, `idiot` (dispatcher), `share/common.sh` (`emit_handoff`,
  `snap_app`, state-directory defaults), `cmd/run`, the `# idiot: host`
  marker in host commands, CLAUDE.md/STYLE.md (the split and the marker).
- `fix-snap-packaging` depends on this change; `native-store-login`'s
  helper runs only host-side.
