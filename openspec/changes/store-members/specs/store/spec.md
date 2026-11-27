## ADDED Requirements

### Requirement: Listing a store's members
`idiot store members` SHALL print on stdout the members of the active store,
one per line sorted by username: username, roles (comma-separated) and
display name. `--role <role>` SHALL keep only members with that role,
`--email` SHALL add each member's email, and `--tsv` SHALL print the same
fields tab-separated (username, roles, display name, then email with
`--email`). Emails SHALL NOT be printed without `--email`. The command SHALL
first check that the account is an admin of the active store and that the
credential has `store_admin`, and fail otherwise, saying which is missing.
For a missing permission it SHALL name the saved credentials that have it,
or say how to create one. With the global store active it SHALL fail
saying the global store has no member list. It SHALL follow the credential
rules for store queries (`--cred`, never printing a credential) and change
nothing in the store.

#### Scenario: Members listed
- **WHEN** the account is an admin of the active store and the credential has `store_admin`
- **THEN** stdout lists each member's username, roles and display name, and no email addresses

#### Scenario: Only one role
- **WHEN** the user runs `idiot store members --role admin`
- **THEN** only members with the `admin` role are listed

#### Scenario: Emails on request
- **WHEN** the user runs `idiot store members --email --tsv`
- **THEN** each line is username, roles, display name and email, tab-separated

#### Scenario: Not an admin
- **WHEN** the account has only the `view` role in the active store
- **THEN** the command fails saying the member list needs an admin of the store, and nothing is printed on stdout

#### Scenario: Missing store_admin
- **WHEN** the account is an admin but the credential lacks `store_admin`, and saved credential `ce-admin` has it
- **THEN** the command fails suggesting `idiot store members --cred ce-admin`

#### Scenario: Global store
- **WHEN** no store is active
- **THEN** the command fails saying the global store has no member list
