# Spec Delta

## ADDED Requirements

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
