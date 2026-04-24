.POSIX:

PREFIX    ?= $(HOME)/.local
BINDIR     = $(PREFIX)/bin
LIBDIR     = $(PREFIX)/lib/idiot
DATA_HOME ?= $(HOME)/.local/share
CACHE_HOME ?= $(HOME)/.cache
USER_DIR   = $(DATA_HOME)/idiot
CACHE_DIR  = $(CACHE_HOME)/idiot

VERSION  := $(shell git describe --tags --always --dirty 2>/dev/null || printf 'unknown')

SHELL_FILES := idiot lib/common.sh $(shell find cmd -type f)

.PHONY: check fmt fmt-check install uninstall purge

check: fmt-check
	@rc=0; \
	shellcheck idiot lib/common.sh || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc

fmt:
	shfmt -w $(SHELL_FILES)

fmt-check:
	@shfmt -d $(SHELL_FILES) || { echo 'Run: make fmt'; exit 1; }

install:
	install -d '$(LIBDIR)' '$(BINDIR)'
	rm -rf '$(LIBDIR)/cmd' '$(LIBDIR)/lib' '$(LIBDIR)/share'
	cp -rp cmd lib share '$(LIBDIR)/'
	printf '%s\n' '$(VERSION)' > '$(LIBDIR)/lib/version'
	install -m755 idiot '$(LIBDIR)/idiot'
	printf '#!/usr/bin/env sh\n: "$${IDIOT_ROOT:=%s}"\nexec "$${IDIOT_ROOT}/idiot" "$$@"\n' \
	    '$(LIBDIR)' > '$(BINDIR)/idiot'
	chmod 755 '$(BINDIR)/idiot'

uninstall:
	rm -rf '$(LIBDIR)'
	rm -f '$(BINDIR)/idiot'

purge: uninstall
	rm -rf '$(USER_DIR)' '$(CACHE_DIR)'
