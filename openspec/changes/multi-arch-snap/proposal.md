# Proposal

## Why

The snap only builds for amd64 (snapcraft.yaml has no `platforms`), so
neither idiot nor its tool components can be installed on the arm64, armhf
and riscv64 boards Ubuntu Core runs on, which are the devices the
components were made for.

## What Changes

- Build the snap and all seven components for amd64, arm64, armhf and
  riscv64, natively (each built on its own architecture), so the same
  snapcraft.yaml works with `snapcraft remote-build`, a Launchpad recipe or
  native CI runners. Lauren runs the builds, registers the name and
  uploads (edge, `grade: devel`); none of that is part of this change.
- `ltrace` is not built for riscv64 in Ubuntu noble, so the riscv64 debug
  component has no `ltrace`. Every other stage-package exists on all four
  architectures (checked against noble's and noble-updates' package
  indexes).
- `snap/local/check-components` reads the built snap's architecture and
  expects the tools that architecture can have.
- README lists the architectures.

stdout: nothing changes in what any command writes or queues in the eval
file. Runtime dependencies: none new; each architecture's components stage
the same packages for that architecture.

## Capabilities

### New Capabilities

### Modified Capabilities
- `installation`: the snap is built for amd64, arm64, armhf and riscv64.
- `bundled-tools`: the tool lists hold on every architecture, except
  `ltrace` on riscv64.

## Impact

`snap/snapcraft.yaml` (`platforms`, the debug part's stage-packages),
`snap/local/check-components`, README. Untested here: the arm64, armhf and
riscv64 builds themselves (Lauren runs those); what can be checked on amd64
is that the grammar resolves to the right package list per architecture.
