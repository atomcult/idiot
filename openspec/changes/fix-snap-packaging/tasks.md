# Tasks

## 1. Stable entry point

- [x] 1.1 Make the Makefile launcher export `IDIOT_LAUNCHER`, and make `cmd/init` emit `/snap/bin/${SNAP_INSTANCE_NAME}` under a snap, else `IDIOT_LAUNCHER`, else the computed path; verify `make install PREFIX=$(mktemp -d)` then `<prefix>/bin/idiot init bash` embeds `<prefix>/bin/idiot`, and `SNAP=/snap/idiot/x9 SNAP_INSTANCE_NAME=idiot ./idiot init fish` embeds `/snap/bin/idiot`
- [x] 1.2 Run `make check` and commit (`fix(init): embed a stable command path`)

## 2. Snap

- [ ] 2.1 Ask Lauren before removing the old `idiot` snap (`x1`) from the dev host, since it may hold v1 data
- [ ] 2.2 Switch `snap/snapcraft.yaml` to `confinement: classic`; set the `core` component to `type: standard` and organise the `core-packages` part into it; verify `snapcraft pack` produces `idiot_*.snap` and `idiot+core_*.comp`
- [ ] 2.3 Install locally (`snap install --dangerous --classic idiot_*.snap`, then the component) and verify: `idiot status` runs through the wrapper; `idiot store use` uses the snap's fzf with host fzf hidden from PATH (add `environment: PATH` if not); `idiot run debugfs -V` names `idiot+core` without the component and works with it (fix `snap_app` lookup if needed)
- [ ] 2.4 Refresh to a second locally built revision, remove the first, and verify an already-open shell still runs `idiot status`
- [ ] 2.5 Update README install instructions (`--classic`, `idiot+core`); run `make check` and commit (`build(snap): classic confinement and populated core component`)
