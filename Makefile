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
SHELL_FILES := idiot $(SHARE_FILES) snap/local/patchelf-host snap/local/pull-tracked snap/local/component-build snap/local/vm-build snap/local/check-components tests/bare-host $(shell find cmd tests/bare-host.d -type f ! -name '*.model')

# Rust binaries installed to lib/idiot/libexec: idiot-store (the store
# client) and idiot-helper (parsing, hashing and sockets)
HELPERS     = idiot-store idiot-helper
CARGO      ?= cargo

LOG := printf '  \033[1;36m%-12s\033[0m %s\n'

.PHONY: check fmt fmt-check hooks install install-sh install-helper helper uninstall purge

hooks:
	@$(LOG) "GIT" "core.hooksPath = .githooks"
	@git config core.hooksPath .githooks

check: fmt-check
	@$(LOG) "SHELLCHECK" "idiot share/*.sh snap/local/* cmd/**"
	@rc=0; \
	shellcheck idiot $(SHARE_FILES) snap/local/patchelf-host snap/local/pull-tracked snap/local/component-build snap/local/vm-build snap/local/check-components tests/bare-host || rc=1; \
	find tests/bare-host.d -type f ! -name '*.model' | xargs shellcheck -x || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc
	@for h in $(HELPERS); do \
	    $(LOG) "CLIPPY" "tools/$$h"; \
	    $(CARGO) clippy --quiet --locked --all-targets --manifest-path "tools/$$h/Cargo.toml" -- -D warnings || exit 1; \
	done

fmt:
	@$(LOG) "SHFMT" "(writing)"
	@shfmt -w $(SHELL_FILES)
	@for h in $(HELPERS); do \
	    $(LOG) "CARGO FMT" "tools/$$h"; \
	    $(CARGO) fmt --manifest-path "tools/$$h/Cargo.toml" || exit 1; \
	done

fmt-check:
	@$(LOG) "SHFMT" "(checking)"
	@shfmt -d $(SHELL_FILES) || { printf '  \033[1;33m%-12s\033[0m %s\n' "HINT" "run: make fmt"; exit 1; }
	@for h in $(HELPERS); do \
	    $(CARGO) fmt --check --manifest-path "tools/$$h/Cargo.toml" || { printf '  \033[1;33m%-12s\033[0m %s\n' "HINT" "run: make fmt"; exit 1; }; \
	done

# the shell tree and the Rust helpers; the snap builds each helper in its
# own part and runs install-sh only
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
	    printf '  \033[1;31m%-12s\033[0m %s\n' "ERROR" "cargo not found: idiot-store and idiot-helper need a Rust toolchain"; exit 1; }
	@for h in $(HELPERS); do \
	    $(LOG) "CARGO" "build --release tools/$$h"; \
	    $(CARGO) build --quiet --release --locked --manifest-path "tools/$$h/Cargo.toml" || exit 1; \
	done

install-helper: helper
	@install -d '$(LIBDIR)/libexec'
	@for h in $(HELPERS); do \
	    $(LOG) "INSTALL" "$(LIBDIR)/libexec/$$h"; \
	    install -m755 "tools/$$h/target/release/$$h" '$(LIBDIR)/libexec/'"$$h" || exit 1; \
	done

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
