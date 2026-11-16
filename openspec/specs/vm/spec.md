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
`idiot vm vars [--name <name> | <state-dir>]` SHALL list the UEFI
variables stored in the VM's `fw_vars.fd` (an OVMF or AAVMF variable
store): the named VM's, or the given state directory's, or, with neither,
the only running VM's (as in "Choosing a VM"). It SHALL NOT read the
host's own variables. It SHALL print each live variable with its vendor
GUID and size, and for PK, KEK, db and dbx each enrolled X.509 certificate
by subject and the number of entries of each hash type. The listing SHALL
go to stdout and SHALL need no tool from the host or a component. A missing state
directory, an unknown VM, a state directory without `fw_vars.fd`, or a
file that isn't a variable store SHALL fail with an error on stderr.

#### Scenario: Secure boot keys of a VM
- **WHEN** a VM was booted with `idiot vm --state-dir ./state image.img` (secure boot) and the user runs `idiot vm vars ./state`
- **THEN** stdout lists the VM's variables, with PK, KEK and db showing the enrolled certificates' subjects (for Ubuntu's OVMF: the Ubuntu OVMF key and Microsoft's KEK and UEFI CAs)

#### Scenario: Named VM
- **WHEN** the VM `a` was booted with secure boot and the user runs `idiot vm vars --name a`
- **THEN** stdout lists `a`'s variables the same way

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

#### Scenario: Host without Python
- **WHEN** the host has no python3 and the user runs `idiot vm vars ./state`
- **THEN** the variables are listed the same way

### Requirement: Named VMs
Every VM `idiot vm` boots SHALL have a name and a state directory holding
everything that VM keeps between runs: its disk overlay, UEFI variables,
TPM state, serial log, ssh port and the settings it was launched with
(image, architecture, secure boot, TPM). `--name <name>` SHALL set the
name; without it the name SHALL be the image's file name without its
directory and its `.img` or `.raw` extension, with characters other than
letters, digits, `-`, `_` and `.` replaced by `-`. A name SHALL be 1 to 32
of those characters and SHALL NOT start with `.` or `-`; any other name
SHALL fail before anything is written. The state directory SHALL be
`${IDIOT_DATA_HOME}/vm/<name>`, or the directory given with
`--state-dir <dir>`, which the VM's name SHALL then refer to until the VM
is deleted. State directories and the files in them SHALL be readable and
writable by the user only. `idiot vm --name <name>` without an image SHALL
boot that existing VM again with its recorded settings. `--tpm`/`--no-tpm`,
`--gfx`/`--no-gfx`, `--ssh-port` and `--port` given on the command line
SHALL replace the recorded ones; the image, `--arch`, `--sb`/`--no-sb` and
`--in-place` are fixed when the VM is created, and booting a VM with a
different one SHALL fail, naming the recorded value and
`idiot vm delete <name>`. Booting a VM that is already running
SHALL fail, naming its ssh port.

#### Scenario: Today's invocation
- **WHEN** the user runs `idiot vm ubuntu-core-24-amd64.img`
- **THEN** the image boots as the VM `ubuntu-core-24-amd64`, with its state in `${IDIOT_DATA_HOME}/vm/ubuntu-core-24-amd64`

#### Scenario: Two VMs from one image
- **WHEN** the user runs `idiot vm --name a core.img` in one terminal and `idiot vm --name b core.img` in another
- **THEN** both VMs run at the same time, each with its own disk overlay, UEFI variables, TPM state and ssh port

#### Scenario: Boot again by name
- **WHEN** the VM `gadget-test` was booted with `--tpm --arch arm64` and has stopped, and the user runs `idiot vm --name gadget-test`
- **THEN** it boots the same image again for arm64 with its TPM, with what the guest wrote last time

#### Scenario: Different image
- **WHEN** the VM `a` was created from `core.img` and the user runs `idiot vm --name a other.img`
- **THEN** the command fails naming `core.img` and `idiot vm delete a`, and nothing is booted

#### Scenario: Different architecture
- **WHEN** the VM `a` was created for amd64 and the user runs `idiot vm --name a --arch arm64`
- **THEN** the command fails naming `amd64` and `idiot vm delete a`

#### Scenario: Already running
- **WHEN** the VM `a` is running and the user runs `idiot vm --name a`
- **THEN** the command fails saying `a` is running, with its ssh port

