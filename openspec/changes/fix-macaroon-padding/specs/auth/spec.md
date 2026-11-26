## MODIFIED Requirements

### Requirement: Credential format compatibility
Credentials created by idiot SHALL be in the format `snapcraft export-login`
produces, usable as `SNAPCRAFT_STORE_CREDENTIALS` by snapcraft and as
`UBUNTU_STORE_AUTH` by snapd's image tooling (`snap download`,
`snap prepare-image`, `ubuntu-image`), and credentials exported by
snapcraft SHALL work with every `idiot auth` command. `idiot auth add`
SHALL save the root and discharge macaroons as the store and Ubuntu One
returned them, and no credential idiot writes SHALL contain base64 padding
(`=`) in either macaroon.

#### Scenario: idiot credential used by snapcraft
- **WHEN** a credential made by `idiot auth add` is activated with `idiot auth login`
- **THEN** `snapcraft whoami` reports the same account

#### Scenario: idiot credential used by snapd
- **WHEN** a credential made by `idiot auth add` for an account with access to brand store `ce-team-test` is activated with that store active
- **THEN** `snap download ce-dummy` downloads the snap instead of failing with a 404

#### Scenario: No padding
- **WHEN** `idiot auth add` saves a credential
- **THEN** neither of its macaroons ends in `=`

#### Scenario: snapcraft credential used by idiot
- **WHEN** a file from `snapcraft export-login` is imported with `idiot auth import`
- **THEN** `idiot auth status` for it shows the account details

## ADDED Requirements

### Requirement: Repairing padded credentials
When `idiot auth login`, `idiot profile use` or `idiot auth export` uses a
saved credential whose root or discharge macaroon has base64 padding, it
SHALL first rewrite the saved file without the padding, keeping the file
private, and print on stderr that the credential was repaired. It SHALL
then use the repaired credential: what `auth login` and `profile use`
export, and what `auth export` outputs. The repair SHALL change nothing
else in the credential, SHALL never contact the network, and SHALL never
print the credential or pass it as a command argument. A credential
without padding SHALL be left untouched, with nothing printed. When the
idiot-store helper is missing, the command SHALL use the credential as
saved and warn that it could not be checked.

#### Scenario: Repaired on login
- **WHEN** the saved credential `work` has a padded root macaroon and the user runs `idiot auth login work`
- **THEN** the saved file no longer has the padding, stderr says `work` was repaired, and `UBUNTU_STORE_AUTH` and `SNAPCRAFT_STORE_CREDENTIALS` hold the repaired credential

#### Scenario: Unpadded credential untouched
- **WHEN** the saved credential `ci` has no padding and the user runs `idiot auth login ci`
- **THEN** the file is unchanged and no repair message is printed

#### Scenario: Repaired on export
- **WHEN** the saved credential `work` is padded and the user runs `idiot auth export work -f out.cred`
- **THEN** both the saved file and `out.cred` hold the repaired credential

#### Scenario: Private after repair
- **WHEN** a credential is repaired
- **THEN** its file is still readable and writable only by its owner
