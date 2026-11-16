# Spec Delta

## MODIFIED Requirements

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
