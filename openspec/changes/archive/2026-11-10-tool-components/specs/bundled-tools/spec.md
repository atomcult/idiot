# Spec Delta

## Purpose

The tools the idiot snap can bundle for host commands, grouped into optional
components so a host or Ubuntu Core device installs only the groups it
needs, and how idiot finds and runs them without changing the user's
environment.

## ADDED Requirements

### Requirement: Tool components
The snap SHALL offer these optional components, each installable on its own
with `snap install idiot+<name>`, and each SHALL provide at least the tools
listed:
- `core` (image inspection): `debugfs`, `mke2fs`, `mcopy`, `mdir`, `sfdisk`,
  `lsblk`, `mkfs.vfat`, `fsck.vfat`, `sgdisk`, `parted`, `dump.erofs`,
  `fsck.erofs`, `zstd`, `lz4`, `cpio`, `simg2img`;
- `vm` (virtual machines): `qemu-system-x86_64`, `qemu-system-aarch64`,
  `qemu-system-arm`, `qemu-system-riscv64`, `qemu-img`, and the UEFI
  firmware `idiot vm` boots for each of those architectures;
- `tpm` (TPM): `swtpm`, `swtpm_setup`, `swtpm_ioctl`, and the `tpm2_*`
  commands from tpm2-tools;
- `network`: `tcpdump`, `nmap`, `iw`, `dig`, `host`, `nslookup`,
  `traceroute`, `mtr`, `iperf3`, `socat`, `curl`, `nft`, `conntrack`,
  `arping`, `tracepath`, `whois`;
- `hardware`: `lspci`, `lsusb`, `i2cdetect`, `i2cget`, `i2cset`,
  `i2cdump`, `dmidecode`, `lshw`, `smartctl`, `nvme`, `gpiodetect`,
  `gpioinfo`, `gpioget`, `gpioset`, `candump`, `cansend`, `picocom`, `mmc`,
  `flashrom`;
- `boot`: `efibootmgr`, `mokutil`, `efi-readvar`, `sign-efi-sig-list`,
  `keyctl`, `mkimage`, `dumpimage`, `fw_printenv`, `fw_setenv`, `dtc`,
  `fdtdump`, `sbsign`, `sbverify`;
- `debug`: `strace`, `ltrace`, `gdb`, `lsof`, `file`, `readelf`, `objdump`,
  `nm`, `strings`, `eu-readelf`, `eu-stack`, `jq`, `pstree`, `fuser`,
  `iostat`, `mpstat`, `pidstat`, `htop`, `hexdump`, `patchelf`.
Every bundled tool SHALL run on the host with no host packages installed,
including on Ubuntu Core.

#### Scenario: Component on a bare host
- **WHEN** the host has no tcpdump and the user installs `idiot+network` and runs `idiot run tcpdump --version`
- **THEN** the bundled tcpdump prints its version

#### Scenario: Only what's installed
- **WHEN** only `idiot+hardware` is installed and the user runs `idiot run lspci`
- **THEN** it runs, and `idiot run tcpdump` fails naming `idiot+network`

### Requirement: Finding a tool
When a host command needs a tool, idiot SHALL use the host's copy if it is
on `PATH`, and otherwise the copy in any installed component.
If neither exists it SHALL fail naming the component that provides the tool
(`install it with: snap install idiot+<component>`), or, for a tool no
component provides, saying it is not bundled. Tool names that are not shell
identifiers (`qemu-system-x86_64`, `qemu-img`) SHALL be found like any
other. The user's `PATH` and `LD_LIBRARY_PATH` SHALL NOT change; whatever a
bundled tool needs is set for its own process only. A source install SHALL
use only the host's tools.

#### Scenario: Host copy preferred
- **WHEN** the host has qemu installed and `idiot+vm` is installed
- **THEN** `idiot vm` runs the host's qemu

#### Scenario: Missing tool names its component
- **WHEN** no component providing `smartctl` is installed, the host has no smartctl, and the user runs `idiot run smartctl -a /dev/sda`
- **THEN** the command fails with an error naming `snap install idiot+hardware`

#### Scenario: Tool nobody bundles
- **WHEN** the user runs `idiot run no-such-tool`
- **THEN** the command fails saying `no-such-tool` is not on PATH and not bundled with idiot

#### Scenario: Shell unchanged
- **WHEN** the user runs any bundled tool through idiot
- **THEN** `PATH` and `LD_LIBRARY_PATH` in the user's shell are the same afterwards

### Requirement: Running bundled tools
`idiot run <tool> [args...]` SHALL run the bundled copy of a tool (the
snap's or an installed component's), or the host's when none is bundled,
with the given arguments, passing its stdin, stdout, stderr and exit status
through unchanged; a missing tool fails as in "Finding a tool". `idiot run --sudo <tool> [args...]` SHALL run the same
tool as root through `sudo`. `idiot run --shell` SHALL open a shell whose
`PATH` starts with the bundled tool directories of every installed
component. `idiot run --list` SHALL print to stdout every component, whether
it is installed, and the tools it provides, and SHALL work without any
component installed.

#### Scenario: Exit status passed through
- **WHEN** the user runs `idiot run dig nonexistent.invalid +short; echo $?`
- **THEN** the printed status is dig's own

#### Scenario: As root
- **WHEN** `idiot+network` is installed and the user runs `idiot run --sudo tcpdump -i any -c 1`
- **THEN** sudo asks for the user's password if it needs one and the bundled tcpdump captures one packet as root

#### Scenario: Shell with every component
- **WHEN** `idiot+vm` and `idiot+debug` are installed and the user runs `idiot run --shell`
- **THEN** `qemu-img` and `strace` both resolve to the bundled copies in that shell, and the parent shell's `PATH` is unchanged after it exits

#### Scenario: Listing components
- **WHEN** only `idiot+core` is installed and the user runs `idiot run --list`
- **THEN** stdout lists all seven components with `core` marked installed and the others not, each with its tools

#### Scenario: Source install
- **WHEN** idiot is installed from source and the user runs `idiot run --list`
- **THEN** it says the components are only available with the snap and exits zero
