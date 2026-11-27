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

## Prompt

The shell integration keeps `IDIOT_PROMPT` set to the active store, arch
and credential (`staging·arm64·work`), and unset when none is active. It is
worked out in your shell after each `idiot` call, so showing it in a prompt
starts no process. A variable you change by hand shows up after the next
`idiot` call; `idiot prompt` is always current.

starship (`~/.config/starship.toml`; the default `format` includes
`$env_var`, otherwise add `${env_var.IDIOT_PROMPT}`):

```toml
[env_var.IDIOT_PROMPT]
format = "[$env_value]($style) "
style = "bold yellow"
```

bash (`~/.bashrc`, after the `idiot init` line):

```sh
PS1='${IDIOT_PROMPT:+[$IDIOT_PROMPT] }'"${PS1}"
```

zsh (`~/.zshrc`):

```sh
setopt PROMPT_SUBST
PROMPT='${IDIOT_PROMPT:+[$IDIOT_PROMPT] }'"${PROMPT}"
```

fish (`~/.config/fish/functions/fish_right_prompt.fish`):

```fish
function fish_right_prompt
    set -q IDIOT_PROMPT; and echo -n "[$IDIOT_PROMPT]"
end
```

`idiot prompt` prints the same segment on stdout, and takes
`--format store,arch,cred,lp,profile` (any of them, in any order) and
`--separator`. For a prompt that can only run commands, starship's
`custom` module can call it, but that runs idiot on every prompt: about
8 ms from a source install, about 120 ms under the snap.

```toml
[custom.idiot]
command = "idiot prompt"
when = true
format = "[$output]($style) "
style = "bold yellow"
```

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
$ idiot vm reset gadget             # discard disk, UEFI vars, TPM, snapshots: freshly flashed
$ idiot vm delete gadget            # forget it (the image is untouched)
```

`idiot vm ssh`, `scp` and `vars` take `--name <name>`; `tpm` takes
`--name <name>` or a state directory. Without a name, `ssh`, `scp`, `vars`,
`stop` and `console-log` act on the only running VM (with several, a
picker on a terminal).

The dev loop, from a built snap to its logs:

```
$ idiot vm install ./my-app_1.0_amd64.snap     # copy, install, follow the change
$ idiot vm logs -f my-app                      # its services' journal
$ idiot vm install ./pc-kernel_custom.snap     # follows it through the reboot
$ idiot vm install hello-world pc-kernel=24/edge   # from the store
```

`install` copies the snaps in and installs them as root in one snapd change,
following it to the end (waiting for ssh when a kernel, gadget or base snap
reboots the guest), then reports each snap's version and revision, or the
failed tasks. A `<file>.assert` next to every snap (as `snap download`
leaves them) makes it an asserted install; otherwise it is `--dangerous`,
and idiot says so. The guest user (your Launchpad username, as for `ssh`)
needs passwordless sudo, as console-conf and system-user accounts have.
`logs` prints a snap's service logs, or the whole journal of the current
boot, on stdout.

An argument that isn't a `.snap` file is a store snap, `<name>` or
`<name>=<channel>` (`--channel` sets it for the others; default `stable`).
idiot downloads it on the host for the VM's architecture (not
`UBUNTU_STORE_ARCH`), from the active store with the active credential
(`--cred <name>` for another saved one, anonymously when there is none),
checks it against the store's sha3-384, and fetches the assertions that
install it asserted. The credential goes to the store only, never into the
VM. Downloads are cached in `~/.cache/idiot/snaps` (readable by you only) and
reused while the channel still points at them.

Snapshots save a stopped VM's disk, UEFI variables and TPM state together,
so a VM can go back to an interesting state (first boot done, keys
enrolled) in seconds instead of a reset and a new install-mode boot:

```
$ idiot vm stop gadget
$ idiot vm snapshot create --name gadget seeded   # after its first boot
$ idiot vm snapshot list --name gadget            # name, taken, UEFI, TPM, ok/broken
$ idiot vm install ./pc-kernel_broken.snap        # ...and the kernel doesn't boot
$ idiot vm stop --force gadget
$ idiot vm snapshot restore --name gadget seeded  # back, TPM-sealed keys and all
$ idiot vm snapshot delete --name gadget seeded
```

The disk snapshot lives inside the VM's overlay (it costs only what the
guest changes afterwards) and the variables and TPM state are copied
beside it, so a restore never pairs a disk with another moment's TPM.
`restore` keeps the snapshot (restore it as often as you like), and it and
`delete` ask first (`--yes` skips the question). Running VMs and
`--in-place` VMs can't be snapshotted; a snapshot of a VM stopped with
`--force` is crash-consistent. `idiot vm reset` discards a VM's snapshots
with its overlay, asking first when it has any (`--yes` skips the
question).

**Changed:** before named VMs, `idiot vm` wrote to the image and threw
the UEFI variables and TPM state away after each run (unless you gave
`--state-dir`). Now nothing is thrown away until you reset or delete the
VM; use `--in-place` if you need the image written.

---

## Inspecting images and assertions

```
$ idiot inspect image core.img                 # model, first boot, seeded snaps, snapd state
$ idiot inspect diff old.img new.img           # only what differs (--tsv, --state)
$ idiot inspect assertion pc.model             # headers, snaps, and is the signature good?
$ snap known model | idiot inspect assertion -
```

`inspect assertion` checks each signature up the chain of account-keys
to one snapd trusts (this machine's assertion database) or Canonical's
root key, which idiot bundles with Canonical's `models` key, so Canonical
models verify offline. It never adds anything to snapd's database and
needs no root; `--remote` fetches brand keys from the store. It also warns
when a model's store isn't the active one, or a saved model (`idiot
model`) with the same name has another revision. It exits 1 when a
signature is invalid.

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
         switch: idiot store use staging · re-pair: idiot auth bind ci · any store: idiot auth bind --any ci
```

