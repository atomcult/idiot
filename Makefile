.POSIX:

PREFIX   ?= $(HOME)/.local
BINDIR    = $(PREFIX)/bin
LIBDIR    = $(PREFIX)/lib/idiot

.PHONY: check install uninstall

check:
	@rc=0; \
	shellcheck idiot lib/common.sh || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc

install:
	install -d '$(LIBDIR)' '$(BINDIR)'
	rm -rf '$(LIBDIR)/cmd' '$(LIBDIR)/lib'
	cp -rp cmd lib '$(LIBDIR)/'
	install -m755 idiot '$(LIBDIR)/idiot'
	printf '#!/usr/bin/env sh\n: "$${IDIOT_ROOT:=%s}"\nexec "$${IDIOT_ROOT}/idiot" "$$@"\n' \
	    '$(LIBDIR)' > '$(BINDIR)/idiot'
	chmod 755 '$(BINDIR)/idiot'

uninstall:
	rm -rf '$(LIBDIR)'
	rm -f '$(BINDIR)/idiot'
