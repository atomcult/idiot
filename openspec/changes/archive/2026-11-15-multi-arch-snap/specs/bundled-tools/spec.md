# Spec Delta

## MODIFIED Requirements

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
Each component SHALL provide its listed tools on every architecture the
snap is built for, except `ltrace`, which Ubuntu does not build for
riscv64. Every bundled tool SHALL run on the host with no host packages
installed, including on Ubuntu Core.

#### Scenario: Component on a bare host
- **WHEN** the host has no tcpdump and the user installs `idiot+network` and runs `idiot run tcpdump --version`
- **THEN** the bundled tcpdump prints its version

#### Scenario: Only what's installed
- **WHEN** only `idiot+hardware` is installed and the user runs `idiot run lspci`
- **THEN** it runs, and `idiot run tcpdump` fails naming `idiot+network`

#### Scenario: ltrace on riscv64
- **WHEN** `idiot+debug` is installed on a riscv64 host without ltrace and the user runs `idiot run ltrace`
- **THEN** it fails saying ltrace is not on PATH and not bundled with idiot, and every other debug tool runs
