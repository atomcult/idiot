## MODIFIED Requirements

### Requirement: Assertion cross-checks
For a `model` assertion, `idiot assertion show` SHALL warn on stderr
when:
- its `store` header differs from the active store (`UBUNTU_STORE_ID`),
  or it names a store and no store is active, or a store is active and it
  names none; the warning SHALL name the saved store (`idiot store`) whose
  ID is the model's `store`, when there is one;
- a model with the same `brand-id` and `model` exists in a cloned model
  repository (`idiot model`) with a different revision or different
  content, naming that file and both revisions;
- its `authority-id` differs from its `brand-id`.
When a saved model is identical it SHALL say so on stderr, naming the
file. These checks SHALL NOT change the exit status.

#### Scenario: Wrong store active
- **WHEN** `UBUNTU_STORE_ID` is `acme-store-id` and the user inspects a model whose `store` header is `other-store-id`, saved with `idiot store add other other-store-id`
- **THEN** stderr warns the model's store is `other-store-id` (saved as `other`) while `acme-store-id` is active, and the command still exits zero

#### Scenario: Saved model is newer
- **WHEN** the user inspects revision 2 of a model and the canonical clone holds revision 3 of the same brand and model
- **THEN** stderr warns that the saved model differs, naming its file and revision 3

#### Scenario: Matches a saved model
- **WHEN** the user inspects a file copied with `idiot model pick --signed`
- **THEN** stderr says it matches the saved model, naming the file
