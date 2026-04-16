PREFIX   ?= $(HOME)/.local
BINDIR    = $(PREFIX)/bin
DATADIR   = $(PREFIX)/share/idiot

.PHONY: check install uninstall

check:
	@rc=0; \
	shellcheck idiot lib/common.sh || rc=1; \
	find cmd -type f | xargs shellcheck -x || rc=1; \
	exit $$rc

install:
	install -d '$(DATADIR)' '$(BINDIR)'
	rm -rf '$(DATADIR)/cmd' '$(DATADIR)/lib'
	cp -rp cmd lib '$(DATADIR)/'
	install -m755 idiot '$(DATADIR)/idiot'
	printf '#!/usr/bin/env sh\nIDIOT_ROOT=%s\nexec "$${IDIOT_ROOT}/idiot" "$$@"\n' \
	    '$(DATADIR)' > '$(BINDIR)/idiot'
	chmod 755 '$(BINDIR)/idiot'

uninstall:
	rm -rf '$(DATADIR)'
	rm -f '$(BINDIR)/idiot'
