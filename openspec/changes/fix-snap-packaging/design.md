# Design

## Context

`snap/snapcraft.yaml` runs `make install PREFIX=$CRAFT_PART_INSTALL`, which
puts the tree in `lib/idiot/` and a launcher in `bin/idiot`; the app sets
`IDIOT_ROOT=$SNAP/lib/idiot`. `cmd/init` computes
`IDIOT_BIN="$(cd "$(dirname "$0")/.." && pwd)/idiot"`, i.e. the script
inside the current revision. The dev host has snapcraft 9.2 and snapd 2.77;
an older idiot snap (`x1`, classic, from v1) is installed there.

## Goals / Non-Goals

**Goals:** a snap that works the same as a source install and survives
refreshes; a correctly populated `core` component.

**Non-Goals:** publishing to the store (needs a classic request; separate
step), multi-arch build configuration.

## Decisions

### Classic confinement
idiot runs host binaries (qemu-system-*, lxc, snap, snapcraft, ssh, git),
reads disk images and writes clones anywhere the user asks, and its whole
purpose is to modify the calling shell's environment. Under strict
confinement each of those needs an interface, several of which do not exist
for arbitrary host binaries. Classic is the honest model; v1 already used
it. Alternative considered: strict with `home`, `network`,
`removable-media` plus bundling qemu etc. — large, still unable to call the
host's snapcraft/lxc, and it would split user data into `~/snap/idiot/`.

### `init` resolves its entry point
- If `$SNAP` is set: `/snap/bin/${SNAP_INSTANCE_NAME}` (respects parallel
  installs). `snap run` sets `SNAP`, `IDIOT_ROOT` and the snap's PATH, so the
  bundled fzf is used.
- Else: `IDIOT_LAUNCHER` if the Makefile's launcher exported it, falling back
  to today's computed path (development checkout, where the path is stable).
The Makefile launcher gains `export IDIOT_LAUNCHER="<bindir>/idiot"`.

### Bundled tools on PATH
`command -v fzf` must find `$SNAP/usr/bin/fzf` first. snapd prepends the
snap's bin directories to PATH for strict snaps; for classic snaps this is
verified during implementation, and if absent the app gets
`environment: PATH: $SNAP/usr/bin:$SNAP/bin:$PATH`.

### Component layout
`core-packages` uses `organize: {'*': (component/core)/}` (snapcraft's
component organise syntax) and the component type is `standard`
(`app` is not a component type). Component files are not inside `$SNAP`;
snapd mounts them under `/snap/<snap>/components/`, keyed by the snap
revision. `snap_app` keeps resolving `$SNAP/usr/bin/<tool>` and gains a
lookup in the `core` component's mount for the current revision. The exact
path is read off a local install during implementation (task 2.3) rather
than assumed here.

## Risks / Trade-offs

- [Classic requires store approval] → fine for local/dev installs now; the
  request is a separate, manual step.
- [Component support with classic snaps] → confirm with `snapcraft pack`
  and `snap install --dangerous --classic` on the dev host before relying
  on it; fallback is staging the tools in the main snap as today.
- [Existing v1 snap `x1` installed] → remove it before testing
  (`snap remove idiot`, Lauren's call since it may hold her v1 data in
  `$SNAP_USER_COMMON`).
