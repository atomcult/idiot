## ADDED Requirements

### Requirement: Snapshot names and the VM they belong to
Every `idiot vm snapshot` subcommand SHALL act on the VM named with
`--name <vm>`, or, without it, on the only VM there is; with several VMs
and no `--name` it SHALL fail listing them and naming `--name`, and an
unknown VM SHALL fail naming `idiot vm list`. A snapshot name SHALL be 1
to 32 letters, digits, `.`, `-` and `_`, SHALL NOT start with `.` or `-`,
and SHALL NOT consist of digits only; any other name SHALL fail before
anything is written. Snapshot names SHALL be unique within a VM, and two
VMs MAY have snapshots of the same name.

#### Scenario: Only VM
- **WHEN** `a` is the only VM and the user runs `idiot vm snapshot list`
- **THEN** `a`'s snapshots are listed

#### Scenario: Several VMs, no name
- **WHEN** the VMs `a` and `b` exist and the user runs `idiot vm snapshot create clean`
- **THEN** the command fails listing `a` and `b` and naming `--name`, and no snapshot is taken

#### Scenario: Numeric name
- **WHEN** the user runs `idiot vm snapshot create --name a 1`
- **THEN** the command fails saying the snapshot name is invalid, and no snapshot is taken

### Requirement: Snapshots need a stopped VM with an overlay
`idiot vm snapshot create`, `restore` and `delete` SHALL refuse a running
VM, naming `idiot vm stop <vm>`, and SHALL change nothing; there are no
snapshots of running VMs. `create` and `restore` SHALL refuse a VM created
with `--in-place`, saying that snapshots need a disk overlay. `create`
SHALL refuse a VM that has no disk overlay yet (never booted, or reset
since), naming `idiot vm --name <vm>` to boot it. `list` SHALL work
whether or not the VM is running.

#### Scenario: Running VM
- **WHEN** the VM `a` is running and the user runs `idiot vm snapshot create --name a clean`
- **THEN** the command fails naming `idiot vm stop a`, and no snapshot is taken

#### Scenario: In-place VM
- **WHEN** the VM `raw` was created with `--in-place` and the user runs `idiot vm snapshot create --name raw clean`
- **THEN** the command fails saying snapshots need a disk overlay, and nothing is written

#### Scenario: Never booted
- **WHEN** the VM `a` was reset and the user runs `idiot vm snapshot create --name a clean`
- **THEN** the command fails naming `idiot vm --name a`

#### Scenario: Listing while running
- **WHEN** the VM `a` is running and has the snapshot `clean`
- **THEN** `idiot vm snapshot list --name a` lists `clean`

### Requirement: Creating a VM snapshot
`idiot vm snapshot create [--name <vm>] <snapshot>` SHALL save the stopped
VM's state under that name: the contents of its disk overlay, its UEFI
variables when it has `fw_vars.fd`, and its TPM state when it has any. It
SHALL NOT write to the image. Saving the disk SHALL NOT copy the overlay:
the snapshot SHALL cost only the disk space of what the guest changes
afterwards. A name the VM already has a snapshot of SHALL fail, naming
`idiot vm snapshot delete`. When the snapshot can't be completely saved,
no part of it SHALL remain. The snapshot's files SHALL be readable and
writable by the user only. On success the command SHALL say the
snapshot's name and which parts it holds, on stderr.

#### Scenario: After first boot
- **WHEN** the VM `a` (secure boot, `--tpm`) completed its first boot, the user ran `idiot vm stop a` and runs `idiot vm snapshot create --name a seeded`
- **THEN** the snapshot `seeded` holds `a`'s disk, UEFI variables and TPM state, and the image is unchanged

#### Scenario: Without a TPM
- **WHEN** the VM `b` was never booted with `--tpm` and the user runs `idiot vm snapshot create --name b clean`
- **THEN** the snapshot `clean` holds `b`'s disk and UEFI variables, and the command says it has no TPM state

#### Scenario: Name taken
- **WHEN** the VM `a` has the snapshot `seeded` and the user runs `idiot vm snapshot create --name a seeded`
- **THEN** the command fails naming `idiot vm snapshot delete`, and `seeded` is unchanged

#### Scenario: Cheap
- **WHEN** the VM `a`'s overlay is 3 GB and the user takes a snapshot of it
- **THEN** the state directory grows by far less than 3 GB

### Requirement: Listing VM snapshots
`idiot vm snapshot list [--name <vm>]` SHALL print one line per snapshot
of the VM to stdout, under a header line, oldest first: its name, the
local date and time it was taken, and whether it holds UEFI variables and
TPM state. A snapshot one of whose parts is missing (its disk snapshot,
or the copies idiot keeps beside it) SHALL be shown as `broken`. With no
snapshots it SHALL print nothing to stdout and say so on stderr.

#### Scenario: Two snapshots
- **WHEN** the VM `a` has the snapshots `clean` and `seeded`, taken in that order
- **THEN** `idiot vm snapshot list --name a` prints a header, then `clean`, then `seeded`, each with its date and its UEFI and TPM columns

#### Scenario: Pipeable
- **WHEN** the user runs `idiot vm snapshot list --name a | grep seeded`
- **THEN** grep prints the `seeded` line

#### Scenario: Disk snapshot removed by hand
- **WHEN** the user removed the internal snapshot `clean` from `a`'s overlay with `qemu-img snapshot -d`
- **THEN** `idiot vm snapshot list --name a` shows `clean` as broken

