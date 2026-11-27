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

### Requirement: Port forwards per VM
Each `--port <host:guest>` SHALL forward that host port to the guest port
and be recorded with the VM, so that booting it again without `--port`
forwards the same ports. A host port SHALL be free as an ssh port is (see
"SSH port per VM"), counting the forwards of other running VMs as well as
their ssh ports. A host port given with `--port` that is not free SHALL
fail before qemu starts, naming the running VM that forwards it when it is
one. A recorded forward whose host port is not free SHALL move to the
first free port above it, keeping its guest port; idiot SHALL say the new
mapping and record it. A `--port` host port that is the VM's ssh port, or
that is given twice, SHALL be refused.

#### Scenario: Second VM, same forward
- **WHEN** the VM `a` is running with `--port 8080:80` and the user runs `idiot vm boot --name b --port 8080:80 core.img`
- **THEN** the command fails saying port 8080 is forwarded by `a`, and qemu is not started

#### Scenario: Recorded forward taken
- **WHEN** the stopped VM `b` recorded `--port 8080:80`, the VM `a` is running with 8080 forwarded, and the user runs `idiot vm boot --name b`
- **THEN** `b` boots with 8081 forwarded to its port 80, idiot says so, and `idiot vm list` shows `8081:80` for `b`

#### Scenario: Clash with its own ssh port
- **WHEN** the user runs `idiot vm boot --ssh-port 9000 --port 9000:80 core.img`
- **THEN** the command fails before qemu starts, saying 9000 is the VM's ssh port

### Requirement: VMs sharing an image
Booting a VM SHALL fail before qemu starts, naming the other VM, when
another running VM writes the same image `--in-place`, or when the VM
writes its image `--in-place` and another running VM boots from that
image. VMs booting from overlays of one image SHALL run side by side.

#### Scenario: Overlays side by side
- **WHEN** the VM `a` is running from an overlay of `core.img` and the user runs `idiot vm boot --name b core.img`
- **THEN** `b` boots from its own overlay of `core.img`

#### Scenario: Image written in place
- **WHEN** the VM `a` is running `--in-place` on `core.img` and the user runs `idiot vm boot --name b core.img`
- **THEN** the command fails saying `a` writes `core.img` in place, and qemu is not started

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

### Requirement: Listing VMs
`idiot vm list` SHALL print one line per VM to stdout, under a header
line: its name, image path, architecture, `running` or `stopped`, ssh
port, and its `--port` forwards (`host:guest`, comma-separated, `-` for
none). A VM SHALL be `running` only while its qemu process is alive. With
no VMs it SHALL print nothing to stdout and say so on stderr.

#### Scenario: Running and stopped
- **WHEN** the VM `a` is running on 8022 and the VM `b` is stopped
- **THEN** `idiot vm list` prints `a` as running with port 8022 and `b` as stopped with its recorded port

#### Scenario: Forwards
- **WHEN** the VM `a` was booted with `--port 8080:80 --port 8443:443`
- **THEN** `idiot vm list` shows `8080:80,8443:443` for `a`, and `-` for a VM without forwards

#### Scenario: Stale pid
- **WHEN** a VM's qemu was killed with SIGKILL and its pid file remains
- **THEN** `idiot vm list` shows the VM as stopped

#### Scenario: Pipeable
- **WHEN** the user runs `idiot vm list | grep running`
- **THEN** grep prints the running VMs' lines
