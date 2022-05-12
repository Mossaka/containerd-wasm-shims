FROM node:slim AS build
RUN npm i wasm-opt -g
WORKDIR /opt/wasmtest
COPY ./wasiwebapp.wasm .
RUN wasm-opt ./wasiwebapp.wasm -O3 -o ./wasiwebapp.opt.wasm

FROM scratch
ENTRYPOINT ["/wasm"]
COPY --from=build /opt/wasmtest/wasiwebapp.opt.wasm /wasm