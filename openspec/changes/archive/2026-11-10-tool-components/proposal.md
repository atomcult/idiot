# Proposal

## Why

The snap bundles almost nothing a host command needs beyond the `core`
component's image-inspection tools. `idiot vm` only works where the host has
qemu, the UEFI firmware packages and swtpm installed, which an Ubuntu Core
device never does. Ubuntu Core also lacks the everyday tools for looking at a
device's network, hardware, boot chain and processes, and has no apt to get
them. Since every host command already runs the snap's patched binaries
host-side, more of those tools can be shipped as optional components, each
installed only where it's wanted.

## What Changes

- New optional components, each a group of tools for one job, installed with
  `snap install idiot+<name>`:
  - `vm`: qemu for amd64, arm64, armhf and riscv64 targets, `qemu-img`, and
    the UEFI firmware `idiot vm` boots (OVMF, AAVMF, ARM and RISC-V EFI);
  - `tpm`: tpm2-tools, swtpm and swtpm-tools;
  - `network`: tcpdump, nmap, iw, dig/host/nslookup, traceroute, mtr,
    iperf3, socat, curl, nft, conntrack, arping, tracepath, whois;
  - `hardware`: lspci, lsusb, i2c-tools, dmidecode, lshw, smartctl,
    nvme-cli, the gpiod tools, can-utils, picocom, mmc-utils, flashrom;
  - `boot`: efibootmgr, mokutil, efitools, keyutils, u-boot-tools
    (`mkimage`, `fw_printenv`), the device-tree compiler (`dtc`,
    `fdtdump`), sbsigntool;
  - `debug`: strace, ltrace, gdb, lsof, file, binutils, elfutils, jq,
    psmisc, sysstat, htop, hexdump, patchelf.
- `core` stays the image-inspection component and gains gdisk, parted,
  erofs-utils, zstd, lz4, cpio and simg2img. **BREAKING**: tpm2-tools moves
  from `core` to `tpm`; `idiot+core` users who use tpm2 tools must install
  `idiot+tpm`.
- Bundled tools are found in every installed component. A command that needs
  a tool the host lacks and no installed component has fails naming the
  component that provides it (`install it with: snap install idiot+vm`), not
  always `core`.
- `idiot vm` uses the bundled qemu, firmware and swtpm when the host has
  none, so it works on a bare host or an Ubuntu Core device with `idiot+vm`
  (and `idiot+tpm` for `--tpm` and `vm tpm-inspect`). The host's own qemu
  and firmware still win when installed.
- `idiot run` gains `--sudo` (run a bundled tool as root: `sudo` resets
  `PATH`, so `sudo tcpdump` in `run --shell` can't find it) and `--list`
  (each component, whether it's installed, and its tools).
- Fixes `require` for bundled tools whose names aren't shell identifiers
  (`qemu-system-x86_64`, `qemu-img`): today a bundled tool with a `-` in
  its name can't be required at all, so `idiot vm` couldn't use a bundled
  qemu even if one were shipped.

stdout: `idiot run --list` writes its listing to stdout (data, pipeable); a
tool run by `idiot run` writes its own stdout as before. Nothing new is queued
in the eval file; `PATH` and `LD_LIBRARY_PATH` stay untouched in the user's
shell.

## Capabilities

### New Capabilities
- `bundled-tools`: the snap's optional tool components, what each holds, how
  host commands and `idiot run` find bundled tools, and what happens when a
  tool is missing.

### Modified Capabilities
- `installation`: the Snap install requirement names the set of components
  instead of only `core`, and the missing-tool scenario names the right one.
- `vm`: firmware, qemu and swtpm come from the host or the `vm`/`tpm`
  components; a missing one names both the package and the component.

## Impact

- `snap/snapcraft.yaml`: six new components and their parts, each patched
  with `snap/local/patchelf-host`; the vm part prunes qemu and firmware to
  what `idiot vm` uses. Runtime dependencies: all staged into the
  components (none on the host). Snap and component sizes grow; `vm` is the
  largest (estimated well over 100 MiB unpacked before pruning).
- `share/common.sh`: component discovery replaces the `core`-only
  `snap_component_dir`; `require`/`snap_tool`/`snap_app` search all
  components and name the right one when a tool is missing.
- `cmd/vm`: firmware search paths include the component, qemu gets the
  component's data and module directories, swtpm from `tpm`.
- `cmd/run`: `--sudo`, `--list`.
- README, CLAUDE.md: components and what each is for.
