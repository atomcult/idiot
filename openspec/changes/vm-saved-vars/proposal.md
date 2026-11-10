# Proposal

## Why

`idiot vm vars <state-dir>` is meant to show a VM's saved UEFI variables,
but nothing specified what it shows, and it showed the wrong machine's:
efi-readvar has no file input, so `efi-readvar <fw_vars.fd` listed the
host's PK/KEK/db as the VM's. Commit 7baf89f fixed it; this change gives
the behaviour a requirement so it can't drift back.

## What Changes

- Specify `idiot vm vars`: it lists the variables in the state directory's
  `fw_vars.fd` (OVMF or AAVMF store), never the host's, and for the
  secure-boot databases the enrolled certificates (by subject) and hashes.
- The listing goes to stdout (it is data), errors to stderr.
- It needs only python3 (no virt-fw-vars or efitools).

stdout: `idiot vm vars` now writes its listing to stdout (it went to
stderr before). Nothing is queued in the eval file. No new runtime
dependency: python3 was already required, and the snap stages nothing new.

## Capabilities

### New Capabilities

### Modified Capabilities
- `vm`: adds the "Saved UEFI variables" requirement.

## Impact

Already implemented in 7baf89f (`cmd/vm` `do_vars`, `share/uefi-vars.py`);
this change records the requirement and its checks. No code changes.
