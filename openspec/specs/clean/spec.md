# clean Specification

## Purpose

Removing development leftovers: LXD containers in snapcraft's `snapcraft`
project and snaps installed from local files (sideloaded, `x<N>` revisions).

## Requirements

### Requirement: Confirmed removal
`idiot clean containers`, `idiot clean snaps` and `idiot clean all` SHALL list
everything they are about to remove and ask for confirmation on the terminal
before removing anything; only an explicit yes SHALL proceed. `-y`/`--yes`
SHALL skip the question. `-n`/`--dry-run` SHALL list without asking or
removing. When there is nothing to remove they SHALL say so and exit zero.

#### Scenario: Confirmation declined
- **WHEN** the user runs `idiot clean snaps` and answers anything but yes
- **THEN** nothing is removed and the exit status is zero

#### Scenario: Non-interactive
- **WHEN** the user runs `idiot clean all --yes`
- **THEN** the containers and sideloaded snaps are removed without a question

#### Scenario: Dry run
- **WHEN** the user runs `idiot clean containers --dry-run`
- **THEN** the containers are listed and none is deleted

### Requirement: Removal errors are visible
If removing a container or snap fails, the command SHALL show the underlying
error, continue with the remaining items, and exit non-zero at the end.

#### Scenario: snap remove needs root
- **WHEN** `snap remove` is refused for lack of privileges
- **THEN** the refusal is shown for that snap, the others are still attempted, and the exit status is non-zero
