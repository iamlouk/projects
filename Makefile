.PHONY: clean

clean:
	# TODO: What about Makefiles in subdirs?
	for makefile in $(shell find . -mindepth 2 -maxdepth 2 -name Makefile) ; do \
		make -C "$$(dirname "$${makefile}")" clean || true ; \
	done
	for cargotoml in $(shell find . -name Cargo.toml) ; do \
		(cd "$$(dirname "$${cargotoml}")"; cargo clean) || true ; \
	done

