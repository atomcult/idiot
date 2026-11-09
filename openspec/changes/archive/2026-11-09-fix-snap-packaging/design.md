# Design

## Context

`snap/snapcraft.yaml` runs `make install PREFIX=$CRAFT_PART_INSTALL`, which
puts the tree in `lib/idiot/` and a launcher in `bin/idiot`; the app sets
`IDIOT_ROOT=$SNAP/lib/idiot`. f270e04 made `cmd/init` embed
`/snap/bin/<instance>` under a snap and the Makefile launcher otherwise.
3e2491e switched to classic confinement; that was a misreading of how idiot
runs and is reverted here, keeping the rest of that commit (component type
`standard`, `organize` into the component, `version: git`, patchelf). The
dev host has snapcraft 9.2 and snapd 2.78; an older idiot snap (`x1`, from
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
Inside confinement core24 is `/`, so bundled ELF binaries just work as
packaged. On the host they would load the host's libc (or miss libraries
the host lacks). `snap/local/patchelf-host` sets each executable's
interpreter to core24's loader (`/snap/core24/current/lib64/...`, per arch)
and gives every ELF file an RPATH into its own tree (relative to `$ORIGIN`)
and then core24, so it runs host-side with no environment at all, and the
programs it starts (fzf previews run `sh`) see a clean environment.

Found on the real install (2.3):
- A patched binary can't run confined: AppArmor denies a confined process
  reading or executing `/snap/core24/<rev>/...`, and once the RPATH lookup
  is denied the loader doesn't fall back to `/lib`. So fzf, the one tool
  used both confined (pickers) and host-side, ships unpatched in `usr/bin`
  with a patched copy in `host/bin`; `snap_tool_dirs` returns `$SNAP/usr/bin`
  confined and `$IDIOT_SNAP/host/bin` plus the component host-side.
- snapcraft's `enable-patchelf` skips files organised into a component, so
  both parts call `patchelf-host` from `override-build` instead.
- `python3-minimal` staged only a dangling `python3 -> python3.12` symlink
  (snapcraft drops files core24 already has); dropped from the component.
  Host commands use the host's python3 (Ubuntu Core 24's root is core24).
- Without patching, nothing would set LD_LIBRARY_PATH in the user's shell
  either way; the per-tool fallback was never needed.

### Component layout and lookup
`core-packages` uses `organize: {'*': (component/core)/}` and the
component type is `standard`. snapd mounts the component at
`/snap/<instance>/components/mnt/core/<component revision>` and links
`/snap/<instance>/components/<snap revision>/core` to it; confined it sets
`SNAP_COMPONENTS=/snap/<instance>/components/<snap revision>`.
`snap_component_dir` uses `SNAP_COMPONENTS` when set, else the snap
revision (`SNAP_REVISION` confined, `current`'s target host-side). The
`$ORIGIN`-relative RPATH keeps working whichever revision path is used.

## Risks / Trade-offs

- [patchelf on strict snaps] → patched binaries can't run confined
  (AppArmor); confined tools stay unpatched, host-side copies are patched
  (2.3).
- [AppArmor reading `/snap/core24/current` or component mounts while
  confined] → checked in 2.3 by running fzf and `idiot run debugfs -V`
  (host-side) with and without the component.
- [Separate data from a source install] → deliberate (each install owns its
  state); one-line copy documented.
- [Existing v1 snap `x1`] → removing it is Lauren's call; it may hold v1
  data in its `common` directory.
