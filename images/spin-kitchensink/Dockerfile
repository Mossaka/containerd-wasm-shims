FROM --platform=${BUILDPLATFORM} rust:1.59 AS build
RUN curl -LO https://go.dev/dl/go1.17.9.linux-amd64.tar.gz && \
    rm -rf /usr/local/go && \ 
    tar -C /usr/local -xzf go1.17.9.linux-amd64.tar.gz && \
    rm go1.17.9.linux-amd64.tar.gz && \
    export PATH=$PATH:/usr/local/go/bin
RUN apt-get update && apt-get install npm -y
RUN wget https://github.com/tinygo-org/tinygo/releases/download/v0.22.0/tinygo_0.22.0_amd64.deb
RUN dpkg -i tinygo_0.22.0_amd64.deb
RUN rustup target add wasm32-wasi
RUN curl -LO https://ziglang.org/builds/zig-linux-x86_64-0.10.0-dev.1927+cf20b97b7.tar.xz && \
    tar -C /usr/local -xJf zig-linux-x86_64-0.10.0-dev.1927+cf20b97b7.tar.xz && \
    rm zig-linux-x86_64-0.10.0-dev.1927+cf20b97b7.tar.xz
ENV PATH="/usr/local/zig-linux-x86_64-0.10.0-dev.1927+cf20b97b7:${PATH}"
RUN which zig
WORKDIR /opt/wasmtest
RUN git clone https://github.com/Mossaka/spin-kitchensink && \
    cp -r spin-kitchensink/* . && \
    rm -rf spin-kitchensink
RUN make build

FROM scratch
COPY --from=build /opt/wasmtest .