# Spec Delta

## Purpose

One command that checks everything idiot depends on in the user's
environment (shell integration, install, tools, VM support, credentials,
data directories) and says what is broken and how to fix it.

## ADDED Requirements

### Requirement: Doctor report
`idiot doctor` SHALL print a report on stderr, grouped into sections
(shell integration, install, tools, VMs, credentials and stores, data
directories), with one line per check starting with `✓` (fine), `!`
(degraded or optional: something some commands need is missing, or a
setting is unusual) or `✗` (broken: something idiot needs is missing or
misconfigured, or a credential is exposed). Every `!` and `✗` line SHALL
say what to do about it. The report SHALL end with the number of problems
(`✗`) and warnings (`!`), or say that nothing was found. The command SHALL
exit 1 when any check is `✗` and 0 otherwise. It SHALL write nothing to
stdout and SHALL NOT change any file, setting or environment variable
other than the usage counter (and, with credential expiry, the
once-a-day warning stamp) every counted command updates. It
SHALL NOT contact the network, and SHALL NOT print any part of a
credential.

#### Scenario: Healthy setup
- **WHEN** everything checks out and the user runs `idiot doctor`
- **THEN** every line is `✓`, the report ends saying nothing was found, and the exit status is zero

#### Scenario: Warnings only
- **WHEN** the only findings are a missing optional component and no `/dev/kvm`
- **THEN** those lines are `!` with a hint each, and the exit status is zero

#### Scenario: Something broken
- **WHEN** a saved credential file is readable by the group
- **THEN** its line is `✗` with a hint to `chmod 600` it, and the exit status is 1

#### Scenario: Nothing on stdout
- **WHEN** the user runs `idiot doctor | wc -c`
- **THEN** `wc` prints `0` and the report is on the terminal

#### Scenario: Read-only and offline
- **WHEN** the user runs `idiot doctor` with no network
- **THEN** it completes, contacts nothing, and no file under idiot's data or cache directories changes apart from the usage counter

### Requirement: Quiet report
`idiot doctor --quiet` SHALL print only the `!` and `✗` lines (each under
its section heading) and the closing count, and SHALL print nothing at all
when every check is `✓`. Its exit status SHALL be the same as without
`--quiet`.

#### Scenario: Quiet and healthy
- **WHEN** everything checks out and the user runs `idiot doctor --quiet`
- **THEN** nothing is printed and the exit status is zero

#### Scenario: Quiet with a problem
- **WHEN** the shell integration is outdated and the user runs `idiot doctor --quiet`
- **THEN** only the integration's `✗` line, its heading and the count are printed, and the exit status is 1

### Requirement: Shell integration and install checks
`idiot doctor` SHALL report:
- `✗` when `IDIOT_SHELL` is unset (no integration) or `IDIOT_EVAL_FILE` is
  not a writable file (outdated integration), with the command to load
  it;
- under the snap, `✗` when `IDIOT_SNAP` is unset or not an existing
  directory, and `!` when `IDIOT_DATA_HOME` is not the snap's common
  directory;
- the kind of install (snap with its instance, revision and version, or
  source with its location and version), and `!` when both a source
  install and the snap are present, naming the one this shell uses;
- `✗` when the `idiot-store` helper is missing or does not run (`auth add`
  and `auth status` need it).
When the integration is missing or outdated the command SHALL still run,
directly (confined under the snap), report those checks, and mark the
checks that need the host as skipped with `!`.

#### Scenario: No integration
- **WHEN** `IDIOT_SHELL` is unset and the user runs `idiot doctor`
- **THEN** the integration line is `✗` suggesting `eval "$(idiot init bash)"` or `idiot init fish | source`, and the exit status is 1

#### Scenario: Outdated integration under the snap
- **WHEN** a shell loaded an integration without `IDIOT_EVAL_FILE` and the user runs `idiot doctor` from the snap
- **THEN** the integration line is `✗`, the host checks are reported as skipped, and the exit status is 1

#### Scenario: Two installs
- **WHEN** both `~/.local/bin/idiot` and the snap are installed and the shell loaded the snap's integration
- **THEN** a `!` line says both are installed and that this shell uses the snap

