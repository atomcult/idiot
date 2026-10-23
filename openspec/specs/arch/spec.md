# arch Specification

## Purpose

Choosing the target build/store architecture for the current shell via
`UBUNTU_STORE_ARCH`, defaulting to the host's architecture.

## Requirements

### Requirement: Setting the architecture
`idiot arch set [<arch>]` SHALL emit an export of `UBUNTU_STORE_ARCH` for one
of amd64, arm64, armhf, i386, ppc64el, riscv64 or s390x, offering an fzf
picker of those when no argument is given (cancelling changes nothing). Any
other value SHALL be rejected with the list of valid ones. `idiot arch unset`
SHALL emit an unset of `UBUNTU_STORE_ARCH`.

#### Scenario: Valid architecture
- **WHEN** the user runs `idiot arch set arm64`
- **THEN** `UBUNTU_STORE_ARCH` is `arm64` in the current shell

#### Scenario: Invalid architecture
- **WHEN** the user runs `idiot arch set x86`
- **THEN** the command fails listing the valid architectures and nothing changes

#### Scenario: Unset
- **WHEN** the user runs `idiot arch unset`
- **THEN** `UBUNTU_STORE_ARCH` is unset

### Requirement: Showing the architecture
`idiot arch status` SHALL print `UBUNTU_STORE_ARCH` when set, and otherwise
the host's architecture in snap naming (e.g. `x86_64` shown as `amd64`).

#### Scenario: Host default
- **WHEN** `UBUNTU_STORE_ARCH` is unset on an x86_64 host
- **THEN** `idiot arch status` prints `amd64`