#### Scenario: Invalid name
- **WHEN** the user runs `idiot vm --name '../x' core.img`
- **THEN** the command fails saying the name is invalid, and nothing is created

#### Scenario: Explicit state directory
- **WHEN** the user runs `idiot vm --tpm --state-dir ./state core.img`
- **THEN** the TPM state and UEFI variables are kept in `./state`, and `idiot vm list` shows the VM `core`

### Requirement: Disk overlay
By default `idiot vm` SHALL NOT write to the image: the VM SHALL boot from
a qcow2 overlay in its state directory, backed by the image opened
read-only, and keep what the guest writes in the overlay between runs.
With `--in-place` the VM SHALL write to the image itself, as recorded in
its settings, and SHALL have no overlay. If the image changed (size or
modification time) since the overlay was created, the boot SHALL fail,
naming `idiot vm reset <name>`, rather than boot an overlay on a different
base. A missing image SHALL fail naming the path the VM was created from.

#### Scenario: Image left pristine
- **WHEN** the user boots `core.img` (an Ubuntu Core image, whose first boot repartitions the disk) and shuts the VM down
- **THEN** `core.img` is byte-for-byte what it was before, and booting the VM again continues from where the guest left off

#### Scenario: Writing the image
- **WHEN** the user runs `idiot vm --in-place core.img` and the guest completes its first boot
- **THEN** `core.img` holds the booted system, and `idiot inspect image core.img` reports `First boot: complete`

#### Scenario: Image replaced underneath
- **WHEN** the user decompresses a new `core.img` over the one the VM `core` was created from and runs `idiot vm core.img`
- **THEN** the command fails naming `idiot vm reset core`, and the overlay is not used

### Requirement: SSH port per VM
Each VM SHALL forward one host port to its guest port 22 and record it.
`--ssh-port <port>` SHALL use that port. Without it a VM SHALL use the
port it recorded last time if that port is free, else 8022 if free, else
the first free port above 8022, and SHALL say which port it took when it
is not the one it had. A port is not free when another VM that is running
uses it, or when something on the host listens on it. A `--ssh-port` that
is not free SHALL fail before qemu starts.

#### Scenario: First VM
- **WHEN** no VM is running, 8022 is free, and the user runs `idiot vm core.img`
- **THEN** the VM's ssh is forwarded from host port 8022

#### Scenario: Second VM
- **WHEN** the VM `a` is running on 8022 and the user runs `idiot vm --name b core.img`
- **THEN** `b` is forwarded from a different free port, which idiot says and `idiot vm list` shows

#### Scenario: Port taken
- **WHEN** something listens on 9000 and the user runs `idiot vm --ssh-port 9000 core.img`
- **THEN** the command fails saying port 9000 is in use, and qemu is not started

#### Scenario: Port taken, host without Python
- **WHEN** the host has no python3, something listens on 9000, and the user runs `idiot vm --ssh-port 9000 core.img`
- **THEN** the command fails saying port 9000 is in use, and qemu is not started

### Requirement: Listing VMs
`idiot vm list` SHALL print one line per VM to stdout, under a header
line: its name, image path, architecture, `running` or `stopped`, and ssh
port. A VM SHALL be `running` only while its qemu process is alive. With
no VMs it SHALL print nothing to stdout and say so on stderr.

#### Scenario: Running and stopped
- **WHEN** the VM `a` is running on 8022 and the VM `b` is stopped
- **THEN** `idiot vm list` prints `a` as running with port 8022 and `b` as stopped with its recorded port

#### Scenario: Stale pid
- **WHEN** a VM's qemu was killed with SIGKILL and its pid file remains
- **THEN** `idiot vm list` shows the VM as stopped

#### Scenario: Pipeable
- **WHEN** the user runs `idiot vm list | grep running`
- **THEN** grep prints the running VMs' lines

### Requirement: Stopping a VM
`idiot vm stop [<name>]` SHALL ask the guest to power down with an ACPI
power button event and wait for qemu to exit, up to `--timeout <seconds>`
(default 60). If qemu is still running then, it SHALL make qemu quit, and
kill it if that fails, and say the guest did not shut down in time. With
`--force` it SHALL quit qemu straight away. Stopping SHALL also stop the
VM's swtpm. Without a name it SHALL stop the only running VM, as in
"Choosing a VM". Stopping a VM that is not running SHALL say so and
succeed.

