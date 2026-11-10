# 🌱 idiot

**Interactive Devtools for IoT** — a shell environment manager and introspection
tool for Snap and Core development. Switch credentials, stores, and architectures
with a single command; everything takes effect in your current shell session.

```
$ idiot store use staging
$ idiot auth login work-creds
$ idiot status
  auth    logged in  work-creds · you@example.com
  store   staging    abc123xyz
  arch    arm64
  lp      yourname
```

---

## Install

From source:

```sh
make install                   # installs to ~/.local by default
make install PREFIX=/usr/local # optional: override install prefix
```

This builds `idiot-store`, idiot's Snap Store client, so it needs a Rust
toolchain (`cargo`). `idiot auth` doesn't need snapcraft: `idiot auth add`
logs in itself and can scope the credential
(`--permissions`, `--snaps`, `--channels`, `--expires`).

Or as a snap (strictly confined, no interfaces to connect; built for
amd64, arm64, armhf and riscv64):

```sh
snap install idiot
snap install idiot+core idiot+network   # optional tool components, any subset
```

The tools idiot's commands need, and many that Ubuntu Core lacks, come in
optional components. Each runs on the host (or a Core device) with no
packages installed; a host's own copy of a tool is used first when it has
one.

| Component | For | Tools (main ones) | Download |
|---|---|---|---|
| `core` | disk images | debugfs, mke2fs, mtools, sfdisk, sgdisk, parted, erofs, zstd, lz4, cpio, simg2img | 3 MB |
| `vm` | `idiot vm` | qemu for amd64, arm64, armhf, riscv64; qemu-img; OVMF/AAVMF/RISC-V UEFI firmware | 37 MB |
| `tpm` | TPMs | tpm2-tools, swtpm, swtpm_setup | 2.5 MB |
| `network` | networks | tcpdump, nmap, iw, dig, traceroute, mtr, iperf3, socat, curl, nft, conntrack, arping | 22 MB |
| `hardware` | devices | lspci, lsusb, i2c-tools, dmidecode, lshw, smartctl, nvme, gpiod, can-utils, picocom, mmc, flashrom | 3.5 MB |
| `boot` | boot chain | efibootmgr, mokutil, efitools, keyctl, mkimage, fw_printenv, dtc, sbsign/sbverify | 1 MB |
| `debug` | processes and binaries | strace, ltrace, gdb, lsof, file, binutils, elfutils, jq, sysstat, htop, patchelf | 15 MB |

With `idiot+vm` (and `idiot+tpm` for `--tpm`), `idiot vm` boots images on
a host without qemu, firmware or swtpm packages, including an Ubuntu Core
device; a host qemu, with its own firmware, still wins when installed.
armhf and riscv64 have no secure-boot firmware, so they boot without it.
The bundled qemu has no display window: use `--no-gfx` (the default).

Run a bundled tool with `idiot run <tool>` (the bundled copy first, then
the host's), as root with `idiot run --sudo <tool>` (plain `sudo` resets
`PATH`, so it can't find one), or open a shell with every installed
component's tools first on `PATH` with `idiot run --shell`.
`idiot run --list` prints each component, whether it's installed, and its
tools, tab-separated. A command that needs a tool you don't have says
which component to install. Tools core24 itself ships
(ip, ethtool, iptables, unsquashfs, cryptsetup, nc, openssl) are on `PATH`
on Ubuntu Core and aren't repeated. `swtpm_setup` can't create EK or
platform certificates from the bundle (its certificate tool's path is
absolute); install the host's swtpm-tools for that.

Every component is built for each of the snap's architectures with the
same tools, except ltrace, which Ubuntu doesn't build for riscv64.

Upgrading: tpm2-tools moved from `core` to `tpm`; if you used them, run
`snap install idiot+tpm`.

The snap keeps its state in `~/snap/idiot/common`, separate from a source
install's `~/.local/share/idiot`. To carry credentials, stores and models
over from a source install, copy them once:

```sh
cp -a ~/.local/share/idiot/. ~/snap/idiot/common/
```

Add shell integration to your rc file (run once):

```sh
eval "$(idiot init bash)"   # or zsh
idiot init fish | source    # fish
```

The `iot` shorthand is also available.

---

## How it works

The shell wrapper hands each `idiot` command a private temp file; commands
that change your environment write `export`/`unset` lines there, and the
wrapper reads, deletes and evaluates the file afterwards, so the changes land
in your current shell, not a subprocess. Nothing a command prints is ever
eval'd: stdout and stderr reach your terminal or pipe as-is
(`idiot auth export work | …`, `idiot run …`).

Under the snap, commands that only manage idiot's own state (`auth login`,
`store use`, `arch`, …) run confined. Commands that need host tools, the
network or your files (`vm`, `run`, `inspect`, `auth add`, …) are handed
back to your shell through the same file and run there, as you, from the
snap's files; that's why the snap needs no interfaces. Your `PATH` and
`LD_LIBRARY_PATH` are never changed: a tool bundled in the snap gets what it
needs in its own process only. After a snap refresh nothing needs reloading.

After upgrading from a version that eval'd stdout, reload the integration
once in already-open shells (`eval "$(idiot init bash)"`); idiot tells you
when it's needed.

---

## Development

```sh
make check    # shfmt + shellcheck, cargo fmt --check + clippy
make fmt      # auto-format shell and Rust files
cargo test --manifest-path tools/idiot-store/Cargo.toml   # helper unit tests (offline)
make purge    # uninstall + delete all user data
```

After `snapcraft pack`, test the snap and its components on a bare host
(a throwaway LXD container with none of their packages):

```sh
snap/local/check-components idiot_*_amd64.snap   # every promised tool is in its component
tests/bare-host idiot_*_amd64.snap                # ...and runs (all components next to it)
tests/bare-host idiot_*_amd64.snap idiot+vm.comp idiot+tpm.comp   # just these
tests/bare-host --boot download --kvm idiot_*_amd64.snap   # also boot Ubuntu Core 24
tests/bare-host --remote my-arm64-box idiot_*_arm64.snap   # another arch, on its own LXD
```

It never uses the default LXD remote (`--remote`, default `local`), mounts
nothing from the host, and deletes the container afterwards (`--keep` to
look around).

---

## Contributing

```sh
make hooks    # wire up git hooks (run once after cloning)
```

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
The `pre-commit` hook runs `make check`; `commit-msg` enforces the format.

---

## License

MIT © Lauren Brock
