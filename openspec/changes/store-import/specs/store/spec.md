## ADDED Requirements

### Requirement: Importing the account's stores
`idiot store import [--all] [--cred <name>]` SHALL save, as saved stores,
the stores the account behind the credential belongs to (the active
credential, or the one `--cred` names). It SHALL skip the global store and
any store whose ID is already saved under some name. On a terminal without
`--all` it SHALL offer the remaining stores in a multi-select picker
showing each store's display name, ID and roles, then prompt for each
picked store's name, defaulting to its display name lowercased with each
run of characters other than `a-z` and `0-9` replaced by `-`, and leading
and trailing `-` removed. With `--all` it SHALL save every remaining store
under its default name without asking. Without a terminal and without
`--all` it SHALL fail before contacting the store, saying to pass `--all`.
Names SHALL follow `idiot store add`'s rules. A default name already saved
for another store SHALL get `-` and the ID's first four characters, and a
typed name that is taken or invalid SHALL be asked for again. No saved
store SHALL be overwritten. It SHALL report each store saved, and finish
with a count of stores saved and skipped, all on stderr, and SHALL never
pair credentials with stores.

#### Scenario: Import everything
- **WHEN** the account belongs to the global store, `Example Team Store` (`abc123`) and `Example Brand Store` (`def456`), none saved, and the user runs `idiot store import --all`
- **THEN** stores `example-team-store` (`abc123`) and `example-brand-store` (`def456`) are saved, and the global store isn't

#### Scenario: Already saved
- **WHEN** `abc123` is saved as `team` and the user runs `idiot store import --all`
- **THEN** `abc123` is skipped and `team` is unchanged

#### Scenario: Name taken by another store
- **WHEN** `example-team-store` is saved for `zzz999` and the account's `Example Team Store` has ID `abc123`
- **THEN** `idiot store import --all` saves `abc123` as `example-team-store-abc1` and leaves `zzz999` as it was

#### Scenario: Picking and naming
- **WHEN** on a terminal the user picks `Example Brand Store` and types `brand` at the name prompt
- **THEN** only `brand` (`def456`) is saved

#### Scenario: No terminal
- **WHEN** the user runs `idiot store import` with stdin not a terminal
- **THEN** the command fails saying to pass `--all`, and contacts nothing

#### Scenario: No credential
- **WHEN** no credential is active and no `--cred` is given
- **THEN** the command fails saying to log in first, and saves nothing
