# cpp/ — the C++26 SDK

A reflective C++26 frontend over the Rust bumbledb engine: ordinary C++
declarations elaborate (via C++26 reflection, at compile time) into the same
`SchemaSpec`/query IR the TypeScript SDK lowers to, crossing one C ABI into
the unmodified engine. The engine stays the sole semantic authority — the
frontend lowers, it never re-judges.

[`AGENTS.md`](AGENTS.md) is normative. The build enforces the pinned
toolchain tuple; this document deliberately does not duplicate its versions.
The lowering contract is [`docs/architecture/75-cpp-lowering.md`](../docs/architecture/75-cpp-lowering.md).

## Tree

```text
cpp/
├── src/       module `bumbledb` — the one SDK module; every internal is a
│              partition (types/, relation/, closed/, schema/, query/,
│              answers/, db/), GCC-only (reflective)
├── foreign/   quarantine zone: module `bumbledb_foreign` (:abi, :raii — the
│              only code that sees the generated C header), the cargo-built
│              bridge staticlib, and the :query_view partition of
│              `bumbledb`
├── bridge/    the Rust C-ABI bridge crate (cargo owns this graph)
└── tests/     runtime/ (module surface), cookbook/ (r01–r32 fingerprint +
               semantics parity), compile_fail/ (pinned diagnostics),
               bridge/ (raw ABI smoke), conformance.cc (libstdc++ surface)
```

## Build

Bring the pinned tools on `PATH`; configure rejects every other tuple. Local
compiler paths belong in a gitignored `CMakeUserPresets.json`. Production
presets discover `g++` (or `$CXX`); the CMake gate is a GCC 16+ floor, not a
pinned binary name.

```sh
cmake --preset dev && cmake --build --preset dev && ctest --preset dev
cmake --preset release && cmake --build --preset release && ctest --preset release
cmake --preset asan-ubsan && cmake --build --preset asan-ubsan && ctest --preset asan-ubsan
```

The lint graph (`lint` / `lint-local`) builds the reflection-free quarantine
with clang-tidy; there is no lint test preset. ThreadSanitizer (`tsan`) is a
supported Linux personality: the engine behind the C ABI is concurrent.

The release graph is whole-program-optimized and the whole production graph
is hardened by default; none of it is configurable. Every configuration
builds with `_GLIBCXX_ASSERTIONS`, `-ftrivial-auto-var-init=zero`,
`-fstack-protector-strong`, `-fstack-clash-protection` (GCC graph only; the
lint Clang rejects the flag on this target), and
`-fzero-call-used-regs=used-gpr`. Dev and release compile with trap-mode
UBSan (`-fsanitize=undefined -fsanitize-trap=all`); the `asan-ubsan` and
`tsan` presets use the runtime sanitizers instead and never define
`_FORTIFY_SOURCE`. Linux additionally enables `_FORTIFY_SOURCE=3`,
`-mbranch-protection=standard`, PIE, and RELRO/NOW. Those Darwin-inert flags
stay off and are recorded in `PINS.md`. The `release` preset additionally
links with GCC LTO. LTO applies to the `release` preset only: a `-flto -g`
link has unbounded memory growth on the pinned Darwin toolchain (registry
entry `gcc-darwin-lto-debug-dsymutil` in `PINS.md`).
