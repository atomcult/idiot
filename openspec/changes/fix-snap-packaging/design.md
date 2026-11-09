# Design

## Context

`snap/snapcraft.yaml` runs `make install PREFIX=$CRAFT_PART_INSTALL`, which
puts the tree in `lib/idiot/` and a launcher in `bin/idiot`; the app sets
`IDIOT_ROOT=$SNAP/lib/idiot`. f270e04 made `cmd/init` embed
`/snap/bin/<instance>` under a snap and the Makefile launcher otherwise.
3e2491e switched to classic confinement; that was a misreading of how idiot
runs and is reverted here, keeping the rest of that commit (component type
`standard`, `organize` into the component, `version: git`, patchelf). The
dev host has snapcraft 9.2 and snapd 2.77; an older idiot snap (`x1`, from
v1) is installed there.

## Goals / Non-Goals

**Goals:** a strict, interface-free snap that works the same as a source
install and survives refreshes; a populated `core` component.

**Non-Goals:** publishing to the store; multi-arch build configuration;
migrating data between a source install and the snap automatically.

## Decisions

### Strict, no plugs
Confined, idiot reads and writes only its own state and runs the bundled
fzf; host-side (handed off, `host-handoff`) it is the user, running files
from `/snap/<instance>/current`. With no plugs there is nothing to review,
nothing to connect, and the snap installs on Ubuntu Core. Alternatives
considered: classic (store review, no Ubuntu Core) and strict with `home`,
`network`, `removable-media` etc. (still can't call the host's lxc,
snapcraft or qemu; more surface for no gain).

### Entry point
The wrapper calls `/snap/bin/<instance>`, so management commands run
confined. snapd sets `SNAP`, `SNAP_USER_COMMON` and its PATH (bundled fzf
first) for that process only. What the host side needs in the user's shell
(`IDIOT_SNAP`, `IDIOT_DATA_HOME`, `IDIOT_CACHE_DIR`) comes from `idiot init`
(`host-handoff`); PATH and LD_LIBRARY_PATH are never among it.

### State
`IDIOT_DATA_HOME=$SNAP_USER_COMMON` and
`IDIOT_CACHE_DIR=$SNAP_USER_COMMON/cache` (defaults in `host-handoff`).
`$SNAP_USER_DATA` is per revision and copied on refresh; `common` isn't, so
credentials don't multiply across revisions. Host-side commands get the
same directories from the variables `idiot init` exports. A source-install user moving to the
snap copies `~/.local/share/idiot/.` into `~/snap/idiot/common/` once
(README).

### Bundled binaries on the host
Inside confinement core24 is `/`, so bundled ELF binaries just work. On the
host they would load the host's libc. `enable-patchelf` on every part that
stages binaries sets their interpreter to
`/snap/core24/current/lib64/ld-linux-x86-64.so.2` (per arch) and their
RPATH into the snap and core24, so they run correctly in both places with
no environment at all, and the programs they start (fzf previews run `sh`)
see a clean environment. Task 2.3 checks the interpreter and RPATH with
`readelf` and runs fzf both confined and host-side. If patching isn't
applied for a strict snap, `snap_app` sets LD_LIBRARY_PATH for that one
tool (`host-handoff`).

### Component layout and lookup
`core-packages` uses `organize: {'*': (component/core)/}` and the
component type is `standard`. snapd mounts components outside `$SNAP`,
per snap revision. Confined, `snap_app` finds them under
`/snap/<instance>/components/<revision>/core`; host-side, through
`current`'s revision. The exact path is read off a local install (task
2.3) and then written down here and in `snap_app`.

## Risks / Trade-offs

- [patchelf on strict snaps] → verified in 2.3; fallback is per-tool
  LD_LIBRARY_PATH, never in the user's shell.
- [AppArmor reading `/snap/core24/current` or component mounts while
  confined] → checked in 2.3 by running fzf and `idiot run debugfs -V`
  (host-side) with and without the component.
- [Separate data from a source install] → deliberate (each install owns its
  state); one-line copy documented.
- [Existing v1 snap `x1`] → removing it is Lauren's call; it may hold v1
  data in its `common` directory.