#### Scenario: Graceful shutdown
- **WHEN** the VM `a` runs Ubuntu Core and the user runs `idiot vm stop a`
- **THEN** the guest shuts down cleanly, qemu exits, the terminal running `idiot vm` returns to its prompt, and no swtpm for `a` remains

#### Scenario: Guest ignores the button
- **WHEN** the guest does not react to ACPI and the user runs `idiot vm stop --timeout 5 a`
- **THEN** after 5 seconds qemu is made to quit and the command says the guest did not shut down in time

#### Scenario: Graceful shutdown, host without Python
- **WHEN** the host has no python3, the VM `a` runs Ubuntu Core with `--tpm`, and the user runs `idiot vm stop a`
- **THEN** the guest shuts down cleanly through the power button, not by qemu quitting, and no swtpm for `a` remains

### Requirement: Resetting a VM
`idiot vm reset <name>` SHALL discard a stopped VM's disk overlay, UEFI
variables and TPM state, and its serial logs, keeping its name, image, ssh
port and settings, so that its next boot is that of a freshly flashed
device with factory secure boot keys and an empty TPM. Resetting a running
VM SHALL fail, naming `idiot vm stop <name>`. For an `--in-place` VM it
SHALL reset the UEFI variables and TPM state and say that the image itself
cannot be reset.

#### Scenario: Freshly flashed
- **WHEN** the VM `a` has completed its first boot and the user runs `idiot vm stop a`, `idiot vm reset a` and `idiot vm --name a`
- **THEN** the guest runs install mode again, as on a newly flashed device, from the unchanged image

#### Scenario: Running VM
- **WHEN** the VM `a` is running and the user runs `idiot vm reset a`
- **THEN** the command fails naming `idiot vm stop a`, and nothing is discarded

### Requirement: Deleting a VM
`idiot vm delete <name>` SHALL remove a stopped VM's state directory and
forget its name (for a `--state-dir` VM: the files idiot created in that
directory, and the directory only if it is then empty),
after asking for confirmation on the terminal; `--yes` SHALL skip the
question, and without a terminal and without `--yes` it SHALL fail. It
SHALL NOT touch the image. Deleting a running VM SHALL fail, naming
`idiot vm stop <name>`.

#### Scenario: Delete
- **WHEN** the VM `a` is stopped and the user runs `idiot vm delete --yes a`
- **THEN** its state directory is gone, `idiot vm list` no longer shows it, and its image is unchanged

