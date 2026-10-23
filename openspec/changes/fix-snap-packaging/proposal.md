# Proposal

## Why

The snap doesn't behave like the `make install` build:

- `idiot init` writes the absolute path of the script inside the snap,
  `/snap/idiot/<revision>/lib/idiot/idiot`, into the user's shell. That path
  is tied to one revision, so it breaks after refreshes. It also runs the
  script outside the snap's environment, so the fzf the snap bundles is never
  used.
- The snap declares strict confinement with no plugs. A strictly confined
  idiot couldn't read `~/.local/share`, reach the network, or run the host
  tools it drives (qemu, lxc, snap, snapcraft, git).
- The `core` component is declared but no part puts files in it.

## What Changes

- The snap uses **classic** confinement, matching what idiot is: a developer
  tool that drives host tools and writes wherever the user points it.
- `idiot init` emits the command's stable entry point: `/snap/bin/idiot`
  inside the snap, and the installed launcher otherwise. Never a path inside
  one revision.
- The `core-packages` part (e2fsprogs, mtools, python3-minimal, util-linux,
  dosfstools, tpm2-tools) is organised into the `core` component, so
  `snap install idiot+core` provides those tools and the base snap doesn't
  carry them.
- The snap's bundled fzf is found before the host's.
- The snap's README/install docs describe `snap install --classic`.
- stdout is not affected. No new runtime dependencies; the snap stages what
  it staged before, split between the snap and its component.

## Capabilities

### New Capabilities
- `installation`: installing idiot from source or as a snap, and the stable command path the shell integration relies on.

### Modified Capabilities

## Impact

- `snap/snapcraft.yaml`, `cmd/init`, `share/common.sh` (`snap_app` error
  text), `Makefile` (install launcher), `README.md`.
- A classic snap needs a store classic-confinement request before it can be
  published to the Snap Store; local `--dangerous --classic` installs work
  without one.
- `native-store-login` adds a Rust part to this snapcraft.yaml and depends on
  this change.
