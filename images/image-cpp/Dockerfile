FROM rust:1.59 AS build
WORKDIR /opt/wasmtest
COPY . .

FROM scratch
ENTRYPOINT ["/wasm"]
COPY --from=build /opt/wasmtest/ctest.wasm /wasm