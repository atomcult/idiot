# Tasks

## 1. Build for all four architectures

- [x] 1.1 snapcraft.yaml: `platforms` for amd64, arm64, armhf and riscv64; the debug part stages ltrace only for amd64, arm64 and armhf; verify with snapcraft's grammar processor that the debug part resolves to the full list for amd64/arm64/armhf and to the list without ltrace for riscv64, and that every part's packages exist in noble for each architecture
- [x] 1.2 `snap/local/check-components`: read `architectures` from the snap's `meta/snap.yaml` and leave ltrace out of the debug expectations on riscv64; verify against the amd64 build (all ok) and against a copy of its index with ltrace removed and the architecture patched to riscv64 (still ok), and amd64 with ltrace removed (fails)
- [x] 1.3 `snapcraft pack` on amd64 still builds `idiot_*_amd64.snap` and all seven components, and check-components passes
- [x] 1.4 README: the architectures, and that riscv64's debug component has no ltrace; run `make check` and commit (`build(snap): build for amd64, arm64, armhf and riscv64`)

## 2. On Lauren's builds (after she runs them)

- [ ] 2.1 For each architecture's build Lauren makes: `snap/local/check-components` passes, and `tests/bare-host --remote <that arch's LXD> <snap>` passes (an LXD on hardware of that architecture, or in an `idiot vm --arch <arch>` VM)
