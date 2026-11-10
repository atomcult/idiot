# Tasks

## 1. Find tools in every component

- [x] 1.1 `share/common.sh`: replace `snap_component_dir` with `snap_component_dirs` (every component dir of the revision in use, fixed order, then unknown ones) and make `snap_tool_dirs` list each one's `usr/bin bin usr/sbin sbin` after `host/bin`; verify with a fake tree (`SNAP_COMPONENTS` pointing at a scratch dir holding `core/` and `vm/`) that `snap_tool` finds tools in both and that a missing component dir is skipped
- [x] 1.2 Add `tool_path`, `tool` and `missing_tool` (reads `$IDIOT_ROOT/share/components/<name>` index files); make `require` accept non-identifier names and `snap_app` use `missing_tool`; verify with fake index files that a missing tool names its component, an unindexed one says "not bundled with idiot", `require qemu-system-x86_64` succeeds when only the bundled copy exists, and a source install (no `IDIOT_SNAP`) uses `PATH` only
- [x] 1.3 Update CLAUDE.md's helper list (`tool`, `tool_path`, `require` on any name); run `make check` and commit (`refactor: find bundled tools in every component`)

## 2. Components in the snap

- [x] 2.1 snapcraft.yaml: each component part writes its index file into the main snap (organise that file to `(default)`, or the `component-index` fallback part from design.md if snapcraft refuses), plus the shared `stage:` prune of docs, man pages and locales; verify `snapcraft pack` and that `unsquashfs -l` of the snap lists `lib/idiot/share/components/core`
- [x] 2.2 `core`: add gdisk, parted, erofs-utils, zstd, lz4, cpio, android-sdk-libsparse-utils; move tpm2-tools to a new `tpm` part with swtpm and swtpm-tools; verify both index files list the spec's tools
- [x] 2.3 Add the `network`, `hardware`, `boot` and `debug` parts and components with the spec's packages; if snapcraft skips sbsigntool because core24 has it, fetch it in the boot part's override-build (design.md); verify each index file contains every tool the spec lists for it (a small check script under `snap/local/`, run by hand after pack)
- [x] 2.4 Test on a bare host: in a fresh LXD `ubuntu:24.04` container with none of the packages installed, `snap install --dangerous` the snap and all components, run every tool the spec lists with `--version`/`--help` (or the closest no-op) through `idiot run`, and fix any tool that execs a helper by absolute path (nmap scripts, `mtr-packet`, gdb's python); also check `tpm2_startup -c --tcti=swtpm:` against a running bundled swtpm (TCTI dlopen through the RPATH)
- [x] 2.5 In the same container: install only `core`, check `idiot run tpm2_pcrread` names `idiot+tpm` and `idiot run lspci` names `idiot+hardware`; refresh to a rebuilt revision and check a component tool still runs in a shell that loaded the integration before the refresh
- [x] 2.6 README: the components, what each is for, sizes, the tpm2-tools move; run `make check` and commit (`feat(snap)!: tool components`)

## 3. `idiot vm` from the components

- [x] 3.1 `cmd/vm`: collect firmware candidate paths in one list, search the `vm` component's copy of each path when using the bundled qemu, and the host's when using the host's; name both the package and `snap install idiot+vm` when nothing is found; prune the vm part to those files and `qemu-system-{x86_64,aarch64,arm,riscv64}` and `qemu-img`; add a `make check` test that every firmware path in `cmd/vm` is kept by the vm part's filter
- [x] 3.2 Add the `vm` part (qemu-system-x86, qemu-system-arm, qemu-system-misc, qemu-utils, ovmf, qemu-efi-aarch64, qemu-efi-arm, qemu-efi-riscv64), rewriting qemu-system-data's absolute symlinks to relative ones; run the bundled qemu with `-L <comp>/usr/share/qemu` and `QEMU_MODULE_DIR` in its own environment only; `--tpm` and `tpm-inspect` use `tool swtpm` and name `idiot+tpm` when missing; verify `snapcraft pack` and the vm component's unpacked size
- [x] 3.3 Test in the bare LXD container (vm and tpm components, no qemu, ovmf or swtpm packages): `idiot vm --tpm` boots an Ubuntu Core 24 amd64 image to its login prompt with secure boot; `--arch arm64` and `--arch riscv64 --no-sb` reach their firmware; without `idiot+tpm` `--tpm` fails before qemu starts; on the host (with qemu installed) `idiot vm` still runs the host's qemu and firmware
- [x] 3.4 Update README's vm section (works without host qemu with `idiot+vm`/`idiot+tpm`); run `make check` and commit (`feat(vm): run from the vm and tpm components`)

## 4. `idiot run --sudo` and `--list`

- [x] 4.1 `cmd/run`: `--sudo <tool> [args]` (sudo of the resolved path) and `--list` (tab-separated name, `installed`/`available`, tools, on stdout; source install: message on stderr, exit 0); `--shell` covers every installed component; update its help; verify in the container: `idiot run --sudo tcpdump -i any -c 1`, `idiot run --list | cut -f1,2` with only some components installed, `idiot run dig nonexistent.invalid; echo $?` passes dig's status through, and `--shell` leaves the parent's `PATH` unchanged
- [x] 4.2 README and CLAUDE.md: `run --sudo`/`--list`; run `make check` and commit (`feat(run): --sudo and --list`)

## 5. Integration

- [x] 5.1 Walk every scenario in the change's bundled-tools, installation and vm specs once more on the final build (bare container, plus Lauren's host for "Host copy preferred"), and run `make check`
