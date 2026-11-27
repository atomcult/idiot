## MODIFIED Requirements

### Requirement: Confirmed removal
`idiot clean [containers|snaps] [-n] [-y]` SHALL remove LXD containers in
snapcraft's project (`containers`), sideloaded snaps (`snaps`), or, with
neither, both. It SHALL list everything it is about to remove and ask for
confirmation on the terminal before removing anything; only an explicit
yes SHALL proceed. `-y`/`--yes` SHALL skip the question. `-n`/`--dry-run`
SHALL list without asking or removing. When there is nothing to remove it
SHALL say so and exit zero. An unknown word SHALL fail naming it, and
`idiot clean help` SHALL show the usage.

#### Scenario: Confirmation declined
- **WHEN** the user runs `idiot clean snaps` and answers anything but yes
- **THEN** nothing is removed and the exit status is zero

#### Scenario: Non-interactive
- **WHEN** the user runs `idiot clean --yes`
- **THEN** the containers and sideloaded snaps are removed without a question

#### Scenario: Dry run
- **WHEN** the user runs `idiot clean containers --dry-run`
- **THEN** the containers are listed and none is deleted

#### Scenario: Old name
- **WHEN** the user runs `idiot clean all -n`
- **THEN** stderr notes that it is now `idiot clean`, and the containers and sideloaded snaps are listed and none is removed
