## cloudevent host implementation for runwasi

[runwasi](https://github.com/cpuguy83/runwasi) is a project that aims to run wasm workloads managed by containerd. 

This project uses runwasi as a library to integrate with the [cloudevents](https://cloudevents.io/) host. It also provides two guest implementations, one in rust and another one in C++. Both the host and guest use [cloudevents-sdk](https://github.com/cloudevents/sdk-rust) to serialize/deserialize events to string. 

This project uses the [Wasm Component Model](https://github.com/WebAssembly/component-model). The main interface file is `wasi-ce.wit`.

### build the host
Run `make build`

### package the guest to image
Run `make load` or `make load_cpp`

### install the host
Run `make install`

### test
Run `make run` or `make run_cpp`

