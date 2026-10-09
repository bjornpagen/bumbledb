# Native musl build userspace; the same image runs locally (Finch) and in CI (Docker).
FROM alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6

RUN apk add --no-cache \
    bash binutils build-base ca-certificates clang cmake curl file git \
    linux-headers musl-dev perl pkgconf python3 tar xz \
    busybox-static cpio linux-virt qemu-system-aarch64

ENV PATH="/root/.cargo/bin:${PATH}"
ARG NEXTEST_VERSION=0.9.148
ARG NEXTEST_SHA256=94676506ceaed01746d78cfe9ed124ee841b2932fd18d9d10f438a347bd9ba5e
RUN mkdir -p /root/.cargo/bin && \
    curl --proto '=https' --tlsv1.2 -fsSL -o /tmp/nextest.tar.gz \
    "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/cargo-nextest-${NEXTEST_VERSION}-aarch64-unknown-linux-musl.tar.gz" && \
    echo "${NEXTEST_SHA256}  /tmp/nextest.tar.gz" | sha256sum -c - && \
    tar -xzf /tmp/nextest.tar.gz -C /root/.cargo/bin && \
    rm /tmp/nextest.tar.gz

COPY rust-toolchain.toml /toolchain/rust-toolchain.toml
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain none --profile minimal && \
    cd /toolchain && rustup toolchain install

# Explicit --target keeps the host proc-macro toolchain distinct from target
# code. The target's C/Rust/LMDB code must all use musl and static CRT linkage.
ENV CARGO_BUILD_TARGET=aarch64-unknown-linux-musl
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-feature=+crt-static"
ENV CC_aarch64_unknown_linux_musl=cc
ENV AR_aarch64_unknown_linux_musl=ar
WORKDIR /workspace
COPY . /workspace
