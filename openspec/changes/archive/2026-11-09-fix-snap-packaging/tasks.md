# Tasks

## 1. Stable entry point

- [x] 1.1 Make the Makefile launcher export `IDIOT_LAUNCHER`, and make `cmd/init` emit `/snap/bin/${SNAP_INSTANCE_NAME}` under a snap, else `IDIOT_LAUNCHER`, else the computed path; verify `make install PREFIX=$(mktemp -d)` then `<prefix>/bin/idiot init bash` embeds `<prefix>/bin/idiot`, and `SNAP=/snap/idiot/x9 SNAP_INSTANCE_NAME=idiot ./idiot init fish` embeds `/snap/bin/idiot`
- [x] 1.2 Run `make check` and commit (`fix(init): embed a stable command path`)

## 2. Snap

- [x] 2.1 Ask Lauren before removing the old `idiot` snap (`x1`) from the dev host, since it may hold v1 data (already gone when 2.3 started)
- [x] 2.2 Set the `core` component to `type: standard` and organise the `core-packages` part into it; verify `snapcraft pack` produces `idiot_*.snap` and `idiot+core_*.comp`
- [x] 2.3 Revert to `confinement: strict` with no plugs (keep the rest of 3e2491e); repack and check bundled binaries' interpreter and RPATH with `readelf -l -d`; with Lauren installing (`snap install --dangerous`, then the component), verify: `snap connections idiot` is empty; `idiot store use` uses the snap's fzf with host fzf hidden; `idiot auth list` reads `~/snap/idiot/common`; `idiot run debugfs -V` (host-side) names `idiot+core` without the component and works with it; record the component mount path in design.md and `snap_app`
- [x] 2.4 Refresh to a second locally built revision, remove the first, and verify an already-open shell still runs `idiot status` and a host command (`idiot run fzf --version`)
- [x] 2.5 Update README (snap and component install, separate state and the one-time copy from a source install); run `make check` and commit (`build(snap): strict confinement with no interfaces`)