### Requirement: Tool and component checks
For each command that needs host tools (`vm`, `vm --tpm`, `inspect image`,
`inspect snap`, `kernel`, `example`, `clean`), `idiot doctor` SHALL check
every tool that command requires, as idiot itself would find it (the
host's copy first, then an installed component's), and report `✓` with
where it was found (the host path or the component), or `!` naming the
command it affects and how to get it (the host package and, under the
snap, `snap install <instance>+<component>`). Under the snap it SHALL list
which components are installed and which are available.

#### Scenario: Tool from a component
- **WHEN** the host has no qemu and `idiot+vm` is installed
- **THEN** the `qemu-system-<host arch>` line is `✓` and names the `vm` component

#### Scenario: Missing tool
- **WHEN** the host has no swtpm and `idiot+tpm` is not installed
- **THEN** a `!` line says `idiot vm --tpm` needs swtpm and suggests `snap install idiot+tpm` (under the snap) or the swtpm package

#### Scenario: Source install
- **WHEN** idiot is installed from source
- **THEN** tools are looked up on `PATH` only and no component list is shown

### Requirement: VM checks
`idiot doctor` SHALL report `/dev/kvm` as `✓` when it exists and the user
can read and write it, `!` with a hint to join the `kvm` group when it
exists but is not accessible, and `!` saying host-architecture VMs will be
emulated when it does not exist. It SHALL check the UEFI firmware
`idiot vm` would boot for the host architecture, with and without secure
boot, and for every other architecture whose qemu is available, reporting
`✓` with the files found or `!` with the package or component that
provides them.

#### Scenario: KVM not accessible
- **WHEN** `/dev/kvm` exists but the user is not in the `kvm` group
- **THEN** the KVM line is `!` and suggests adding the user to the `kvm` group

#### Scenario: Firmware missing
- **WHEN** neither the host's ovmf package nor `idiot+vm` is installed on an amd64 host
- **THEN** the amd64 firmware line is `!` naming the `ovmf` package (and `idiot+vm` under the snap)

### Requirement: Credential and store checks
`idiot doctor` SHALL report `✗` when the credentials directory is
accessible by anyone but its owner or any credential file is readable or
writable by anyone but its owner, naming the path and the `chmod` that
fixes it; `✗` when `IDIOT_ACTIVE_CRED` names a credential that is no
longer saved; `!` when the active credential's contents differ from the
saved file of that name; `!` for a saved store file that is empty or holds
more than one line; and `!` when `UBUNTU_STORE_ID` matches no saved store.
When credential expiry records exist (see the auth capability), it SHALL
also report `✗` for an expired active credential, `!` for any other
expired credential or one expiring within the warning window, and `!` for
credentials whose expiry is unknown, suggesting `idiot auth status`.
Comparing credentials SHALL NOT print them or pass them as command
arguments.

#### Scenario: Loose permissions
- **WHEN** `~/.local/share/idiot/auth/work` has mode 0644
- **THEN** a `✗` line names the file and `chmod 600`, without showing its contents

#### Scenario: Active credential removed
- **WHEN** `IDIOT_ACTIVE_CRED` is `old` and no credential `old` is saved
- **THEN** a `✗` line says the active credential no longer exists and suggests `idiot auth login` or `idiot auth logout`

#### Scenario: Expired active credential
- **WHEN** expiry records exist and the active credential has expired
- **THEN** its line is `✗` suggesting `idiot auth add`

### Requirement: Data directory checks
`idiot doctor` SHALL report `!` when the other install's data directory
holds credentials, stores or models this install does not see (under the
snap: `${XDG_DATA_HOME:-~/.local/share}/idiot`; from source:
`~/snap/<instance>/common`), with the command to copy it over; `!` when a
per-revision snap directory (`~/snap/<instance>/<revision>`) holds idiot
state; and `!` when eval files older than a day are left in the eval-file
directory, with how to delete them.

#### Scenario: Data left behind after switching to the snap
- **WHEN** `~/.local/share/idiot/auth` holds credentials and the user runs `idiot doctor` from the snap
- **THEN** a `!` line says the snap does not see them and shows how to copy `~/.local/share/idiot` into `~/snap/idiot/common`

#### Scenario: Stale eval files
- **WHEN** three `idiot.*` eval files older than a day are in the eval-file directory
- **THEN** a `!` line reports three stale eval files and how to remove them
