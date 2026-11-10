# Spec Delta

## MODIFIED Requirements

### Requirement: Usage level
Every dispatched command (excluding help, version, status, prompt and init)
SHALL increment an invocation counter in `IDIOT_CACHE_DIR`. The count SHALL
select the logo shown in help and version, and `idiot status` SHALL show an
XP bar with the progress from the current level to the next (or
`max level`).

#### Scenario: Status does not count
- **WHEN** the user runs `idiot status` twice
- **THEN** the invocation count is unchanged

#### Scenario: Prompt does not count
- **WHEN** the user's prompt runs `idiot prompt` on every redraw
- **THEN** the invocation count is unchanged

#### Scenario: Level up
- **WHEN** the count reaches 10
- **THEN** the logo changes from 💀 to 🥚
