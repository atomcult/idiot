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

Doing that must not leak into the user's environment. `idiot init` and every
command leave PATH, LD_LIBRARY_PATH and the rest of the shell as they were;
the snap's tools get their library and lookup setup only in the process
that runs them.

## What Changes

- Commands are split into **confined** (state only, plus the bundled fzf)
  and **host** (anything else). A host command marks itself with a
  `# idiot: host` line; the dispatcher reads that marker.
- Under the snap, the confined entry point (`/snap/bin/idiot`, which the
  shell wrapper calls) answers a host command by writing one `env …`
  invocation of `/snap/<instance>/current/lib/idiot/idiot` to the eval file
  and exiting; the wrapper runs it in the user's shell. Help for a host
  command still runs confined. Outside a snap nothing changes: every
  command runs directly.
- The handoff sets only idiot's own variables (`IDIOT_ROOT`, `IDIOT_SNAP`,
  the data and cache directories). It never sets PATH or LD_LIBRARY_PATH.
- Host-side, `snap_app` runs a bundled tool (from the snap or its `core`
  component) with whatever it needs set for that one process; host tools
  (qemu, lxc, git, snap, ssh) run with the user's environment untouched.
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
- `shell-integration`: eval file location and evaluation order, the exit status rule for handoffs, confined/host command split, and the guarantee that idiot never changes tool-lookup variables in the user's shell.

## Impact

- `cmd/init`, `idiot` (dispatcher), `share/common.sh` (`emit_handoff`,
  `snap_app`, state-directory defaults), `cmd/run`, the `# idiot: host`
  marker in host commands, CLAUDE.md/STYLE.md (the split and the marker).
- `fix-snap-packaging` depends on this change; `native-store-login`'s
  helper runs only host-side.
