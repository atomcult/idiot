# Design

## Context

See proposal.md for why. Current state:

- One component, `core`, built by the `core-packages` part:
  stage-packages organised into `(component/core)` and patched by
  `snap/local/patchelf-host` (core24's loader, `$ORIGIN`-relative RPATH,
  since a component's mount path changes with each revision). Patched
  binaries only run host-side; confined, AppArmor denies
  `/snap/core24/<rev>`.
- `snap_component_dir` (share/common.sh) knows only `core`: `SNAP_COMPONENTS`
  confined, otherwise `/snap/<inst>/components/<rev of current>/core`.
  `snap_tool_dirs` lists `host/bin` and core's bin dirs; `snap_tool` and
  `snap_app` search them; `snap_app`'s error always says
  `snap install idiot+core`.
- `require` checks `PATH`, then (host-side of the snap) defines a shell
  function of the tool's name that runs the bundled copy. A name with a
  non-identifier character can't be a function, so it falls through to
  `die`: `require qemu-system-x86_64` can never use a bundled qemu.
- `cmd/vm` hard-codes firmware under `/usr/share/OVMF`,
  `/usr/share/qemu-efi-*`, and calls `qemu-system-<arch>` and `swtpm`
  by name.
- core24 itself already ships iproute2, ethtool, iptables, sbsigntool,
  squashfs-tools, cryptsetup, netcat-openbsd, openssl, procps and xxd. On
  Ubuntu Core those are already on `PATH`.

Measured with `apt-cache` on noble (unpacked, minus core24's packages,
counting every alternative dependency, so an overestimate): `core` today
~12 MiB; vm ~120 MiB for x86 alone, plus 540 MiB for arm and 330 MiB for
riscv before pruning (`qemu-efi-aarch64` is 322 MiB of 64 MiB zero-padded
pflash images, which squashfs compresses to almost nothing, and
`qemu-system-misc` holds about twenty emulators of which we need one); tpm
~14 MiB; network, hardware, debug, boot each well under 100 MiB once
alternatives are counted once.

## Goals / Non-Goals

**Goals:**
- Any subset of components can be installed; each is self-contained.
- The tool → component mapping is known even for components that aren't
  installed, so errors and `run --list` can name them.
- No new environment in the user's shell; no interfaces.

**Non-Goals:**
- Building for more architectures than today (the snap has no `platforms`;
  that's a separate change). The vm component's design does cover an arm64
  build (it would still bundle all four emulators).
- tshark/wireshark (about 170 MiB on its own). It can become its own
  component later without changing this design.
- Running bundled tools confined. They stay host-side only.
- Kernel-version-specific tools (perf, bpftrace).

## Decisions

### One part per component, every part patched
Each component gets a part `<name>-packages` with its stage-packages,
`organize: {'*': (component/<name>)/}` and the same
`patchelf-host "$CRAFT_PART_INSTALL" "$CRAFT_ARCH_TRIPLET_BUILD_FOR"`
override-build `core-packages` uses. Shared libraries are duplicated
between components (libcurl in `network` and `tpm`, for instance).
*Alternative:* a shared `libs` component that others depend on. snapd has
no component dependencies, so a user could install `network` without
`libs` and get broken binaries; duplication costs a few MiB.

### Prune at stage time
Every component part drops `usr/share/{doc,man,info,locale,lintian,bug}`
with `stage:` filters. The vm part also keeps only
`qemu-system-{x86_64,aarch64,arm,riscv64}` and `qemu-img` from
`usr/bin`, and from the firmware packages only the files `cmd/vm` can
pick (the `find_first` candidates in `setup_firmware`/
`_setup_firmware_nosb`); the list of firmware paths moves into one place
in `cmd/vm` so the snapcraft filter and the script can't drift (a
`make check` test compares them).
*Alternative:* split `vm` per architecture. More components to explain,
and pruning gets most of the size back anyway.

### Components found by listing, not by name
`snap_component_dirs` replaces `snap_component_dir`: it lists the
directories under the components dir for the revision in use
(`$SNAP_COMPONENTS` confined; host-side `/snap/<inst>/components/<rev>/`
with `<rev>` read from the `current` link, as today) and prints those that
exist, in a fixed order (`core vm tpm network hardware boot debug`, then
any unknown ones). `snap_tool_dirs` prints `host/bin` then each
component's `usr/bin bin usr/sbin sbin`. Because the revision comes from
`current` each time, a refresh switches to the new revision's components
with nothing to reload.

### A build-time index of which component has which tool
Each component part also writes the names of the executables in its
`usr/bin bin usr/sbin sbin` to
`lib/idiot/share/components/<name>` in the *main* snap (organize that one
file to `(default)`, everything else to the component). idiot reads those
files to name the right component when a tool is missing and for
`run --list`, whether or not the component is installed.
*Alternative:* a hand-written table in `share/`. It would go stale the
first time a package adds or renames a binary.
*Fallback if snapcraft won't organise one file of a part into a different
partition than the rest:* a final `component-index` part
(`after:` every component part) that walks the component partitions'
stage dirs.

### Tools by path, not only by function
New `tool_path <name>` prints the host's copy (`command -v`) or the bundled
copy's absolute path, or fails. New `tool <name> [args]` runs it, so a
command can use any name: `tool qemu-system-x86_64 ...`. `require` keeps
its current behaviour for identifier names (existing callers like
`debugfs ...` keep working) and, for other names, only checks
`tool_path`. Its error comes from one helper, `missing_tool <name>`, which
looks the name up in the index: `install it with: snap install idiot+<c>`,
or `not bundled with idiot`, followed by the caller's package hint.
`snap_app` uses the same helper instead of always naming `core`.

### qemu with its own data
The bundled qemu was built for `/usr/share/qemu` and
`/usr/lib/<triplet>/qemu`, which on a bare host don't exist (or belong to a
different qemu version). When `tool_path` resolves qemu into the `vm`
component, `cmd/vm` adds `-L <comp>/usr/share/qemu` and runs qemu with
`QEMU_MODULE_DIR=<comp>/usr/lib/<triplet>/qemu` in qemu's environment only.
qemu-system-data's absolute symlinks (into `/usr/share/seabios`,
`/usr/lib/ipxe/qemu`) are rewritten relative in the vm part's
override-build so they resolve inside the component. A host qemu gets no
`-L`, and only host firmware; a bundled qemu prefers the component's
firmware. Mixing the two is what goes wrong (a ROM from another qemu
version).

