# Static Linux ARM64

Bumbledb's Rust core and standalone Linux executables can be built for
`aarch64-unknown-linux-musl` without a dynamic loader or shared libraries.
The qualification lane builds **every workspace release binary** with its
default dependencies, including LMDB's C implementation and duty's TLS stack.
It does not run benchmarks.

This is separate from the existing glibc-based npm native add-on. A Node
`.node` file is a shared object loaded by Node, not a static executable.
There is no new public C API: the C driver below is a private link test for
the Rust core archive. Rust consumers still use the Rust API and pinned
toolchain; the archive is not a compiler-independent Rust ABI.

## Reproduce the complete qualification

On an ARM64 machine with Finch running (Docker works with the same arguments):

```sh
finch build --platform linux/arm64 \
  -t bumbledb-static-linux-arm64 \
  -f scripts/static-linux-arm64/Dockerfile .
mkdir -p bench-out/static-linux-arm64/artifacts
finch run --rm --platform linux/arm64 \
  --mount type=volume,src=bumbledb-musl-target,dst=/workspace/target \
  --mount "type=bind,src=$PWD/bench-out/static-linux-arm64/artifacts,dst=/out" \
  bumbledb-static-linux-arm64 \
  bash scripts/static-linux-arm64/check.sh /out
```

The image uses a digest-pinned Alpine base and the repository's exact Rust
nightly. Alpine packages are installed from the 3.23 repositories; their
patch versions can advance. This is a repeatable build procedure, not a
claim of byte-for-byte reproducibility across future package updates.
The first toolchain build downloads and compiles nextest; subsequent builds
reuse container and Cargo caches. QEMU uses software emulation (TCG), so
no KVM device or privileged container is required. The build container runs
as root for its disposable chroot and device-node setup.

The gate requires all of these checks to pass:

- Every emitted workspace binary is AArch64 ELF with no `PT_INTERP` or
  `DT_NEEDED`; a real dynamic executable must fail this verifier.
- The delivered `libbumbledb.a` contains the core, Rust dependencies, LMDB,
  musl libc, and Rust's matching unwinder. The C link succeeds with that
  archive and fails without it.
- A private C/Rust consumer exercises threads/TLS, caught panics, real LMDB
  transactions, typed queries, exact float aggregation, budget refusal, and
  close/reopen persistence.
- The consumer and duty execute in a fresh chroot with no `/lib` or
  `/usr/lib`, then in full-system QEMU running a real ARM64 Linux kernel on
  an emulated Cortex-A53. The guest has no shared libraries or network.
- All-feature workspace nextest tests and doctests, the store-disabled log
  compile check, and the release allocation gate pass on native ARM64 musl.
  Tests explicitly marked ignored remain ignored, including external-service
  and performance qualification; their absence is not reported as a pass.

`verification.json` and `SHA256SUMS` are written only after all stages succeed.
Individual logs can remain after failure; their presence alone is not success.
CI records the exact source commit and uploads the archive, two tested
executables, link map, ELF reports, QEMU evidence, and notices. It does not
tag, release, or publish packages.

## Embedding and linking details

For an ordinary Rust executable, build inside the musl image:

```sh
RUSTUP_TOOLCHAIN=nightly-2026-08-15 \
  cargo build --locked --release --target aarch64-unknown-linux-musl
```

The image sets target-specific `+crt-static` and musl C compiler/archive
tools. Host compiler plug-ins stay separate from the target artifacts.
Do not pass global `RUSTFLAGS="-C target-feature=+crt-static"` to host proc-macros.

The archive test emits `rlib,staticlib` with `cargo rustc`, then merges the
core archive with the **same Rust toolchain's** musl `libc.a` and
`libunwind.a`. Its C link uses matching CRT startup objects, `-nostdlib`,
`-static`, and `--eh-frame-hdr`. These details matter: mixing Alpine's
implicit libc with Rust's bundled musl can duplicate private TLS/allocator
symbols, and omitting the unwind header breaks caught panics. Use
`scripts/static-linux-arm64/link-probe.py` as the tested reference rather
than assuming `cc app.c libbumbledb.a` is a supported public C integration.

QEMU **user mode** is insufficient for this test: its missing
`get_robust_list` syscall prevents LMDB's robust process-shared mutex setup.
Full-system QEMU supplies the real kernel behavior. LMDB locking and Rust
unwinding are deliberately preserved, not compiled out.

## Distribution scope

Static linking does not remove third-party license obligations. The gate
preserves dependency license/notice files and Rust's distribution notices
(including musl and unwinder attribution) in `notices.tar.gz`. Its inventory
is conservative and includes build/test dependencies; it is not a linked-code
SBOM or release-license approval. The kernel and BusyBox are test fixtures,
not Bumbledb product binaries. Any future public distribution must retain
applicable notices and satisfy source-distribution obligations for fixtures
if they are included. This lane does not change the published 1.0.1 assets.
