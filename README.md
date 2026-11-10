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

## VMs

```
$ idiot vm ubuntu-core-24-amd64.img          # the VM "ubuntu-core-24-amd64"
$ idiot vm --name gadget --tpm core.img      # another, from the same image
$ idiot vm --name gadget                     # boot it again later
```

Every VM has a name (`--name`, else the image's file name) and keeps its
disk, UEFI variables, TPM state and serial log between runs, in
`~/.local/share/idiot/vm/<name>` (`~/snap/idiot/common/vm/<name>` under the
snap) or a `--state-dir`. The image is never written: the guest writes a
qcow2 overlay, so one image can back several VMs and stays pristine.
`--in-place` writes the image itself instead, as idiot used to (for an
image you'll inspect or flash afterwards). The image, `--arch`, `--sb` and
`--in-place` are fixed when a VM is created; `--tpm`, `--gfx` and ports can
change on any boot.

Each VM forwards one host port to its ssh: 8022 for the first, the next
free one for the others (`--ssh-port` to choose), recorded so `idiot vm
ssh` finds it. The serial console stays on the terminal (Ctrl-a x quits,
Ctrl-a c is qemu's monitor) and is also logged to `serial.log` in the
VM's state directory.

```
$ idiot vm list                     # name, arch, running/stopped, port, image
$ idiot vm stop gadget              # power button, then quit after --timeout
$ idiot vm console-log -f gadget    # its serial console (--previous: last boot)
$ idiot vm reset gadget             # discard disk, UEFI vars, TPM: freshly flashed
$ idiot vm delete gadget            # forget it (the image is untouched)
```

`idiot vm ssh`, `scp` and `vars` take `--name <name>`; `tpm` takes
`--name <name>` or a state directory. Without a name, `ssh`, `scp`, `vars`,
`stop` and `console-log` act on the only running VM (with several, a
picker on a terminal).

**Changed:** before named VMs, `idiot vm` wrote to the image and threw
the UEFI variables and TPM state away after each run (unless you gave
`--state-dir`). Now nothing is thrown away until you reset or delete the
VM; use `--in-place` if you need the image written.

---

## Profiles

Save the whole setup once and switch it with one command:

```
$ idiot profile save staging-ci          # credential, store, arch, lp
$ idiot profile use staging-ci
$ idiot profile list
  * staging-ci  auth ci · store staging · arch arm64 · lp alice
    personal    auth me · store none
```

A profile records names (credential, store), never credential contents.
Settings that aren't set when you save are recorded as cleared, so using
the profile clears them (`store none` switches back to the global store).
`--fields auth,store` records only those; the rest are left as they are when
you use it. `profile use` checks everything before changing anything: a
profile naming a removed credential changes nothing. It sets
`IDIOT_LP_USER` for the current shell only and leaves the username saved
with `idiot lp set` alone. `idiot profile status` (and the `profile` row in
`idiot status`) shows what has changed since.

---

## Credentials and stores

A credential belongs to the store you were using when you added it:
`idiot auth add ci` with `staging` active pairs `ci` with `staging`. From
then on `idiot status`, `auth login`, `store use` and `store unset` warn when
the active credential and store don't match, and say how to fix it:

```
warning: credentials 'ci' belong to store 'staging', but store 'prod' is active
         switch: idiot store use staging · re-pair: idiot auth bind ci
```

The warning never blocks anything. `idiot auth bind [<name>]` re-pairs a
credential with the active store; imported credentials, and ones saved
before idiot recorded pairings, have none and are never warned about until
you bind them.

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