### Requirement: Serial console log
The guest's serial console SHALL stay interactive on the terminal running
`idiot vm` (with and without `--gfx`), and everything the guest writes to
it SHALL also be written to a log file in the VM's state directory. Each
boot SHALL start a new log, keeping the previous boot's. `idiot vm
console-log [<name>]` SHALL print the current log to stdout; `--previous`
SHALL print the previous boot's; `-f` SHALL keep printing what the guest
writes until interrupted. Without a name it SHALL use the only running VM,
or, when none is running, the only VM.

#### Scenario: Read the console of a running VM
- **WHEN** the VM `a` is booting in one terminal and the user runs `idiot vm console-log a | grep -i secure` in another
- **THEN** grep prints the firmware's and kernel's secure boot lines from the current boot

#### Scenario: After a failed boot
- **WHEN** the VM `a` failed to boot and was booted again, and the user runs `idiot vm console-log --previous a`
- **THEN** the failed boot's console output is printed

### Requirement: Choosing a VM
`idiot vm ssh`, `scp` and `vars` SHALL act on the VM named with
`--name <name>`, and `stop` and `console-log` on the VM named as their
argument. Without a name they SHALL act on the only running VM. If no VM
is running they SHALL fail saying so; if several are, they SHALL offer a
picker of the running VMs when a terminal is attached and fzf is
available, and otherwise fail listing their names and naming `--name`.
`reset`, `delete` and `tpm` act on stopped VMs and SHALL require a name
(`tpm`: `--name <name>` or a state directory). An unknown name SHALL fail
naming `idiot vm list`. `ssh` and `scp` SHALL connect to the chosen VM's
recorded ssh port; `--port <port>` SHALL still connect to that port
without choosing a VM. `idiot vm tpm` SHALL refuse a running VM, whose
swtpm holds the state.

#### Scenario: One VM running
- **WHEN** only the VM `b` is running, on port 8023, and the user runs `idiot vm ssh`
- **THEN** ssh connects to `localhost` port 8023

#### Scenario: Several running, no terminal
- **WHEN** the VMs `a` and `b` are running and the user runs `idiot vm ssh -- true </dev/null` from a script
- **THEN** the command fails listing `a` and `b` and naming `--name`

#### Scenario: Copy into a named VM
- **WHEN** the user runs `idiot vm scp --name a ./pc.snap vm:`
- **THEN** the file is copied into VM `a` over its recorded ssh port

### Requirement: Installing snaps into a VM
`idiot vm install [--name <name>] <file.snap>... [--dangerous] [--classic]
[--devmode]` SHALL copy each snap file into the chosen VM (as in
"Choosing a VM"), install them in the guest as root in one snapd change,
follow that change until it is done, and remove the copies from the guest
afterwards. Every file SHALL be checked to exist on the host before
anything is copied. When every snap has an assertion file next to it
(`<file>.assert` for `<file>.snap`), the assertions SHALL be acknowledged
in the guest and the snaps installed without `--dangerous`, unless
`--dangerous` is given; otherwise the snaps SHALL be installed with
`--dangerous` and the command SHALL say so. `--classic` and `--devmode`
SHALL be passed to snapd. When the guest reboots during the change (a
kernel, gadget or base snap), the command SHALL wait for the VM's ssh to
return, up to `--timeout <seconds>` (default 300), and keep following the
change. On success it SHALL say each installed snap's name, version and
revision; on failure it SHALL show the change's failed tasks with snapd's
error and exit non-zero. If `sudo` in the guest asks for a password, the
command SHALL fail saying so instead of prompting. All of its output SHALL
go to stderr.

#### Scenario: Unasserted app snap
- **WHEN** the VM `a` is running Ubuntu Core with a console-conf user and the user runs `idiot vm install ./hello_1.0_amd64.snap`
- **THEN** the snap is installed with `--dangerous`, the command says it was installed without assertions and prints `hello` with its version and an `x1` revision, and no copy of the file remains in the guest

#### Scenario: With assertions
- **WHEN** the user ran `snap download hello-world`, giving `hello-world_29.snap` and `hello-world_29.assert`, and runs `idiot vm install hello-world_29.snap`
- **THEN** the assertion is acknowledged and the snap installed as revision 29 without `--dangerous`

#### Scenario: Kernel snap reboots the guest
- **WHEN** the user runs `idiot vm install --name a ./pc-kernel_custom.snap`
- **THEN** the command says the guest is rebooting, waits for ssh to come back, and reports the change done with the new kernel revision once snapd finishes it after the reboot

#### Scenario: Install fails
- **WHEN** the snap's install hook exits with an error
- **THEN** the command shows the failed task and the hook's error from snapd, and exits non-zero

#### Scenario: Missing file
- **WHEN** the user runs `idiot vm install ./nope.snap`
- **THEN** the command fails naming `./nope.snap`, and nothing is copied

#### Scenario: Several VMs running
- **WHEN** the VMs `a` and `b` are running and the user runs `idiot vm install --name b ./app.snap`
- **THEN** the snap is installed in `b` only

### Requirement: Guest logs
`idiot vm logs [--name <name>] [<snap>[.<app>]] [-f] [-n <lines>]` SHALL
print, on stdout, the chosen VM's journal entries for that snap's services
(as `snap logs` shows them), or, without a snap, the guest's whole journal
for the current boot. `-n` SHALL limit the output to the last `<lines>`
entries (default 10 with a snap, all without); `-f` SHALL keep printing new
entries until interrupted. The VM is chosen as in "Choosing a VM". A snap
that is not installed in the guest SHALL fail with snapd's error on
stderr and a non-zero exit.

#### Scenario: Service logs
- **WHEN** the snap `my-daemon` is installed in the only running VM and the user runs `idiot vm logs my-daemon -n 50`
- **THEN** stdout shows the last 50 journal entries of `my-daemon`'s services

#### Scenario: Follow while reinstalling
- **WHEN** the user runs `idiot vm logs -f my-daemon` in one terminal and `idiot vm install ./my-daemon.snap` in another
- **THEN** the first terminal shows the service stopping and starting again with the new revision

#### Scenario: Whole journal
- **WHEN** the user runs `idiot vm logs --name a | grep -i apparmor`
- **THEN** grep prints the AppArmor lines of `a`'s current boot

#### Scenario: Unknown snap
- **WHEN** the user runs `idiot vm logs nosuchsnap`
- **THEN** the command fails with snapd's error on stderr and nothing on stdout
