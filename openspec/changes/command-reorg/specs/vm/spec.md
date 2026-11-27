## MODIFIED Requirements

### Requirement: Serial console log
The guest's serial console SHALL stay interactive on the terminal running
`idiot vm` (with and without `--gfx`), and everything the guest writes to
it SHALL also be written to a log file in the VM's state directory. Each
boot SHALL start a new log, keeping the previous boot's. `idiot vm logs
--console [--name <name>]` SHALL print the current log to stdout;
`--previous` SHALL print the previous boot's; `-f` SHALL keep printing what
the guest writes until interrupted. A snap name and `-n` SHALL be refused
with `--console`, and `--previous` without it. Without a name it SHALL use
the only running VM, or, when none is running, the only VM.

#### Scenario: Read the console of a running VM
- **WHEN** the VM `a` is booting in one terminal and the user runs `idiot vm logs --console --name a | grep -i secure` in another
- **THEN** grep prints the firmware's and kernel's secure boot lines from the current boot

#### Scenario: After a failed boot
- **WHEN** the VM `a` failed to boot and was booted again, and the user runs `idiot vm logs --console --previous --name a`
- **THEN** the failed boot's console output is printed

#### Scenario: Console and a snap
- **WHEN** the user runs `idiot vm logs --console my-daemon`
- **THEN** the command fails saying a snap can't be given with `--console`, and nothing is printed on stdout

#### Scenario: Old name
- **WHEN** the user runs `idiot vm console-log --previous a`
- **THEN** stderr notes that it is now `idiot vm logs --console`, and the previous boot's console output of `a` is printed

### Requirement: Choosing a VM
`idiot vm ssh`, `scp`, `vars` and `logs --console` SHALL act on the VM
named with `--name <name>`, and `stop` on the VM named as its argument. Without a name they SHALL act on the only running VM. If no VM
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
