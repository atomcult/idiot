.POSIX:

PREFIX    ?= $(HOME)/.local
BINDIR     = $(PREFIX)/bin
LIBDIR     = $(PREFIX)/lib/idiot
DATA_HOME ?= $(HOME)/.local/share
CACHE_HOME ?= $(HOME)/.cache
USER_DIR   = $(DATA_HOME)/idiot
CACHE_DIR  = $(CACHE_HOME)/idiot

VERSION  := $(shell git describe --tags --always --dirty 2>/dev/null || printf 'unknown')

SHELL_FILES := idiot share/common.sh share/fzf.sh share/arch.sh share/creds.sh share/stores.sh share/lp.sh share/models.sh share/examples.sh $(shell find cmd -type f)

LOG := printf '  \033[1;36m%-12s\033[0m %s\n'

.PHONY: check fmt fmt-check install uninstall purge

check: fmt-check
	@$(LOG) "SHELLCHECK" "idiot share/*.sh cmd/**"
	@rc=0; \
	shellcheck idiot share/common.sh share/fzf.sh share/arch.sh share/creds.sh share/stores.sh share/lp.sh share/models.sh share/examples.sh || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc

fmt:
	@$(LOG) "SHFMT" "(writing)"
	@shfmt -w $(SHELL_FILES)

fmt-check:
	@$(LOG) "SHFMT" "(checking)"
	@shfmt -d $(SHELL_FILES) || { printf '  \033[1;33m%-12s\033[0m %s\n' "HINT" "run: make fmt"; exit 1; }

install:
	@$(LOG) "MKDIR" "$(LIBDIR)"
	@install -d '$(LIBDIR)' '$(BINDIR)'
	@$(LOG) "COPY" "cmd lib -> $(LIBDIR)/"
	@rm -rf '$(LIBDIR)/cmd' '$(LIBDIR)/share'
	@cp -rp cmd share '$(LIBDIR)/'
	@$(LOG) "VERSION" "$(VERSION)"
	@printf '%s\n' '$(VERSION)' > '$(LIBDIR)/share/version'
	@$(LOG) "INSTALL" "$(LIBDIR)/idiot"
	@install -m755 idiot '$(LIBDIR)/idiot'
	@$(LOG) "WRITE" "$(BINDIR)/idiot"
	@printf '#!/usr/bin/env sh\n: "$${IDIOT_ROOT:=%s}"\nexec "$${IDIOT_ROOT}/idiot" "$$@"\n' \
	    '$(LIBDIR)' > '$(BINDIR)/idiot'
	@chmod 755 '$(BINDIR)/idiot'

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
