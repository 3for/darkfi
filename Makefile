.POSIX:

# Install prefix
PREFIX = /usr/local

# Cargo binary
CARGO = cargo

# Here it's possible to append "cashierd" and "gatewayd".
BINS = drk darkfid

# Dependencies which should force the binaries to be rebuilt
BINDEPS = \
	Cargo.toml \
	$(shell find src -type f) \
	$(shell find token -type f) \
	$(shell find sql -type f)

all: $(BINS)

$(BINS): $(BINDEPS)
	$(CARGO) build --release --all-features --bin $@
	cp -f target/release/$@ $@

test:
	$(CARGO) test --release --all-features

fix:
	$(CARGO) clippy --release --all-features --fix --allow-dirty

clippy:
	$(CARGO) clippy --release --all-features

install: all
	mkdir -p $(DESTDIR)$(PREFIX)/bin
	mkdir -p $(DESTDIR)$(PREFIX)/share/darkfi
	mkdir -p $(DESTDIR)$(PREFIX)/share/doc/darkfi
	cp -f $(BINS) $(DESTDIR)$(PREFIX)/bin
	for i in $(BINS); \
	do \
		cp -f example/config/$$i.toml $(DESTDIR)$(PREFIX)/share/doc/darkfi; \
	done;

uninstall:
	for i in $(BINS); \
	do \
		rm -f $(DESTDIR)$(PREFIX)/bin/$$i; \
	done;
	rm -rf $(DESTDIR)$(PREFIX)/share/doc/darkfi
	rm -rf $(DESTDIR)$(PREFIX)/share/darkfi

clean:
	rm -f $(BINS)

distclean: clean
	rm -rf target

.PHONY: all test fix clippy install uninstall clean distclean
