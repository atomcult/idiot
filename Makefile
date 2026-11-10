.POSIX:

PREFIX    ?= $(HOME)/.local
BINDIR     = $(PREFIX)/bin
LIBDIR     = $(PREFIX)/lib/idiot
DATA_HOME ?= $(HOME)/.local/share
CACHE_HOME ?= $(HOME)/.cache
USER_DIR   = $(DATA_HOME)/idiot
CACHE_DIR  = $(CACHE_HOME)/idiot

VERSION  := $(shell git describe --tags --always --dirty 2>/dev/null || printf 'unknown')

SHARE_FILES := $(wildcard share/*.sh)
SHELL_FILES := idiot $(SHARE_FILES) snap/local/patchelf-host snap/local/pull-tracked $(shell find cmd -type f)

HELPER_DIR  = tools/idiot-store
HELPER_BIN  = $(HELPER_DIR)/target/release/idiot-store
CARGO      ?= cargo

LOG := printf '  \033[1;36m%-12s\033[0m %s\n'

.PHONY: check fmt fmt-check hooks install install-sh install-helper helper uninstall purge

hooks:
	@$(LOG) "GIT" "core.hooksPath = .githooks"
	@git config core.hooksPath .githooks

check: fmt-check
	@$(LOG) "SHELLCHECK" "idiot share/*.sh snap/local/* cmd/**"
	@rc=0; \
	shellcheck idiot $(SHARE_FILES) snap/local/patchelf-host snap/local/pull-tracked || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc
	@$(LOG) "CLIPPY" "$(HELPER_DIR)"
	@$(CARGO) clippy --quiet --locked --all-targets --manifest-path '$(HELPER_DIR)/Cargo.toml' -- -D warnings

fmt:
	@$(LOG) "SHFMT" "(writing)"
	@shfmt -w $(SHELL_FILES)
	@$(LOG) "CARGO FMT" "$(HELPER_DIR)"
	@$(CARGO) fmt --manifest-path '$(HELPER_DIR)/Cargo.toml'

fmt-check:
	@$(LOG) "SHFMT" "(checking)"
	@shfmt -d $(SHELL_FILES) || { printf '  \033[1;33m%-12s\033[0m %s\n' "HINT" "run: make fmt"; exit 1; }
	@$(CARGO) fmt --check --manifest-path '$(HELPER_DIR)/Cargo.toml' || { printf '  \033[1;33m%-12s\033[0m %s\n' "HINT" "run: make fmt"; exit 1; }

# the shell tree and the store helper; the snap builds the helper in its own
# part and runs install-sh only
install: install-sh install-helper

install-sh:
	@$(LOG) "VERSION" "$(VERSION)"
	@$(LOG) "MKDIR" "$(LIBDIR)"
	@install -d '$(LIBDIR)' '$(BINDIR)'
	@$(LOG) "COPY" "cmd lib -> $(LIBDIR)/"
	@rm -rf '$(LIBDIR)/cmd' '$(LIBDIR)/share'
	@cp -rp cmd share '$(LIBDIR)/'
	@printf '%s\n' '$(VERSION)' > '$(LIBDIR)/share/version'
	@$(LOG) "INSTALL" "$(LIBDIR)/idiot"
	@install -m755 idiot '$(LIBDIR)/idiot'
	@$(LOG) "WRITE" "$(BINDIR)/idiot"
	@printf '#!/usr/bin/env sh\n: "$${IDIOT_ROOT:=%s}"\nexport IDIOT_LAUNCHER="%s"\nexec "$${IDIOT_ROOT}/idiot" "$$@"\n' \
	    '$(LIBDIR)' '$(BINDIR)/idiot' > '$(BINDIR)/idiot'
	@chmod 755 '$(BINDIR)/idiot'

helper:
	@command -v '$(CARGO)' >/dev/null 2>&1 || { \
	    printf '  \033[1;31m%-12s\033[0m %s\n' "ERROR" "cargo not found: idiot-store (used by idiot auth) needs a Rust toolchain"; exit 1; }
	@$(LOG) "CARGO" "build --release $(HELPER_DIR)"
	@$(CARGO) build --quiet --release --locked --manifest-path '$(HELPER_DIR)/Cargo.toml'

install-helper: helper
	@$(LOG) "INSTALL" "$(LIBDIR)/libexec/idiot-store"
	@install -d '$(LIBDIR)/libexec'
	@install -m755 '$(HELPER_BIN)' '$(LIBDIR)/libexec/idiot-store'

uninstall:
	@$(LOG) "RM" "$(LIBDIR)"
	@rm -rf '$(LIBDIR)'
	@$(LOG) "RM" "$(BINDIR)/idiot"
	@rm -f '$(BINDIR)/idiot'

purge: uninstall
	@$(LOG) "RM" "$(USER_DIR)"
	@rm -rf '$(USER_DIR)'
	@$(LOG) "RM" "$(CACHE_DIR)"
	@rm -rf '$(CACHE_DIR)'
