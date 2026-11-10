# Spec Delta

## MODIFIED Requirements

### Requirement: Environment summary
`idiot status` SHALL print on stderr one row each for auth, store, arch and
lp, without contacting the network:
- auth: `logged in` when `UBUNTU_STORE_AUTH` or `SNAPCRAFT_STORE_CREDENTIALS`
  is set, annotated with the active credential name, if cached its account
  email, and, if known, its expiry (`expires in <N>d`, `expires in <N>h`,
  `expires in <1h` or `expired`, highlighted within the expiry warning
  window and when expired); otherwise `not logged in`.
- store: the saved store name whose ID matches `UBUNTU_STORE_ID` (with the
  ID), the bare ID if no saved store matches, or `global`.
- arch: `UBUNTU_STORE_ARCH`, or the host architecture marked `host default`.
- lp: the active Launchpad username, or `not set`.

#### Scenario: Fresh shell
- **WHEN** nothing has been activated
- **THEN** the rows read `not logged in`, `global`, the host arch marked `host default`, and `not set` (unless a username was saved)

#### Scenario: Credential expiry
- **WHEN** the active credential `work` expires in 41 days
- **THEN** the auth row's annotation includes `work` and `expires in 41d`

#### Scenario: Expired credential
- **WHEN** the active credential has expired
- **THEN** the auth row's annotation includes `expired`, and nothing contacts the store

#### Scenario: Named store
- **WHEN** `UBUNTU_STORE_ID` matches the saved store `staging`
- **THEN** the store row shows `staging` and its ID

#### Scenario: Unknown store ID
- **WHEN** `UBUNTU_STORE_ID` matches no saved store
- **THEN** the store row shows the raw ID
