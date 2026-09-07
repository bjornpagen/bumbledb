# Native musl build userspace; the same image is used by Finch and CI Docker.
FROM alpine:3.23@sha256:fd791d74b68913cbb027c6546007b3f0d3bc45125f797758156952bc2d6daf40

RUN apk add --no-cache \
    bash binutils build-base ca-certificates clang cmake curl file git \
    linux-headers musl-dev perl pkgconf python3 tar xz \
    busybox-static cpio linux-virt qemu-system-aarch64

ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /workspace
COPY rust-toolchain.toml /toolchain/rust-toolchain.toml
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain none --profile minimal && \
    channel=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' /toolchain/rust-toolchain.toml) && \
    rustup toolchain install "$channel" --profile minimal --component rustfmt,clippy && \
    rustup default "$channel"

# Explicit --target keeps the host proc-macro toolchain distinct from target
# code. The target's C/Rust/LMDB code must all use musl and static CRT linkage.
ENV CARGO_BUILD_TARGET=aarch64-unknown-linux-musl
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-feature=+crt-static"
ENV CC_aarch64_unknown_linux_musl=cc
ENV AR_aarch64_unknown_linux_musl=ar
RUN --mount=type=cache,target=/tmp/bumbledb-nextest-build \
    CARGO_TARGET_DIR=/tmp/bumbledb-nextest-build \
    cargo install cargo-nextest --version 0.9.143 --locked --target aarch64-unknown-linux-musl
COPY . /workspace
