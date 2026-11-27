---
name: using-idiot
description: >
  Operates idiot, the shell environment manager and introspection tool for
  Snap Store and Ubuntu Core development, on both Ubuntu Classic hosts and
  Ubuntu Core devices. Covers installing it (source or snap, plus its optional
  tool components), loading the shell integration in agent shells, switching
  credentials, stores, architectures and profiles, booting and driving QEMU
  VMs, inspecting Core images and assertions, running bundled tools, and
  diagnosing problems with idiot doctor, including what works differently or
  not at all on Ubuntu Core.
  WHEN: use idiot, run idiot, install idiot, idiot on Ubuntu Core, idiot on a
  Core device, idiot snap components, idiot init, idiot auth, idiot store use,
  idiot vm, boot a Core image in a VM, idiot inspect image, assertion show,
  idiot run, idiot doctor, idiot profile, switch snap store credentials.
license: MIT
metadata:
  author: Lauren Brock
  version: "1.0.0"
  summary: "Run idiot — credentials, stores, VMs, image inspection and bundled tools — on Ubuntu Classic and Ubuntu Core."
  tags:
    - snap
    - ubuntu-core
    - snap-store
    - qemu
    - shell
---

# Using idiot

## Overview

idiot switches Snap Store credentials, stores, target architectures and
Launchpad identity in the current shell, boots Ubuntu Core images in QEMU,
and inspects images, snaps and assertions. It is a shell function (from
`idiot init`) around a POSIX sh dispatcher. Most of it works the same on
Ubuntu Classic and Ubuntu Core; see
[references/classic-vs-core.md](references/classic-vs-core.md) for the
differences, and [references/commands.md](references/commands.md) for the
command reference.

## Rules

- **Load the shell integration in every shell that runs idiot.** Each agent
  Bash call is a fresh non-interactive shell, and commands that change the
  environment (`auth use`, `store use`, `arch set`, `profile use`) refuse
  to run without the wrapper. Start the command with the init line and chain
  what depends on it in the same call:
  ```bash
  eval "$(idiot init bash)" && idiot store use staging && idiot status
  ```
  Environment changes don't outlive that call. To make them stick, tell the
  user to run the command in their own shell.
- **Read stdout as data only.** Messages, progress and errors go to stderr;
  stdout carries only what was asked for (`auth export`, `vm logs`,
  `run --list`, `inspect diff --tsv`, `prompt`). Pipe stdout, and read
  stderr for diagnostics.
- **Never print credentials.** `idiot auth export` writes a credential to
  stdout. Only run it with `--file <path>` or piped straight into the
  command that needs it, never so its output reaches the transcript.
- **Leave logins to the user.** `idiot auth add` asks for the Ubuntu One
  password and one-time code: ask the user to run it themselves.
- **Confirm before destroying anything.** `clean`, `vm reset`, `vm delete`,
  `auth remove`, `store remove` and `profile remove` delete data. Ask the
  user first, then pass `--yes` where the command needs a terminal to
  confirm on.
- **Start with `idiot doctor`** when something doesn't work. It is read-only
  and offline. `✓` fine, `!` worth knowing, `✗` broken (exit 1). Use
  `idiot doctor --quiet` for only the problems. Each line says what to do.

## Workflow

1. **Find the platform and install.** Check for Ubuntu Core
   (`grep -q '^ID=ubuntu-core' /etc/os-release` or `snap model`) and how
   idiot is installed (`command -v idiot`; `snap list idiot`). On Core, only
   the snap works. Install steps for each are in
   [references/classic-vs-core.md](references/classic-vs-core.md).
2. **Install the components the task needs.** Tools come from optional snap
   components; a missing tool's error names the component to install.
   `idiot run --list` shows what is installed. Installing a component is
   `snap install idiot+<component>` (needs sudo: confirm with the user).
3. **Load the integration** (see Rules) and run `idiot doctor --quiet`.
4. **Do the task** with the commands in
   [references/commands.md](references/commands.md). Check
   [references/classic-vs-core.md](references/classic-vs-core.md) before
   using `kernel`, `example`, `model update`, `clean containers`, zsh/fish,
   or KVM on a Core device.
5. **Report** what changed (`idiot status` for the environment, `idiot vm
   list` for VMs), and anything the user must do in their own shell.

## Tasks

| Task | Command |
|---|---|
| What's active | `idiot status` (store, credential with expiry, arch, lp, profile) |
| Switch credential / store / arch | `idiot auth use <name>`, `idiot store use <name>`, `idiot arch set <arch>` |
| Whole setup at once | `idiot profile save <name>`, `idiot profile use <name>` |
| Boot a Core image | `idiot vm [--name <n>] [--tpm] <image>` (serial console on the terminal) |
| Try a built snap in a VM | `idiot vm install ./app.snap` (or a store snap: `idiot vm install hello-world=edge`), then `idiot vm logs -f app` |
| Save and roll back a VM | `idiot vm snapshot create --name a seeded`, `idiot vm snapshot restore --name a seeded` (VM stopped) |
| What's in an image | `idiot inspect image core.img`, `idiot inspect diff a.img b.img` |
| What's in a store | `idiot store browse --tsv`, `idiot store channels <snap>`, `idiot store check <snap>` |
| Fetch an assertion | `idiot assertion fetch snap-declaration <snap>` (`--chain --raw` for `snap ack`, `-o file`) |
| Fetch what a channel has | `idiot assertion fetch snap-revision <snap> [<channel>]` (default stable, the target arch) |
| Is an assertion valid | `idiot assertion show pc.model` (exit 1 on a bad signature) |
| What snapd did (changes, kept revisions) | `idiot inspect state --name a` (or a state.json, an image, `--local`; secrets always redacted; `--json` for scripts) |
| Write a model or validation set | `idiot assertion new model`, `idiot assertion check kiosk.json` (`--set`/`--snap`/`--no-edit` without a terminal) |
| A bundled tool | `idiot run <tool>`, `idiot run --sudo <tool>`, `idiot run --list` |

Booting a VM (`idiot vm boot <image>`) stays in the foreground on the serial
console until the guest powers off: run it in the background (stdin from
`/dev/null`) or in a terminal the user can see, then drive it with
`idiot vm ssh`, `install`, `logs` and `stop`. Always name the VM
(`--name <n>` for `ssh`, `scp`, `install` and `logs`; a bare `<n>` for
`stop`): with several running, they open a picker.