The warning never blocks anything. `idiot auth bind [<name>]` re-pairs a
credential with the active store; imported credentials, and ones saved
before idiot recorded pairings, have none and are never warned about until
you bind them.

A personal Ubuntu One login usually works in the global store and in every
brand store its account belongs to. Mark such a credential for any store
and it is never warned about: `idiot auth bind --any [<name>]`, or
`idiot auth add --any <name>` when you add it. Plain `idiot auth bind`
pairs it with one store again.
`idiot auth list` shows each credential's store after its expiry.

Credentials saved by idiot v0.2.0 and earlier can carry base64 padding that snapd
(`snap download`, `ubuntu-image`, `snap prepare-image`) can't read: it
then downloads anonymously, so private and brand-store snaps fail with a
404. idiot repairs a credential the next time you activate or export it
(`Repaired credentials 'work'`), and `idiot doctor` lists any still
affected.

### Checking a store

When a snap won't download, the store answers 404 for many reasons. `idiot
store check` works out which one:

```
$ idiot store check ce-dummy --arch arm64
Checking ce-dummy in store 'ce-team-test' for arm64, latest/stable

  ✓ credential 'work'
  ✓ member of store 'ce-team-test' (view)
  ✓ ce-dummy is visible with your credential (private to anonymous users)
  ✗ no arm64 revision in latest/stable

