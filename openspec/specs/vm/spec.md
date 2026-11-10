# vm Specification

## Purpose

Booting Ubuntu Core and other disk images under QEMU from `idiot vm`, with
optional UEFI secure boot and an emulated TPM, for any supported target
architecture.

## Requirements

### Requirement: CPU model for the target architecture
When the target architecture matches the host and `/dev/kvm` exists,
`idiot vm` SHALL use KVM with the host CPU. Otherwise it SHALL emulate the
most capable CPU model QEMU offers for that architecture, so that 64-bit
guests boot under emulation.

#### Scenario: arm64 image on an amd64 host
- **WHEN** the user runs `idiot vm --arch arm64 core-arm64.img` on an amd64 host
- **THEN** QEMU emulates a 64-bit arm64 CPU and the image's firmware starts

#### Scenario: Native with KVM
- **WHEN** the target matches the host and `/dev/kvm` exists
- **THEN** the VM runs with KVM acceleration and the host CPU model

### Requirement: Firmware selection
With secure boot (the default) `idiot vm` SHALL boot the architecture's
secure-boot UEFI firmware with Microsoft keys enrolled; armhf and riscv64,
for which no secure-boot firmware exists, SHALL boot their UEFI firmware
without secure boot and say so. With `--no-sb` it
SHALL boot the architecture's UEFI firmware without secure boot on amd64,
arm64, armhf and riscv64. It SHALL use the host's firmware when the host
qemu is used and the firmware is installed, and the `vm` component's
firmware otherwise. If the needed firmware is in neither, it SHALL fail
naming the package that provides it and the `vm` component.

#### Scenario: No secure boot on arm64
- **WHEN** the user runs `idiot vm --arch arm64 --no-sb image.img`
- **THEN** the VM boots non-secure-boot UEFI firmware

#### Scenario: Firmware missing
- **WHEN** the arm64 firmware package is not installed and `idiot+vm` is not installed
- **THEN** the command fails naming `qemu-efi-aarch64` and `snap install idiot+vm`

#### Scenario: Secure boot unavailable
- **WHEN** the user runs `idiot vm --arch armhf image.img`
- **THEN** it says no secure boot firmware exists for armhf and boots armhf's UEFI firmware

#### Scenario: Firmware from the component
- **WHEN** the host has no OVMF and `idiot+vm` is installed
- **THEN** `idiot vm image.img` boots with secure boot using the component's firmware

### Requirement: Emulated TPM lifetime
With `--tpm`, the emulated TPM SHALL run only for the lifetime of the VM: it
SHALL be stopped when QEMU exits, however QEMU exits.

#### Scenario: VM shut down
- **WHEN** a VM started with `--tpm` exits
- **THEN** no swtpm process for that VM remains

### Requirement: Emulator and TPM from the snap
`idiot vm` SHALL run the host's `qemu-system-<arch>` when installed and the
`vm` component's otherwise, with that qemu's own ROMs and modules (the
component's qemu SHALL NOT use the host's, and the reverse). `--tpm` and
`idiot vm tpm` SHALL use the host's swtpm, or the `tpm` component's
when the host has none. A missing emulator SHALL fail naming
`snap install idiot+vm`, and a missing swtpm `snap install idiot+tpm`.

#### Scenario: VM on Ubuntu Core
- **WHEN** an Ubuntu Core device has `idiot+vm` and `idiot+tpm` installed and the user runs `idiot vm --tpm core.img`
- **THEN** the image boots with secure boot and an emulated TPM, with no host packages installed

#### Scenario: Foreign architecture from the component
- **WHEN** the host has no qemu, `idiot+vm` is installed, and the user runs `idiot vm --arch riscv64 --no-sb image.img` on an amd64 host
- **THEN** the component's `qemu-system-riscv64` boots the image with the component's RISC-V firmware

#### Scenario: No emulator
- **WHEN** the host has no qemu and `idiot+vm` is not installed
- **THEN** `idiot vm image.img` fails naming `snap install idiot+vm`

#### Scenario: TPM without swtpm
- **WHEN** the host has no swtpm, `idiot+tpm` is not installed, and the user runs `idiot vm --tpm image.img`
- **THEN** the command fails naming `snap install idiot+tpm` before starting qemu

### Requirement: Saved UEFI variables
`idiot vm vars <state-dir>` SHALL list the UEFI variables stored in the
state directory's `fw_vars.fd` (an OVMF or AAVMF variable store), and
SHALL NOT read the host's own variables. It SHALL print each live variable
with its vendor GUID and size, and for PK, KEK, db and dbx each enrolled
X.509 certificate by subject and the number of entries of each hash type.
The listing SHALL go to stdout and SHALL need no tool beyond python3. A
missing state directory, a state directory without `fw_vars.fd`, or a file
that isn't a variable store SHALL fail with an error on stderr.

#### Scenario: Secure boot keys of a VM
- **WHEN** a VM was booted with `idiot vm --state-dir ./state image.img` (secure boot) and the user runs `idiot vm vars ./state`
- **THEN** stdout lists the VM's variables, with PK, KEK and db showing the enrolled certificates' subjects (for Ubuntu's OVMF: the Ubuntu OVMF key and Microsoft's KEK and UEFI CAs)

#### Scenario: Not the host's keys
- **WHEN** the host has its own secure-boot keys enrolled and the user runs `idiot vm vars ./state`
- **THEN** none of the host's certificates appear; only the VM's store is read

#### Scenario: arm64 VM
- **WHEN** an arm64 VM was booted with `--state-dir ./state-arm` and the user runs `idiot vm vars ./state-arm`
- **THEN** its AAVMF variables are listed the same way

#### Scenario: Pipeable
- **WHEN** the user runs `idiot vm vars ./state | grep KEK`
- **THEN** the KEK line is printed by grep

#### Scenario: No saved variables
- **WHEN** the state directory has no `fw_vars.fd`
- **THEN** the command fails on stderr, saying the directory has no `fw_vars.fd`

#### Scenario: Without virt-fw-vars or efitools
- **WHEN** neither virt-fw-vars nor efi-readvar is installed
- **THEN** `idiot vm vars ./state` still lists the variables
