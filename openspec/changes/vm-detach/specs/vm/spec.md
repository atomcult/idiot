# Spec Delta

## ADDED Requirements

### Requirement: Detached VMs
`idiot vm boot --detach` (and `idiot vm --detach <image>`) SHALL start the
VM in the background and return once QEMU has started it, leaving the VM
running after the command, and the shell it ran in, have gone. It SHALL
check and refuse what a foreground boot checks and refuses, and fail with
QEMU's error when QEMU cannot start the VM. On success it SHALL say the
VM's name and ssh port and name `idiot vm logs --console -f` and
`idiot vm stop`, and print nothing on stdout. A detached VM's emulated TPM
SHALL keep running with it, and SHALL still stop when QEMU exits, however
QEMU exits. `--detach` SHALL apply only to the boot it is given with: it
SHALL NOT be recorded in the VM's settings.

#### Scenario: Boot and carry on
- **WHEN** the user runs `idiot vm boot --detach --tpm core.img` in a terminal
- **THEN** the command returns to the prompt with the VM's name and ssh port, `idiot vm list` shows the VM running, and `idiot vm logs --console -f` shows the guest booting

#### Scenario: Stop a detached VM
- **WHEN** the VM `a` was booted with `--detach --tpm` and the user runs `idiot vm stop a`
- **THEN** the guest shuts down, qemu exits, and no swtpm for `a` remains

#### Scenario: QEMU cannot start
- **WHEN** the user runs `idiot vm boot --detach --name a core.img -- -no-such-flag`
- **THEN** the command fails with QEMU's error, the VM is not running, and no swtpm for `a` remains

#### Scenario: Not remembered
- **WHEN** the VM `a` was booted with `--detach`, stopped, and the user runs `idiot vm boot --name a`
- **THEN** the VM boots in the foreground on the terminal's serial console

## MODIFIED Requirements

### Requirement: Serial console log
The guest's serial console SHALL stay interactive on the terminal running
`idiot vm` (with and without `--gfx`), unless the VM was booted with
`--detach`, and everything the guest writes to it SHALL also be written to
a log file in the VM's state directory. A detached VM's console SHALL be
written only to that log. Each boot SHALL start a new log, keeping the
previous boot's. `idiot vm logs --console [--name <name>]` SHALL print the
current log to stdout; `--previous` SHALL print the previous boot's; `-f`
SHALL keep printing what the guest writes until interrupted. A snap name
and `-n` SHALL be refused with `--console`, and `--previous` without it.
Without a name it SHALL use the only running VM, or, when none is running,
the only VM.

#### Scenario: Read the console of a running VM
- **WHEN** the VM `a` is booting in one terminal and the user runs `idiot vm logs --console --name a | grep -i secure` in another
- **THEN** grep prints the firmware's and kernel's secure boot lines from the current boot

#### Scenario: Console of a detached VM
- **WHEN** the VM `a` was booted with `--detach` and the user runs `idiot vm logs --console -f --name a`
- **THEN** the guest's console output is printed as the guest writes it, until the VM stops or the user interrupts

#### Scenario: After a failed boot
- **WHEN** the VM `a` failed to boot and was booted again, and the user runs `idiot vm logs --console --previous --name a`
- **THEN** the failed boot's console output is printed

#### Scenario: Console and a snap
- **WHEN** the user runs `idiot vm logs --console my-daemon`
- **THEN** the command fails saying a snap can't be given with `--console`, and nothing is printed on stdout

#### Scenario: Old name
- **WHEN** the user runs `idiot vm console-log --previous a`
- **THEN** stderr notes that it is now `idiot vm logs --console`, and the previous boot's console output of `a` is printed
