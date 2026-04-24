.POSIX:

PREFIX   ?= $(HOME)/.local
BINDIR    = $(PREFIX)/bin
LIBDIR    = $(PREFIX)/lib/idiot

VERSION  := $(shell git describe --tags --always --dirty 2>/dev/null || printf 'unknown')

.PHONY: check install uninstall

check:
	@rc=0; \
	shellcheck idiot lib/common.sh || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc

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
