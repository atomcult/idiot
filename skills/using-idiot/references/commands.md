# idiot command reference

Every command takes `--help` (or `help`) and prints usage on stderr.
`idiot <group> --help` lists a group's subcommands. Check `--help` for
options not listed here before guessing.

## Contents

- [Environment: status, auth, store, arch, lp, profile](#environment)
- [VMs](#vms)
- [Inspecting](#inspecting)
- [Writing assertions](#writing-assertions)
- [Tools and housekeeping](#tools-and-housekeeping)
- [Development checkouts](#development-checkouts)
- [Environment variables](#environment-variables)

## Environment

These change the current shell, so they need the shell integration loaded
in the same shell (see SKILL.md). They run confined under the snap.

| Command | Does |
|---|---|
| `idiot status` | Active store, credential (and expiry), arch, lp user, profile |
| `idiot prompt [--format store,arch,cred,lp,profile] [--separator s]` | The same as one segment on stdout |
| `idiot auth list` / `status` | Saved credentials with expiry and store; the active one's login status (`status` asks the store) |
| `idiot auth add <name> [--permissions …] [--snaps …] [--channels …] [--expires …]` | Logs in (interactive: password and OTP) and saves a credential paired with the active store (`--any`: for any store). Doesn't activate it. **Leave to the user.** |
| `idiot auth login <name>` / `logout` | Activate / clear a credential (sets `SNAPCRAFT_STORE_CREDENTIALS`, `UBUNTU_STORE_AUTH`, `IDIOT_ACTIVE_CRED`) |
| `idiot auth import <name> <file>` | Save a credential exported by snapcraft |
| `idiot auth export [<name>] [-f <file>]` | Credential on stdout, or to a file. **Never let it reach the transcript.** |
| `idiot auth bind [--any] [<name>]` | Re-pair a credential with the active store; `--any`: for any store, never warned about |
| `idiot auth remove <name>` | Delete a saved credential (confirm first) |
| `idiot store add <name> <id>` / `use <name>` / `unset` / `list` / `remove` | Saved stores; `use` sets `UBUNTU_STORE_ID` and `IDIOT_ACTIVE_STORE` |
| `idiot store check <snap> [--channel c] [--arch a] [--cred n]` | Why a snap can (exit 0) or can't (exit 1) be downloaded from the active store: membership, release, credential (expiry, permissions, padding). Downloads nothing. Host command |
| `idiot store channels <snap> [--arch a] [--tsv]` | The snap's channel map (stdout) |
| `idiot store import [--all] [--cred n]` | Save the stores the account belongs to (skips global and saved ones). Interactive picker and name prompts; `--all` uses names from the display names. Host command |
| `idiot store assertion [--chain] [--raw \| -o f] <type> <key>...` | Fetch an assertion (account, account-key, store, model, snap-declaration `<snap>`, snap-revision `<snap> <rev>`, validation-set `<acct>/<name>`, or any type by `header=value`) and summarise it; `--chain` adds what snapd needs to accept it. Host command |
| `idiot store roles [--tsv]` | Your stores and roles (stdout), the active one marked `*` |
| `idiot store snaps [--tsv \| --pick] [--cred n]` | Snaps in the active store (stdout); the full list only for admins with a `store_admin` credential |
| `idiot store browse [<snap>…] [filters] [--tsv [--revisions]] [--cred n]` | Explore the active store in fzf (needs a terminal); for agents always `--tsv`: one row per snap/channel/arch (snap, channel, arch, revision, version, released, type, base, confinement, publisher, visibility), or per revision with `--revisions` (snap, revision, version, arches, status, uploaded, channels or `-`, base, confinement, grade). Filters: `--arch`, `--channel` (`edge`, `2.0/*`, `*/stable`), `--version` glob, `--revision`, `--type`, `--base`, `--confinement`, `--since YYYY-MM-DD`, `--status`/`--unreleased` (revisions), `--name` glob, `--publisher`, `--private`/`--public`. Host command |
| `idiot store members [--role r] [--email] [--tsv] [--cred n]` | The active brand store's members: username, roles, display name (stdout); admins with a `store_admin` credential only. Emails only with `--email`: other people's data, keep them out of the transcript unless needed. Host command |
| `idiot arch set <arch>` / `unset` / `status` | Sets `UBUNTU_STORE_ARCH` (amd64, arm64, armhf, riscv64, …) |
| `idiot lp set <user>` / `unset` / `status` | Launchpad username (also the VM guest user for `vm ssh`) |
| `idiot profile save <name> [--fields auth,store,arch,lp]` | Record the current setup (names only, never credential contents) |
| `idiot profile use <name>` / `list` / `status` / `remove` | Switch everything at once; checks everything before changing anything |

Commands whose name argument is optional (`auth login`, `auth remove`,
`auth export`, `profile use`, `profile remove`) open an fzf picker without
one: always pass the name.

`auth login`, `profile use` and `auth export` repair a credential with
base64 padding (saved by older idiot), which snapd can't read: it then
downloads anonymously and private or brand-store snaps 404. If snap
downloads 404 with an idiot credential, run `idiot doctor`.

The active credential and store are warned about when they don't belong
together; the warning says how to fix it and never blocks.

## VMs

| Command | Does |
|---|---|
| `idiot vm [--name n] [--arch a] [--tpm] [--no-sb] [--in-place] [--ssh-port p] [--port h:g] [--state-dir d] <image> [-- qemu-args]` | Create and boot a VM (foreground, serial console; Ctrl-a x quits) |
| `idiot vm --name n` | Boot an existing VM again |
| `idiot vm list` | Name, arch, state, ssh port, image |
| `idiot vm ssh --name n [ssh-args]` / `scp --name n src dst` | As the lp user, on the VM's forwarded port (`:path` is the guest side for scp) |
| `idiot vm install --name n [--channel c] [--cred name] <snap>...` | Copy in and install in one change, following reboots; a `<snap>` is a `.snap` file (`<file>.assert` beside every one makes them asserted, else `--dangerous`) or a store snap `name[=channel]`, downloaded on the host for the VM's arch with the active credential, verified, cached, with its assertions |
| `idiot vm logs --name n [snap[.app]] [-f] [-n lines]` | The guest journal on stdout |
| `idiot vm stop [--timeout s] [--force] <n>` | Power button, then quit after the timeout |
| `idiot vm console-log [-f] [--previous] <n>` | The serial log |
| `idiot vm vars --name n` (or a state directory) | UEFI secure boot variables (PK, KEK, db, dbx) |
| `idiot vm tpm --name n` | swtpm on the stopped VM's TPM state for tpm2-tools |
| `idiot vm snapshot create [--name n] <snapshot>` | Save a stopped VM's disk, UEFI vars and TPM state under a name (not `--in-place` VMs) |
| `idiot vm snapshot list [--name n]` | Its snapshots on stdout: name, taken, UEFI, TPM, ok/broken (works while running) |
| `idiot vm snapshot restore [--yes] [--name n] <snapshot>` | Put the stopped VM back to it (the snapshot is kept; confirm first) |
| `idiot vm snapshot delete [--yes] [--name n] <snapshot>` | Remove every part of it, broken ones too (confirm first) |
| `idiot vm reset [--yes] <n>` | Discard disk overlay, UEFI vars, TPM state and snapshots (confirms when it has snapshots) |
| `idiot vm delete [--yes] <n>` | Forget the VM; the image is untouched (confirm first) |

The image is never written unless `--in-place`: the guest writes a qcow2
overlay in the VM's state directory. The image, `--arch`, `--sb` and
`--in-place` are fixed when the VM is created. The first VM gets ssh port
8022, later ones the next free port.

## Inspecting

Host commands. Image inspection needs the `core` component (or the host's
e2fsprogs, mtools and fdisk).

| Command | Does |
|---|---|
| `idiot inspect image <disk.img>` | Model, first boot, seeded snaps, snapd state |
| `idiot inspect diff [--tsv] [--state] <a.img> <b.img>` | Only what differs |
| `idiot inspect state [--view <v>] [--json \| -o <file> [--force]] <file \| - \| --name <vm> \| --local>` | fzf explorer of snapd's state.json (snaps and kept revisions, connections, changes and task logs, raw tree) from a state file, a raw or qcow2 Core image, stdin, an idiot vm VM (ssh while running, its disk when stopped) or this machine (sudo). Secrets are always redacted; `--json`/`-o` export the redacted state. `inspect image --extract-state` saves it redacted too |
| `idiot inspect assertion [--remote] <file \| ->` | Headers, snaps, signature chain; exit 1 on an invalid signature. Offline for Canonical models; never touches snapd's database |
| `idiot inspect snap <file.snap>` | Its snapcraft.yaml or snap.yaml |
| `idiot inspect changes` | Browse recent snapd changes (fzf) |
| `idiot model list` / `add` / `remove` / `pick` / `update` | Model assertion repos; `pick` is an fzf picker; `update` needs git |

## Writing assertions

Host commands. Drafts are the unsigned JSON `snap sign` takes (every
scalar a string); snap IDs, types and revisions come from the active
store, as the credential sees it.

| Command | Does |
|---|---|
| `idiot assertion new <model\|system-user\|validation-set> [--set h=v]... [--add h=v]... [--snap name[=channel]]... [--arch a] [--cred c] [--no-edit] [-o file\|-]` | Asks for each header (defaults: the credential's account, the arch, the active store, UC24 essentials), fills each snap's ID/type/channel or revision, opens $EDITOR, checks, writes `<model>.json` / `<username>.json` / `<name>-<seq>.json` (never overwrites). Without a terminal every required header must be given |
| `idiot assertion edit <file.json> [--refresh] [--snap name[=channel]]... [--set …] [--no-edit] [-o file\|-]` | Same prompts, the file's values as defaults; bumps a model's revision (a validation set's revision or sequence), moves the timestamp on; `--refresh` looks snaps up again; keeps fields it doesn't model; refuses signed input |
| `idiot assertion check <file.json\|->` | snapd's rules for the three types, offline; exit 1 on an error |
| `idiot assertion sign <file.json> [-k key] [--chain] [-o file\|-]` | Checks, warns when the key isn't registered to the `authority-id` (store lookup), runs `snap sign -k <key>` on the terminal (it asks the passphrase; idiot never reads the key), writes `<name>.model`/`.assert` beside the draft (0600 for system-users), reports the signature status. Without `-k` and a terminal: snap's `default` key |

## Tools and housekeeping

| Command | Does |
|---|---|
| `idiot run [--sudo] <tool> [args]` | A bundled tool, else the host's |
| `idiot run --list` / `--shell` | Components and their tools / a shell with them on PATH |
| `idiot doctor [--quiet]` | Read-only, offline health check; exit 1 on `✗` |
| `idiot clean containers` / `snaps` / `all` `[-n] [--yes]` | Remove snapcraft build containers / unasserted snaps; `-n` lists what would go (confirm first) |
| `idiot version [--short]` | Version |

## Development checkouts

Need git: Classic only.

| Command | Does |
|---|---|
| `idiot kernel [<series>] [<flavour>] [--branch b] [--dest d]` | Clone an Ubuntu kernel from Launchpad (no shallow clones: large) |
| `idiot example <repo> [--dest d] [--branch b] [--tag t] [--subdir p] [--bare]` | Clone an example repo (cached mirror); `list` / `add` / `remove` / `update` |

Options documented as "omit value for fzf picker" open a picker when given
without a value: always pass the value in a non-interactive shell.

## Environment variables

| Variable | Set by / for |
|---|---|
| `SNAPCRAFT_STORE_CREDENTIALS`, `UBUNTU_STORE_AUTH`, `IDIOT_ACTIVE_CRED` | `auth login`: what snapcraft and snap use |
| `UBUNTU_STORE_ID`, `IDIOT_ACTIVE_STORE` | `store use` |
| `UBUNTU_STORE_ARCH` | `arch set` |
| `IDIOT_LP_USER` | `lp`, `profile use` (this shell only) |
| `IDIOT_PROMPT` | Kept current by the wrapper for prompts |
| `IDIOT_CRED_WARN_DAYS` | Days before expiry to warn (default 7) |
| `IDIOT_DATA_HOME`, `IDIOT_CACHE_DIR` | State directories (set by `init` under the snap) |
| `NO_COLOR` | Disable colour |
