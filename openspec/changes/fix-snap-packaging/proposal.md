# Proposal

## Why

The snap doesn't behave like the `make install` build:

- `idiot init` wrote the absolute path of the script inside one snap
  revision, `/snap/idiot/<revision>/lib/idiot/idiot`, into the user's
  shell; it broke after refreshes and ran the script outside the snap.
- The snap is strictly confined with no plugs, but nothing made idiot work
  that way: its commands tried to reach the network, host tools and
  `~/.local/share` from inside confinement.
- The `core` component is declared but no part puts files in it.

## What Changes

- The snap stays **strictly confined with no interfaces**. Confined, idiot
  only changes its own state; everything else is handed to the user's shell
  and runs from the snap's files outside confinement (`host-handoff`).
  No interface is ever needed, so the snap installs on Ubuntu Core and
  needs no store review.
- `idiot init` embeds the snap's command, `/snap/bin/<instance>` (the
  confined entry point), or the installed launcher for a source install.
  Never a path inside one revision. (Done: f270e04.)
- idiot's state lives in `~/snap/<instance>/common` (`$SNAP_USER_COMMON`),
  shared by every revision and by the host side. A source install keeps
  `~/.local/share/idiot`; the two don't share data.
- The `core-packages` part is organised into the `core` component, so
  `snap install idiot+core` provides the device tools and the base snap
  doesn't carry them.
- Bundled binaries are patched (`enable-patchelf`) to load core24's
  libraries, so they run host-side without LD_LIBRARY_PATH.
- README describes installing the snap and its component.
- stdout: unchanged. No new runtime dependencies.

## Capabilities

### New Capabilities
- `installation`: installing idiot from source or as a snap, where each keeps its state, and the stable command path the shell integration relies on.

### Modified Capabilities

## Impact

- `snap/snapcraft.yaml` (revert `confinement: classic` from 3e2491e; keep
  its component, version and patchelf fixes), `cmd/init` (f270e04, kept),
  `Makefile` (launcher), `README.md`.
- Depends on `host-handoff`. `native-store-login` adds a Rust part to this
  snapcraft.yaml and depends on both.
