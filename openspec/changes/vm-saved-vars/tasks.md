# Tasks

## 1. Verify the implementation against the requirement

- [ ] 1.1 Boot amd64 and arm64 VMs with `--state-dir` (host qemu, from a source checkout), then check every scenario in the change's vm spec: certificate subjects for PK/KEK/db, no host certificates (compare with `efi-readvar` on the host), `| grep KEK`, the missing-`fw_vars.fd` error, and a host PATH without virt-fw-vars or efi-readvar
- [ ] 1.2 Run `make check` and commit (`docs(openspec): specify idiot vm vars`)
