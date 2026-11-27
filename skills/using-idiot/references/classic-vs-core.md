# idiot on Ubuntu Classic and Ubuntu Core

## Contents

- [Installing](#installing)
- [Shell integration](#shell-integration)
- [Where state lives](#where-state-lives)
- [Tools and components](#tools-and-components)
- [What differs on Ubuntu Core](#what-differs-on-ubuntu-core)
- [Confined and host commands](#confined-and-host-commands)

## Installing

| | Ubuntu Classic | Ubuntu Core |
|---|---|---|
| From source | `make install` (to `~/.local`; needs make and cargo) | not possible: no make, cargo or git |
| Snap | `sudo snap install idiot` | `sudo snap install idiot` |
| Components | `sudo snap install idiot+core idiot+vm ...` | the same; the only way to get most tools |
| Local build | `sudo snap install --dangerous idiot_*.snap idiot+*.comp` | the same, after copying the files over |

Add `--channel` (for example `--edge`) when the snap isn't on stable yet.
Components follow the snap's channel and revision. Installing a local build
with `--dangerous` replaces the store snap: confirm with the user first.

A host can have both a source install and the snap; the shell integration
that was loaded decides which one runs. `idiot doctor` warns about it.

## Shell integration

Add one line to the shell's rc file, once:

```bash
eval "$(idiot init bash)"   # ~/.bashrc
eval "$(idiot init zsh)"    # ~/.zshrc
idiot init fish | source    # ~/.config/fish/config.fish
```

Ubuntu Core ships bash only; zsh and fish aren't available there.

After upgrading from a version that eval'd stdout, already-open shells need
the line run again once; idiot says so. After a snap refresh nothing needs
reloading.

## Where state lives

| | Source install | Snap (Classic and Core) |
|---|---|---|
| Data (credentials, stores, models, profiles, VMs) | `~/.local/share/idiot` | `~/snap/idiot/common` |
| Cache | `~/.cache/idiot` | `~/snap/idiot/common/cache` |
| VMs | `~/.local/share/idiot/vm/<name>` | `~/snap/idiot/common/vm/<name>` |

The two installs don't share state. To carry it over to the snap, once:
`cp -a ~/.local/share/idiot/. ~/snap/idiot/common/`.

## Tools and components

The snap itself bundles only fzf. Other tools come from optional
components, and a host's own copy of a tool always wins:

| Component | For | Main tools |
|---|---|---|
| `core` | disk images (`inspect image`, `inspect diff`) | debugfs, mke2fs, mtools, sfdisk, sgdisk, parted, erofs, zstd, lz4, cpio, simg2img |
| `vm` | `idiot vm` | qemu-system for amd64/arm64/armhf/riscv64, qemu-img, UEFI firmware |
| `tpm` | `idiot vm --tpm`, `idiot vm tpm` | swtpm, swtpm_setup, tpm2-tools |
| `network` | networks | tcpdump, nmap, iw, dig, traceroute, mtr, iperf3, socat, curl, nft |
| `hardware` | devices | lspci, lsusb, i2c-tools, dmidecode, lshw, smartctl, nvme, gpiod, can-utils, picocom |
| `boot` | boot chain | efibootmgr, mokutil, efitools, keyctl, mkimage, fw_printenv, dtc, sbsign |
| `debug` | processes and binaries | strace, ltrace, gdb, lsof, file, binutils, elfutils, jq, htop, patchelf |

- `idiot run <tool>` runs a bundled tool (else the host's);
  `idiot run --sudo <tool>` runs it as root, since plain `sudo` resets PATH
  and can't find it; `idiot run --shell` opens a shell with every bundled
  tool first on PATH.
- `idiot run --list` prints each component, whether it's installed, and
  its tools, tab-separated.
- A command missing a tool names the component: install it and retry.
- The bundled qemu has no display window: VMs use the serial console
  (`--no-gfx`, the default).
- `swtpm_setup` can't create EK or platform certificates from the bundle;
  the host's swtpm-tools package can (Classic only).

## What differs on Ubuntu Core

- **No source install, no git.** `idiot kernel`, `idiot example` and
  `idiot model update` clone git repositories and fail on Core. Run them on
  a Classic host and copy the results over, or point idiot at copies.
- **bash only.** Use `idiot init bash`.
- **Components are the tool source.** There is no apt. Core 24 itself ships
  ip, ethtool, iptables, unsquashfs, cryptsetup, nc, openssl, ssh, scp and
  sudo, so those are already on PATH; everything else comes from a
  component.
- **`idiot clean containers`** needs `lxc`, which Core has only with the
  lxd snap installed.
- **VMs are usually emulated.** Without `/dev/kvm` (most Core devices, and
  any guest arch other than the host's), `idiot vm` runs QEMU with TCG:
  boot times are minutes, not seconds. `idiot doctor` reports it. Use
  `idiot vm stop --timeout <seconds>` with a generous timeout.
- **sudo** works for a system-user or console-conf account; `idiot vm
  install` also needs passwordless sudo inside the guest.
- **No python3 is needed** on either platform.

On Classic, KVM needs the user in the `kvm` group
(`sudo usermod -aG kvm $USER`, then a new login); `idiot doctor` says when
`/dev/kvm` isn't accessible.

The host's swtpm (`/usr/bin/swtpm`, Classic) runs under an AppArmor profile
that lets it create sockets only under `~` and `/tmp`. A VM whose state
directory is elsewhere (for example under `/run/user`) fails with
`--tpm`; idiot's error says so. Use a `--state-dir` under `~`.

## Confined and host commands

The snap is strictly confined with no interfaces to connect. Commands that
only change idiot's own state (`auth use`, `store use`, `arch`, `lp`,
`profile`, `status`, `prompt`) run confined. Commands that need host tools,
the network or the user's files (`vm`, `run`, `inspect`, `auth add`,
`auth export`, `doctor`, `clean`, `kernel`, `example`, `model update`) are
handed back to the user's shell by the wrapper and run there, as the user,
from the snap's files. This is why the shell integration must be loaded:
without it, host commands can't run from the snap at all, and `idiot
doctor` runs a reduced, confined set of checks and exits 1.

Host commands can't change the environment, so commands that do both are
split (for example `auth add` saves a credential; `auth use` activates
it). idiot never changes PATH or LD_LIBRARY_PATH in the user's shell.
