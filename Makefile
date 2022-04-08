PREFIX ?= /usr/local
INSTALL ?= install
TEST_IMG_NAME ?= wasmtest:latest
TEST_IMG_NAME_CPP ?= wasmtest_cpp:latest
TEST_IMG_NAME_DOTNET ?= wasmtest_dotnet:latest

CONTAINERD_NAMESPACE ?= default

.PHONY: build
build:
	cargo build --release

.PHONY: install
install:
	$(INSTALL) target/release/containerd-shim-cehostshim-v1 $(PREFIX)/bin

# TODO: build this manually instead of requiring buildx
test/out/img.tar: crates/image-rs/Dockerfile crates/image-rs/src/lib.rs crates/image-rs/Cargo.toml crates/image-rs/Cargo.lock
	mkdir -p $(@D)
	docker buildx build --platform=wasi/wasm -o type=docker,dest=$@ -t $(TEST_IMG_NAME) ./crates/image-rs

test/out_cpp/img.tar: crates/image-cpp/Dockerfile
	mkdir -p $(@D)
	docker buildx build --platform=wasi/wasm -o type=docker,dest=$@ -t $(TEST_IMG_NAME_CPP) ./crates/image-cpp

test/out_dotnet/img.tar: crates/aspnet/Dockerfile
	mkdir -p $(@D)
	docker buildx build --platform=wasi/wasm -o type=docker,dest=$@ -t $(TEST_IMG_NAME_DOTNET) ./crates/aspnet

load: test/out/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import $<

load_cpp: test/out_cpp/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import $<

load_dotnet: test/out_dotnet/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import $<

run:
	sudo ctr run --cni --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME) testwasm

run_cpp:
	sudo ctr run --cni --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME_CPP) testwasm

run_dotnet:
	sudo ctr run --cni --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME_DOTNET) testdotnet

clean:
	sudo rm -rf ./test