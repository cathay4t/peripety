DESTDIR ?=
prefix ?= /usr
etcdir = /etc
INSTALL ?= install
bindir = $(prefix)/bin
mandir = $(prefix)/share/man
systemdunitdir ?= $(shell pkg-config --variable=systemdsystemunitdir systemd)

all:
	cargo build --all --release

target/release/peripetyd target/release/prpt:
	cargo build --all --release

tidy:
	cargo fmt

run: all
	sudo target/debug/peripetyd

check:
	./tests/scsi.sh
	./tests/lvm_tp.sh
	./tests/dmmp.sh

clean:
	cargo clean

install: target/release/peripetyd target/release/prpt
	$(INSTALL) -d $(DESTDIR)$(bindir)
	$(INSTALL) -m 755 target/release/peripetyd $(DESTDIR)$(bindir)/
	$(INSTALL) -m 755 target/release/prpt $(DESTDIR)$(bindir)/
	$(INSTALL) -d $(DESTDIR)$(mandir)/man1
	$(INSTALL) -m 644 doc/prpt.1 $(DESTDIR)$(mandir)/man1/
	$(INSTALL) -d $(DESTDIR)$(etcdir)
	$(INSTALL) -m 644 etc/peripetyd.conf $(DESTDIR)$(etcdir)/
	$(INSTALL) -d $(DESTDIR)$(systemdunitdir)
	$(INSTALL) -m 644 etc/peripetyd.service $(DESTDIR)$(systemdunitdir)/
