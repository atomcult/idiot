# command-layout Specification

## Purpose

How idiot's commands are named and grouped so a user can guess them: one
verb per kind of action across groups, one group per subject, explicit
forms for overloaded commands, and old names that keep working after a
rename.

## Requirements

### Requirement: Verbs for the active setting
A group that keeps an active setting in the shell SHALL name its commands
by what they do to it: `use` activates something saved by name (`auth use`,
`store use`, `profile use`), `set` takes a free value (`arch set`,
`lp set`), `unset` clears the active setting, and `status` shows it. Every
such group (`auth`, `store`, `profile`, `arch`, `lp`) SHALL have `unset`
and `status`.

#### Scenario: Same verbs everywhere
- **WHEN** the user runs `idiot auth help`, `idiot store help` and `idiot profile help`
- **THEN** each lists `use`, `unset` and `status`

#### Scenario: Free values
- **WHEN** the user runs `idiot arch help` and `idiot lp help`
- **THEN** each lists `set`, `unset` and `status`

### Requirement: Explicit forms for overloaded commands
`idiot vm boot [options] [<image>] [-- <qemu-args>]` SHALL boot a VM exactly
as `idiot vm [options] [<image>] [-- <qemu-args>]` does, and
`idiot example clone <repo> [options]` SHALL clone exactly as
`idiot example <repo> [options]` does. The bare forms SHALL keep working,
without a note. Help for `vm` and `example` SHALL show the explicit form
first and list the bare one as a shortcut.

#### Scenario: Boot
- **WHEN** the user runs `idiot vm boot --name a core.img`
- **THEN** the VM `a` boots, as `idiot vm --name a core.img` would

#### Scenario: Clone
- **WHEN** the user runs `idiot example clone pc-gadget --bare`
- **THEN** the files of `pc-gadget` are extracted, as `idiot example pc-gadget --bare` would

#### Scenario: Shortcut kept
- **WHEN** the user runs `idiot vm core.img`
- **THEN** the VM boots and stderr has no rename note

### Requirement: Old command names
Each renamed command SHALL keep running under its old name:

| Old | New |
|---|---|
| `store assertion` | `assertion fetch` |
| `inspect assertion` | `assertion show` |
| `assertion pick` | `model pick` |
| `assertion repos list`, `add`, `remove`, `update` | `model list`, `add`, `remove`, `update` |
| `assertion repos` | `model` |
| `auth login` | `auth use` |
| `auth logout` | `auth unset` |
| `vm console-log [<name>]` | `vm logs --console [--name <name>]` |
| `clean all` | `clean` |
| `store roles` | `store list --account` |

An old name SHALL print one line on stderr naming the new form, then behave
exactly as the new command with the same arguments: the same stdout, eval
file contents, exit status and host handoff. Old names SHALL NOT be listed
in any help, and SHALL be counted as the new command (status capability,
usage level). Asking an old name for help SHALL show the new command's
help after the note.

#### Scenario: Alias with a note
- **WHEN** the user runs `idiot auth login work`
- **THEN** stderr says `auth login` is now `idiot auth use`, and `work` is activated exactly as `idiot auth use work` activates it

#### Scenario: Stdout unchanged
- **WHEN** the user runs `idiot assertion pick --signed -o - >pc.model` and selects a model
- **THEN** `pc.model` holds only the assertion, and the note is on stderr

#### Scenario: Group alias
- **WHEN** the user runs `idiot assertion repos update`
- **THEN** stderr notes it is now `idiot model update`, and every repository is updated

#### Scenario: Hidden from help
- **WHEN** the user runs `idiot help`, `idiot assertion help`, `idiot store help` or `idiot auth help`
- **THEN** none of `assertion pick`, `assertion repos`, `store assertion`, `store roles`, `auth login` or `auth logout` is listed

### Requirement: Main help
`idiot help` SHALL list every top-level command under a heading that
describes it:
- **Store & environment**: `status`, `profile`, `auth`, `store`, `arch`, `lp`;
- **Assertions & inspection**: `assertion`, `model`, `inspect`;
- **VM & device**: `vm`;
- **Get sources**: `kernel`, `example`;
- **Setup & tools**: `init`, `prompt`, `run`, `clean`, `doctor`, `version`.

#### Scenario: Every command listed
- **WHEN** the user runs `idiot help`
- **THEN** every command under `cmd/` (and `version`) appears under exactly one heading, `model` among them

### Requirement: Assertion and model groups
Every command that fetches, shows, writes, checks or signs an assertion
SHALL be under `idiot assertion`: `fetch`, `show`, `new`, `edit`, `check`
and `sign`. The model repositories and picking a model from them SHALL be
their own group, `idiot model`: `list`, `add`, `remove`, `update` and
`pick`. `idiot assertion help` SHALL point at `idiot model` for models
kept in repositories.

#### Scenario: Assertion help
- **WHEN** the user runs `idiot assertion help`
- **THEN** it lists `fetch`, `show`, `new`, `edit`, `check` and `sign`, and names `idiot model` for the model repositories

#### Scenario: Model help
- **WHEN** the user runs `idiot model help`
- **THEN** it lists `list`, `add`, `remove`, `update` and `pick` and the configured repositories
