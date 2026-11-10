# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Saved UEFI variables
`idiot vm vars [--name <name> | <state-dir>]` SHALL list the UEFI
variables stored in the VM's `fw_vars.fd` (an OVMF or AAVMF variable
store): the named VM's, or the given state directory's, or, with neither,
the only running VM's (as in "Choosing a VM"). It SHALL NOT read the
host's own variables. It SHALL print each live variable with its vendor
GUID and size, and for PK, KEK, db and dbx each enrolled X.509 certificate
by subject and the number of entries of each hash type. The listing SHALL
go to stdout and SHALL need no tool beyond python3. A missing state
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
