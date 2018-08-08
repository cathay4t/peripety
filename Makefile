DESTDIR ?=
prefix ?= /usr
etcdir = /etc
INSTALL ?= install
bindir = $(prefix)/bin

all:
	cargo build --all --release

target/release/peripetyd:
	cargo build --all --release

tidy:
	cargo fmt

clean:
	cargo clean

install: target/release/peripetyd
	$(INSTALL) -d $(DESTDIR)$(bindir)
	$(INSTALL) -m 755 target/release/peripetyd $(DESTDIR)$(bindir)/