### Requirement: Restoring a VM snapshot
`idiot vm snapshot restore [--yes] [--name <vm>] <snapshot>` SHALL put the
stopped VM's disk overlay, UEFI variables and TPM state back to what they
were when the snapshot was taken, discarding what changed since; a part
the VM didn't have then (no `fw_vars.fd`, no TPM state) SHALL be removed,
so the next boot starts it afresh. It SHALL keep the snapshot, the VM's
settings, ssh port and serial logs. It SHALL ask for confirmation on the
terminal first; `--yes` SHALL skip the question, and without a terminal
and without `--yes` it SHALL fail. An unknown snapshot SHALL fail naming
`idiot vm snapshot list`, and a broken one SHALL fail saying which part is
missing. When restoring fails before the disk is restored, the VM's state
SHALL be unchanged; when it is interrupted after, `snapshot list` SHALL
say the VM is half-restored, and restoring the same snapshot again SHALL
complete it.

#### Scenario: Back to the seeded state
- **WHEN** the VM `a` has the snapshot `seeded`, the user installed a kernel that doesn't boot, stopped `a`, ran `idiot vm snapshot restore --yes --name a seeded` and boots `a` again
- **THEN** the guest boots as it was when `seeded` was taken, with the same secure boot variables and TPM-sealed keys, and does not run install mode

#### Scenario: Restore twice
- **WHEN** the user restores `seeded`, boots and changes `a`, stops it, and restores `seeded` again
- **THEN** `a` is back at `seeded` both times, and `seeded` is still listed

#### Scenario: TPM added later
- **WHEN** the snapshot `clean` of `a` was taken before `a` was ever booted with `--tpm`, and the user restores it after booting with `--tpm`
- **THEN** `a`'s TPM state is gone, and its next `--tpm` boot starts with an empty TPM

#### Scenario: No terminal
- **WHEN** the user runs `idiot vm snapshot restore --name a seeded </dev/null` from a script
- **THEN** the command fails naming `--yes`, and nothing is changed

#### Scenario: Unknown snapshot
- **WHEN** the VM `a` has no snapshot `nope` and the user runs `idiot vm snapshot restore --yes --name a nope`
- **THEN** the command fails naming `idiot vm snapshot list`, and nothing is changed

### Requirement: Deleting a VM snapshot
`idiot vm snapshot delete [--yes] [--name <vm>] <snapshot>` SHALL remove
every part of the snapshot that exists, broken snapshots included, and
leave the VM's current state as it is. It SHALL ask for confirmation on
the terminal first; `--yes` SHALL skip the question, and without a
terminal and without `--yes` it SHALL fail. An unknown snapshot SHALL
fail naming `idiot vm snapshot list`.

#### Scenario: Delete
- **WHEN** the VM `a` is stopped with the snapshots `clean` and `seeded`, and the user runs `idiot vm snapshot delete --yes --name a clean`
- **THEN** only `seeded` is listed, and `a` boots as it was before the command

#### Scenario: Broken snapshot
- **WHEN** the snapshot `clean` of `a` is broken and the user runs `idiot vm snapshot delete --yes --name a clean`
- **THEN** what was left of `clean` is removed, and it is no longer listed

## MODIFIED Requirements

### Requirement: Resetting a VM
`idiot vm reset [--yes] <name>` SHALL discard a stopped VM's disk overlay,
UEFI variables and TPM state, its serial logs and its snapshots, keeping
its name, image, ssh port and settings, so that its next boot is that of a
freshly flashed device with factory secure boot keys and an empty TPM.
When the VM has snapshots it SHALL say how many will be discarded and ask
for confirmation on the terminal first; `--yes` SHALL skip the question,
and without a terminal and without `--yes` it SHALL fail and discard
nothing. A VM without snapshots SHALL be reset without a question.
Resetting a running VM SHALL fail, naming `idiot vm stop <name>`. For an
`--in-place` VM it SHALL reset the UEFI variables and TPM state and say
that the image itself cannot be reset.

#### Scenario: Freshly flashed
- **WHEN** the VM `a` has completed its first boot and the user runs `idiot vm stop a`, `idiot vm reset a` and `idiot vm --name a`
- **THEN** the guest runs install mode again, as on a newly flashed device, from the unchanged image

#### Scenario: Running VM
- **WHEN** the VM `a` is running and the user runs `idiot vm reset a`
- **THEN** the command fails naming `idiot vm stop a`, and nothing is discarded

#### Scenario: Snapshots discarded
- **WHEN** the stopped VM `a` has two snapshots and the user runs `idiot vm reset a` on a terminal and answers yes
- **THEN** the command says two snapshots will be discarded, and afterwards `idiot vm snapshot list --name a` lists none

#### Scenario: Snapshots, no terminal
- **WHEN** the stopped VM `a` has a snapshot and the user runs `idiot vm reset a </dev/null` from a script
- **THEN** the command fails naming `--yes`, and nothing is discarded

### Requirement: Deleting a VM
`idiot vm delete <name>` SHALL remove a stopped VM's state directory and
forget its name (for a `--state-dir` VM: the files idiot created in that
directory, its snapshots included, and the directory only if it is then
empty), after asking for confirmation on the terminal; `--yes` SHALL skip
the question, and without a terminal and without `--yes` it SHALL fail. It
SHALL NOT touch the image. Deleting a running VM SHALL fail, naming
`idiot vm stop <name>`.

#### Scenario: Delete
- **WHEN** the VM `a` is stopped and the user runs `idiot vm delete --yes a`
- **THEN** its state directory is gone, `idiot vm list` no longer shows it, and its image is unchanged

#### Scenario: State directory with snapshots
- **WHEN** the VM `s` was booted with `--state-dir ./state`, has a snapshot, and the user runs `idiot vm delete --yes s`
- **THEN** `./state` is gone, unless it holds files idiot didn't create, which are kept