✗ ce-dummy isn't released for arm64 in latest/stable; arm64 revisions are in: latest/edge
```

It looks the snap up as snapd does, tries the download without and with
the credential (downloading nothing), and checks your role in the store
and the credential itself: expiry, `package_access`, and padding snapd
can't read. It exits 0 when the snap can be downloaded.

```
$ idiot store channels ce-dummy          # revisions per channel and arch (--tsv)
$ idiot store roles                      # your stores and roles, the active one marked
$ idiot store snaps                      # the snaps in the active store (--pick: fzf)
$ idiot store snaps --cred ce-admin      # with another saved credential, once
$ idiot store members --role admin       # who is in the store, and with which roles
$ idiot store import                     # save your stores (pick and name them; --all for all)
```

`idiot store browse` explores the store in fzf: the snaps `store snaps`
lists (or the ones you name), each one's channel map in the preview, and,
when you open a snap, its channel map and every revision (released or
not, when your credential may read the history). Enter on a row prints
snap, channel, architecture and revision; Escape goes back. Filters
narrow the list and the rows, and `--tsv` prints the rows instead:

```
$ idiot store browse                                  # the whole store, in fzf
$ idiot store browse --arch arm64 --channel edge      # snaps with an arm64 edge revision
$ idiot store browse --version '1.4*' --tsv           # where is 1.4 released?
$ idiot store browse --publisher alice --private      # alice's private snaps
$ idiot store browse ce-dummy --unreleased --revisions --tsv
$ idiot store browse hello core22 --tsv               # public snaps work without a credential
```

`idiot store assertion` fetches assertions from the store and summarises
them as `idiot inspect assertion` does (`--raw` prints them as served,
`-o` saves them):

```
$ idiot store assertion account canonical
$ idiot store assertion snap-declaration hello-world
$ idiot store assertion model canonical/ubuntu-core-24-amd64
$ idiot store assertion validation-set acme/base-set      # the latest sequence
$ idiot store assertion serial brand-id=acme model=gw serial=1234
```

With `--chain` it also fetches everything snapd needs before it accepts
the assertion (signing keys, the snap's declaration, accounts), so a
side-loaded snap can be installed asserted:

```
$ idiot store assertion --chain -o hello.assert snap-revision hello-world 29
$ sudo snap ack hello.assert && sudo snap install ./hello-world_29.snap
```

`store snaps` lists a whole brand store only for its admins, with a
credential that has `store_admin` (`idiot auth add ce-admin --permissions
store_admin,package_access`); otherwise it lists the snaps you publish or
collaborate on there and says what the full list needs. `store members`
needs the same (an admin with `store_admin`) and fails without it; it
leaves members' emails out unless asked (`--email`). `--cred <name>`
works on all of them and never changes the active credential.

idiot also knows when each credential expires, read offline from the
credential itself when it's added or imported (or from the store, the
first time `idiot auth status` runs for an older one). `idiot status`,
`idiot auth list` and `idiot auth status` show it (`expires in 41d`,
yellow within a week, red once expired), and the first idiot command
each day warns when the active credential expires within
`IDIOT_CRED_WARN_DAYS` days (default 7; `0` warns only once it has
expired):

```
idiot: credential 'ci' expires in 3d — renew with: idiot auth add
```

---

## Troubleshooting

When something doesn't work, run `idiot doctor`. It checks the shell
integration, the install, every tool idiot's commands use (and which copy
they'd run: yours or a snap component's), KVM and the VM firmware,
credential permissions and expiry, saved stores, and data left behind by
another install, without changing anything or touching the network:

```
$ idiot doctor
Shell integration
  ✓ shell integration           bash, from the snap
  ✓ data directory              ~/snap/idiot/common

Install
  ✓ snap                        idiot, revision x7, v0.1.0
  ✓ idiot-store helper          /snap/idiot/x7/lib/idiot/libexec/idiot-store
  ✓ idiot-helper                /snap/idiot/x7/lib/idiot/libexec/idiot-helper

Tools
  ✓ qemu-system-x86_64          vm component
  ! swtpm not found — needed by idiot vm --tpm; snap install idiot+tpm (or install the swtpm package)
  ...

VMs
  ! /dev/kvm is not accessible — VMs will be emulated (slow); run: sudo usermod -aG kvm you, then log in again
  ✓ amd64 secure boot firmware  /snap/idiot/components/x7/vm/usr/share/OVMF/OVMF_CODE_4M.secboot.fd OVMF_VARS_4M.ms.fd
  ...

Credentials and stores
  ✓ credentials                 3 credentials, private
  ! credential 'old' has padding snapd can't read (private snap downloads 404) — repair it with: idiot auth login old
  ✓ active credential           work
  ✓ work                        expires in 41d
  ! credential 'ci' expires in 3d — renew it with: idiot auth add
  ✓ stores                      1 saved store

Data directories
  ✓ data                        ~/snap/idiot/common
  ✓ eval files                  none stale

4 warnings
```

`✓` is fine, `!` is worth knowing (something some commands need is
missing), `✗` is broken, and makes it exit 1. `idiot doctor --quiet` prints
only the `!` and `✗` lines, and nothing when all is well. It runs without
the shell integration too, to tell you that's what's missing.

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
