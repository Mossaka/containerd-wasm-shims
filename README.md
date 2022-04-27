# Containerd wasm shims

[runwasi](https://github.com/cpuguy83/runwasi) is a project that aims to run wasm workloads managed by containerd. 

This project aims to provide custom shim implementations that can run wasm workloads, using [runwasi](https://github.com/cpuguy83/runwasi) as a library

## cloudevent shim
This shim implements a [cloudevents](https://cloudevents.io/) host. It starts a HTTP server and generates cloudevent to pass to the wasm modules. 

It also provides two guest implementations, one in rust and another one in C++. Both the host and guest use [cloudevents-sdk](https://github.com/cloudevents/sdk-rust) to serialize/deserialize events to string. 

This project uses the [Wasm Component Model](https://github.com/WebAssembly/component-model). The main interface file is `wasi-ce.wit`.

## asp.net shim

This shim uses asp.net core server. 

## spin shim

This shim uses [spin](https://github.com/fermyon/spin) engine. 

### build the host
Run `make build`

### package the guest to image
Run `make load`

### install the host
Run `make install`

### test
Run `make run` or `make run_cpp` or `make run_dotnet`

> If you want to compile a statically linked binary
> you can do so by running 
> `RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-musl`. 