### swtpm and tpm2-tools
`swtpm socket` needs no `swtpm_setup` or config file, so `cmd/vm` only
swaps the name for `tool swtpm`. tpm2-tools load their TCTI with
`dlopen("libtss2-tcti-<name>.so.0")`; that is resolved through the
calling library's `DT_RPATH`, which `patchelf-host` sets with
`--force-rpath`, so `--tcti=swtpm:` finds the bundled TCTI. (Verified in
the tasks, not assumed.)

### Packages core24 already has
snapcraft may skip stage-packages already in the base. That's harmless for
libraries, but tools the spec promises (`sbsign` and `sbverify` from
sbsigntool, in core24) must be in the component for classic hosts. If
snapcraft skips them, the boot part fetches them in its override-build
(`apt-get download` + `dpkg-deb -x` into the part's install dir) before
patching. Other base tools (ip, ethtool, iptables, unsquashfs) are not
promised by any component: they're on `PATH` on Core and one apt install
away on classic.

### `idiot run --sudo` and `--list`
`--sudo` resolves the tool with `tool_path` and runs
`sudo -- <path> <args>`. The patched binaries hard-code core24's loader
and their RPATH, so they need nothing from the environment sudo resets.
`--list` writes one line per component to stdout, tab-separated: the
name, `installed` or `available`, and the tools space-separated, so it
pipes into `grep`/`cut`. Under a source install it says on stderr that
components come with the snap and exits zero.

## Risks / Trade-offs

- [Snap and component size: vm is still the largest download] → pruning;
  everything is optional; the base snap doesn't grow apart from the index.
- [A bundled tool that execs a helper by absolute path, e.g.
  `/usr/lib/...` or `/usr/libexec/...` (nmap's scripts, mtr's
  `mtr-packet`, gdb's python)] → test the tools in each component's task,
  on a host without the packages (a fresh LXD container), and fix by
  wrapper or relative symlink where they do.
- [Tools needing root or capabilities (tcpdump, nft, dmidecode, flashrom)
  fail as the user] → `idiot run --sudo`; mtr falls back to `mtr-packet`
  needing caps it won't have: document `--sudo`.
- [Host AppArmor profiles attach by path (swtpm's `/usr/bin/swtpm`)] →
  they don't attach to the bundled path, so the bundled copy runs
  unconfined as the user, like any other user program.
- [Moving tpm2-tools out of `core` breaks `idiot run tpm2_*` for users who
  had only `core`] → the error names `idiot+tpm`; README release note.

## Migration Plan

Users with `idiot+core` who use tpm2-tools: `snap install idiot+tpm`.
Nothing else changes for existing installs. Rollback: revert the snap to
the previous revision (`snap revert idiot`); components revert with it.

## Open Questions

- Whether to add a `capture` component with tshark later (doesn't change
  this design: one more part and index file).
