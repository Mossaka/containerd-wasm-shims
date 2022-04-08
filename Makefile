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
	sudo $(INSTALL) target/release/containerd-shim-*-v1 $(PREFIX)/bin
	
# TODO: build this manually instead of requiring buildx
test/out_rs/img.tar: images/image-rs/Dockerfile images/image-rs/src/lib.rs images/image-rs/Cargo.toml images/image-rs/Cargo.lock
	mkdir -p $(@D)
	sudo docker buildx build --platform=wasi/wasm -o type=docker,dest=$@ -t $(TEST_IMG_NAME) ./images/image-rs

test/out_cpp/img.tar: images/image-cpp/Dockerfile
	mkdir -p $(@D)
	sudo docker buildx build --platform=wasi/wasm -o type=docker,dest=$@ -t $(TEST_IMG_NAME_CPP) ./images/image-cpp

test/out_dotnet/img.tar: images/aspnet/Dockerfile
	mkdir -p $(@D)
	sudo docker buildx build --platform=wasi/wasm -o type=docker,dest=$@ -t $(TEST_IMG_NAME_DOTNET) ./images/aspnet

load: test/out_rs/img.tar test/out_cpp/img.tar test/out_dotnet/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import $^

run:
	sudo ctr run --cni --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME) testwasm

run_cpp:
	sudo ctr run --cni --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME_CPP) testwasm

run_dotnet:
	sudo ctr run --cni --rm --runtime=io.containerd.aspdotnet.v1 docker.io/library/$(TEST_IMG_NAME_DOTNET) testdotnet

clean:
	sudo rm -rf ./test