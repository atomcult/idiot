PREFIX   ?= $(HOME)/.local
BINDIR    = $(PREFIX)/bin
DATADIR   = $(PREFIX)/share/idiot

.PHONY: check install uninstall

check:
	shellcheck idiot lib/common.sh
	find cmd -type f | xargs shellcheck

install:
	install -d '$(DATADIR)' '$(BINDIR)'
	rm -rf '$(DATADIR)/cmd' '$(DATADIR)/lib'
	cp -rp cmd lib '$(DATADIR)/'
	install -m755 idiot '$(DATADIR)/idiot'
	printf '#!/usr/bin/env sh\nIDIOT_ROOT=%s\nexec %s/idiot "$$@"\n' \
	    '$(DATADIR)' '$(DATADIR)' > '$(BINDIR)/idiot'
	chmod 755 '$(BINDIR)/idiot'

uninstall:
	rm -rf '$(DATADIR)'
	rm -f '$(BINDIR)/idiot'
