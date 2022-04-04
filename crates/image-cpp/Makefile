WASI_CC       ?= /opt/wasi-sdk/bin/clang
WIT_BINDGEN   ?= wit-bindgen
WASMLINK      ?= wasmlink

bindgen:
	$(WIT_BINDGEN) c --export ../../../wit/ephemeral/wasi-ce.wit --out-dir bindings
	
build:
	$(WASI_CC) -Wall -I . -I ./bindings -c -o wasi_ce.o bindings/wasi-ce.c
	$(WASI_CC) -Wall -mexec-model=reactor lib.cpp wasi_ce.o -o ctest.wasm


link: link-fs

link-fs:
	$(WASMLINK) ctest.wasm \
		--interface wasi-cache=../../../wit/ephemeral/wasi-cache.wit \
		--profile wasmtime \
		--module wasi-cache=../../../crates/cache-fs/target/wasm32-wasi/release/wasi_cache_fs.wasm \
		--output ctest-fs-linked.wasm

clean:
	rm *.wasm && rm *.o
