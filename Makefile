PREFIX ?= /usr/local
INSTALL ?= install
TEST_IMG_NAME ?= wasmtest:latest
TEST_IMG_NAME_CPP ?= wasmtest_cpp:latest
TEST_IMG_NAME_DOTNET ?= wasmtest_dotnet:latest
TEST_IMG_NAME_SPINK ?= wasmtest_spink:latest

CONTAINERD_NAMESPACE ?= default

.PHONY: build
build:
	cargo build --release

.PHONY: install
install:
	sudo $(INSTALL) target/release/containerd-shim-*-v1 $(PREFIX)/bin

update-deps:
	cargo update
	
test/out_rs/img.tar: images/image-rs/Dockerfile images/image-rs/src/lib.rs images/image-rs/Cargo.toml images/image-rs/Cargo.lock
	mkdir -p $(@D)
	docker build -t $(TEST_IMG_NAME) ./images/image-rs
	docker save -o $@ $(TEST_IMG_NAME)

test/out_cpp/img.tar: images/image-cpp/Dockerfile
	mkdir -p $(@D)
	docker build -t $(TEST_IMG_NAME_CPP) ./images/image-cpp
	docker save -o $@ $(TEST_IMG_NAME_CPP)

test/out_dotnet/img.tar: images/aspnet/Dockerfile
	mkdir -p $(@D)
	docker build -t $(TEST_IMG_NAME_DOTNET) ./images/aspnet
	docker save -o $@ $(TEST_IMG_NAME_DOTNET)	

test/out_spin-k/img.tar: images/spin-kitchensink/Dockerfile
	mkdir -p $(@D)
	docker build -t $(TEST_IMG_NAME_SPINK) ./images/spin-kitchensink
	docker save -o $@ $(TEST_IMG_NAME_SPINK)

load: test/out_rs/img.tar test/out_cpp/img.tar test/out_dotnet/img.tar test/out_spin-k/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import test/out_rs/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import test/out_cpp/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import test/out_dotnet/img.tar
	sudo ctr -n $(CONTAINERD_NAMESPACE) image import test/out_spin-k/img.tar

run:
	sudo ctr run --net-host --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME) testwasm

run_cpp:
	sudo ctr run --net-host --rm --runtime=io.containerd.cehostshim.v1 docker.io/library/$(TEST_IMG_NAME_CPP) testwasm

run_dotnet:
	sudo ctr run --net-host --rm --runtime=io.containerd.aspdotnet.v1 docker.io/library/$(TEST_IMG_NAME_DOTNET) testdotnet

run_spink:
	sudo ctr run --net-host --rm --runtime=io.containerd.spin.v1 docker.io/library/$(TEST_IMG_NAME_SPINK) testspink

clean:
	sudo rm -rf ./test