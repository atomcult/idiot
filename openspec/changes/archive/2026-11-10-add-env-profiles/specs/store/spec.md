# Spec Delta

## ADDED Requirements

### Requirement: Warning on a store switch
`idiot store use` and `idiot store unset` SHALL, after queueing their
change, print the auth capability's store mismatch warning when the active
credential is paired with a store other than the one now active. The
switch itself SHALL happen regardless, and the exit status SHALL be zero.

#### Scenario: Switching away from the credential's store
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store use prod`
- **THEN** `UBUNTU_STORE_ID` is `prod`'s ID, the command exits zero, and stderr warns that `ci` belongs to `staging` while `prod` is active

#### Scenario: Switching to the credential's store
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store use staging`
- **THEN** no warning is printed

#### Scenario: Back to global
- **WHEN** credential `ci` is active and paired with store `staging`, and the user runs `idiot store unset`
- **THEN** `UBUNTU_STORE_ID` is unset and stderr warns that `ci` belongs to `staging` while the global store is active
