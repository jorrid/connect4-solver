FROM ubuntu:24.04 AS base
WORKDIR /project
RUN apt-get -y update && apt-get -y install curl musl-tools binutils build-essential

FROM base AS rust_deps
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="$PATH:/root/.cargo/bin"
# Switch to nightly
RUN rustup toolchain install nightly
RUN rustup default nightly
RUN rustup target add x86_64-unknown-linux-musl
# Fetch build dependencies
RUN mkdir src
COPY Cargo.toml Cargo.lock .cargo /project/
RUN echo "pub fn main() {panic!(\"If you see this, the source did not get copied in a later stage.\");}" > src/main.rs
RUN cargo fetch --manifest-path /project/Cargo.toml
RUN rm -r ./*

FROM ubuntu:24.04 AS code_server
RUN apt-get update -y && apt-get install -y curl
# --method standalone means that everything goes into ~/.local/
RUN curl -fsSL https://code-server.dev/install.sh | sh -s -- --method standalone
RUN /root/.local/bin/code-server --verbose --install-extension rust-lang.rust-analyzer
RUN rm -r /root/.local/share/code-server/CachedExtensionVSIXs

FROM rust_deps AS rust_src
COPY ./ /project/
# Need this to cause cargo cache invalidation, see https://github.com/docker/buildx/issues/554
# and https://francoisbest.com/posts/2021/cargo-docker-mtime
RUN --network=none touch /project/Cargo.toml /project/src/main.rs

FROM rust_deps AS dev
RUN rustup component add rust-src
COPY --from=code_server /root/.local/ /root/.local/
RUN --network=none mv /root/.local/bin/code-server /usr/bin/code-server
# --disable-workspace-trust : workspace trust is stored in the *browser* (local storage) by code-server
# see https://github.com/coder/code-server/issues/4212 (also open tabs etc. are stored in local storage)
CMD code-server --disable-telemetry --auth none --disable-workspace-trust --socket /project/code-server-socket /project

FROM rust_src AS main_build
RUN --network=none --mount=type=cache,target=/cargo_target \
    cargo build --release --target-dir=/cargo_target --manifest-path /project/Cargo.toml && \
    cp /cargo_target/x86_64-unknown-linux-musl/release/connect4-rust /project/main

FROM alpine AS main
COPY --from=main_build /project/main /
CMD ["/main"]
