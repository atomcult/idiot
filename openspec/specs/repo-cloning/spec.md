# repo-cloning Specification

## Purpose

How idiot clones and fetches git repositories for kernels, example projects
and model assertions, and how it reports the outcome.

## Requirements

### Requirement: Git failures are reported
When a clone, fetch, pull or archive performed by `idiot kernel`,
`idiot example` (including `update`) or `idiot model update` fails, the
command SHALL report the failure on stderr and exit non-zero, and SHALL NOT
print a success message for that repository. When updating several
repositories, a failure in one SHALL be reported as a warning, the rest SHALL
still be processed, and the command SHALL exit non-zero at the end.

#### Scenario: Kernel clone fails
- **WHEN** the user runs `idiot kernel noble` and the clone fails (e.g. network loss)
- **THEN** the command exits non-zero with `clone failed` and does not print `Cloned to`

#### Scenario: Example fetch fails
- **WHEN** the user runs `idiot example pc-gadget` and fetching the mirror fails
- **THEN** the command exits non-zero with `fetch failed` and creates no destination directory

#### Scenario: One of several model repos fails
- **WHEN** the user runs `idiot model update` and one repo cannot be pulled
- **THEN** a warning names that repo, the other repos are still updated, and the exit status is non-zero

### Requirement: Multi-subdirectory extraction stops on failure
When `idiot example <name> --subdir` extracts several selected
subdirectories and one of them is missing or fails to extract, the command
SHALL stop, report which subdirectory failed, and exit non-zero.

#### Scenario: Missing subdirectory
- **WHEN** two subdirectories are selected and the second does not exist at the chosen ref
- **THEN** the command exits non-zero naming the second subdirectory and does not report success
