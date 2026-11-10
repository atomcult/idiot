# Design

## Context

See proposal.md. Nothing in the build is tied to amd64 except the missing
`platforms`: `patchelf-host` prefixes each binary's own interpreter path
with core24's (`/lib/ld-linux-aarch64.so.1`, `ld-linux-armhf.so.3`,
`ld-linux-riscv64-lp64d.so.1` all exist in core24 for their arch),
`component-build` and `patchelf-host` take the triplet from
`CRAFT_ARCH_TRIPLET_BUILD_FOR`, the firmware packages are `Architecture:
all`, qemu-system-x86/arm/misc are built for every architecture, and rustup
ships 1.91.1 for armv7 and riscv64gc hosts.

## Goals / Non-Goals

**Goals:** one snapcraft.yaml that packs on each of the four architectures.

**Non-Goals:** cross-building (build-on amd64 for arm64); running builds,
registering or uploading (Lauren's); CI configuration.

## Decisions

### Native platforms only
`platforms: {amd64, arm64, armhf, riscv64}` in the short form (build-on =
build-for). *Alternative:* `build-on: [amd64]` with foreign `build-for`.
The rust part would need a cross linker and target, and stage-packages for
a foreign architecture add a second apt architecture to the build
instance; every build service Lauren might use builds natively anyway.

### Per-architecture packages through snapcraft's grammar
The debug part lists ltrace under `- to amd64`, `- to arm64` and
`- to armhf` entries instead of a plain item. *Alternative:* `- to riscv64:
[]` plus `- else: [ltrace]`; the explicit form reads plainly and is what
the grammar documents. Checked by resolving the part's stage-packages with
snapcraft's own grammar processor for each target.

### check-components follows the snap's architecture
It reads `architectures` from the snap's `meta/snap.yaml` and drops
`ltrace` from the debug expectations for riscv64, so the same check runs on
every build.

## Risks / Trade-offs

- [The non-amd64 builds have never run: a package might ship different
  binaries there (dmidecode ships only `dmidecode` off x86), or patchelf
  0.18 might break other binaries on other ISAs] → check-components on
  each build catches a missing promised tool; the gdb-style loader
  fallback in `patchelf-host` covers broken program headers on any arch;
  the bare-host test (tasks) is the real check.
- [riscv64 and armhf builds are slow on Launchpad (qemu-system-misc, gdb)]
  → acceptable; they only rebuild when Lauren starts them.
