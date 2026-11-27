## MODIFIED Requirements

### Requirement: Installing snaps into a VM
`idiot vm install [--name <name>] [--channel <channel>] [--cred <name>]
<snap>... [--dangerous] [--classic] [--devmode]` SHALL install each
`<snap>` in the chosen VM (as in "Choosing a VM"). An argument ending in
`.snap` SHALL be a file on the host. `<name>=<channel>`, or any other
argument, SHALL be a snap in the store, fetched as in "Installing snaps
from the store". The command SHALL copy every file and every fetched snap
into the guest, install them all as root in one snapd change, follow that
change until it is done, and remove the copies from the guest afterwards.
Every file SHALL be checked to exist on the host, and every store snap
fetched, before anything is copied. When every snap has an assertion file
next to it (`<file>.assert` for `<file>.snap`, which store snaps always
have), the assertions SHALL be acknowledged in the guest and the snaps
installed without `--dangerous`, unless `--dangerous` is given. Otherwise
the snaps SHALL be installed with `--dangerous` and the command SHALL say
so, naming any store snaps that are installed without their store
revision because of it. `--classic` and `--devmode` SHALL be passed to
snapd. When the guest reboots during the change (a kernel, gadget or base
snap), the command SHALL wait for the VM's ssh to return, up to `--timeout
<seconds>` (default 300), and keep following the change. On success it
SHALL say each installed snap's name, version and revision. On failure it
SHALL show the change's failed tasks with snapd's error and exit non-zero.
If `sudo` in the guest asks for a password, the command SHALL fail saying
so instead of prompting. All of its output SHALL go to stderr.

#### Scenario: Unasserted app snap
- **WHEN** the VM `a` is running Ubuntu Core with a console-conf user and the user runs `idiot vm install ./hello_1.0_amd64.snap`
- **THEN** the snap is installed with `--dangerous`, the command says it was installed without assertions and prints `hello` with its version and an `x1` revision, and no copy of the file remains in the guest

#### Scenario: With assertions
- **WHEN** the user ran `snap download hello-world`, giving `hello-world_29.snap` and `hello-world_29.assert`, and runs `idiot vm install hello-world_29.snap`
- **THEN** the assertion is acknowledged and the snap installed as revision 29 without `--dangerous`

#### Scenario: Store snap and file in one change
- **WHEN** `./pc_custom.assert` sits next to `./pc_custom.snap` and the user runs `idiot vm install pc-kernel=24/edge ./pc_custom.snap`
- **THEN** both snaps are installed asserted in one snapd change, and the kernel's revision is the store's `24/edge` revision for the VM's architecture

#### Scenario: Store snap with an unasserted file
- **WHEN** the user runs `idiot vm install hello-world ./my-app_1.0_amd64.snap` and `./my-app_1.0_amd64.assert` doesn't exist
- **THEN** both snaps are installed with `--dangerous` in one change, and stderr names `hello-world` as installed without its store revision

#### Scenario: Kernel snap reboots the guest
- **WHEN** the user runs `idiot vm install --name a ./pc-kernel_custom.snap`
- **THEN** the command says the guest is rebooting, waits for ssh to come back, and reports the change done with the new kernel revision once snapd finishes it after the reboot

#### Scenario: Install fails
- **WHEN** the snap's install hook exits with an error
- **THEN** the command shows the failed task and the hook's error from snapd, and exits non-zero

#### Scenario: Missing file
- **WHEN** the user runs `idiot vm install ./nope.snap`
- **THEN** the command fails naming `./nope.snap`, and nothing is copied or downloaded

#### Scenario: Several VMs running
- **WHEN** the VMs `a` and `b` are running and the user runs `idiot vm install --name b ./app.snap`
- **THEN** the snap is installed in `b` only

## ADDED Requirements

### Requirement: Installing snaps from the store
For each store snap given to `idiot vm install`, the command SHALL
download, on the host, the revision released in the snap's channel (its
`=<channel>`, else `--channel <channel>`, else `stable`; a channel without
a track means `latest/`) for the VM's recorded architecture, from the
active store (`UBUNTU_STORE_ID`, else the global store).
`UBUNTU_STORE_ARCH` SHALL NOT affect it. The download SHALL use the active
credential, or the saved credential `--cred <name>` names for that call
only, and SHALL be anonymous when there is neither, saying so. A
credential SHALL be sent only to the store, never to the VM or to the
download server the store redirects to, and SHALL never be printed,
logged or passed on a command line. Each downloaded snap SHALL match the
size and sha3-384 digest the store publishes for that revision, or the
command SHALL fail and keep nothing of it. Downloads SHALL be kept in
idiot's cache directory, readable only by the user. A cached revision
SHALL be reused, without downloading it again, when its digest still
matches the store's. Unless `--dangerous` is given, the command SHALL
fetch from the store the assertions that let snapd install that revision
asserted: its snap-revision and snap-declaration, and the account-keys
and accounts they need. It SHALL write them next to the snap, each after the
assertions it needs. Before anything is copied into the guest, it SHALL
fail when:
- a snap name or channel is invalid;
- the store doesn't show the snap to the credential;
- the channel has no release for the VM's architecture (listing the
  channels that have one);
- the store refuses the download.

Each of these failures SHALL suggest `idiot store check <snap>` with the
same architecture and channel. Download progress SHALL go to stderr.

#### Scenario: Public snap, anonymous
- **WHEN** no credential is active, the VM `a` is `amd64`, and the user runs `idiot vm install hello-world`
- **THEN** stderr says the download is anonymous, `hello-world`'s `latest/stable` amd64 revision is downloaded and verified, and it is installed asserted as that revision

#### Scenario: The VM's architecture, not the target's
- **WHEN** `UBUNTU_STORE_ARCH` is `amd64`, the VM `pi` was created with `--arch arm64`, and the user runs `idiot vm install --name pi hello-world`
- **THEN** the arm64 revision is downloaded and installed

#### Scenario: Private snap in a brand store
- **WHEN** store `ce-team-test` is active, the active credential can download the private snap `ce-dummy`, and the user runs `idiot vm install ce-dummy=edge`
- **THEN** `ce-dummy`'s `latest/edge` revision is downloaded with the credential and installed asserted, and nothing in the guest holds the credential

#### Scenario: Another credential for one call
- **WHEN** credential `work` is active and the user runs `idiot vm install --cred ce-admin ce-dummy`
- **THEN** the download uses `ce-admin`, and afterwards `work` is still the active credential

#### Scenario: Cached revision reused
- **WHEN** `hello-world` revision 29 was downloaded by an earlier install and `latest/stable` still points at it
- **THEN** a second `idiot vm install hello-world` doesn't download the snap again

#### Scenario: Corrupted download or cache
- **WHEN** a downloaded or cached file's sha3-384 differs from the store's
- **THEN** a cached file is downloaded again, and a fresh download that doesn't match fails the command with nothing kept and nothing copied into the guest

#### Scenario: No release for the VM's architecture
- **WHEN** the VM is `arm64` and `my-tool` has no arm64 revision in `latest/stable`
- **THEN** the command fails before copying anything, lists the channels that have an arm64 revision, and suggests `idiot store check my-tool --arch arm64 --channel stable`

#### Scenario: Snap the store doesn't show
- **WHEN** the store doesn't show `nope` to the credential
- **THEN** the command fails before copying anything and suggests `idiot store check nope` with the VM's architecture and the channel

#### Scenario: Invalid channel
- **WHEN** the user runs `idiot vm install hello-world=`
- **THEN** the command fails naming the empty channel, and contacts nothing

#### Scenario: Explicitly dangerous
- **WHEN** the user runs `idiot vm install --dangerous hello-world`
- **THEN** the snap is downloaded and verified, no assertions are fetched, and it is installed with `--dangerous`
