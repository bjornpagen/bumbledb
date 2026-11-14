# Native musl build userspace; the same image runs locally (Finch) and in CI (Docker).
FROM alpine:3.24.4@sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf

RUN apk add --no-cache \
    bash binutils build-base ca-certificates clang cmake curl file git \
    linux-headers musl-dev perl pkgconf python3 tar xz \
    busybox-static cpio linux-virt qemu-system-aarch64

ENV PATH="/root/.cargo/bin:${PATH}"
ARG NEXTEST_VERSION=0.9.152
ARG NEXTEST_SHA256=f168dbfe9c9c3a4b247cf10d9c260f3dac90a1373f5c4b1bfd3b0b8ac8d77095
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
