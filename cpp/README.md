# cpp/ — the C++26 SDK

A reflective C++26 frontend over the Rust bumbledb engine: ordinary C++
declarations elaborate (via C++26 reflection, at compile time) into the same
`SchemaSpec`/query IR the TypeScript SDK lowers to, crossing one C ABI into
the unmodified engine. The engine stays the sole semantic authority — the
frontend lowers, it never re-judges.

Normative references:

- `AGENTS.md` (this directory) — the language dialect and file discipline.
- `../docs/architecture/75-cpp-lowering.md` — the lowering contract
  (byte-exact fingerprint parity with the TypeScript SDK).
- `../docs/handoffs/2026-08-08-cpp-sdk-design-record.md` — the design
  record (the §-numbers cited in comments throughout this tree).

## Tree

```text
cpp/
├── src/       module `bumbledb` — the one SDK module; every internal is a
│              partition (types/, relation/, closed/, schema/, query/,
│              answers/, db/), GCC-only (reflective)
├── foreign/   quarantine zone: module `bumbledb_foreign` (:abi, :raii — the
│              only code that sees the generated C header), the cargo-built
│              bridge staticlib, and the :foreign_program partition of
│              `bumbledb`
├── bridge/    the Rust C-ABI bridge crate (cargo owns this graph)
└── tests/     runtime/ (module surface), cookbook/ (r01–r32 fingerprint +
               semantics parity), compile_fail/ (pinned diagnostics),
               bridge/ (raw ABI smoke)
```

## Build

```sh
cmake --preset dev && cmake --build --preset dev && ctest --preset dev
cmake --preset release && cmake --build --preset release && ctest --preset release
cmake --preset asan-ubsan && cmake --build --preset asan-ubsan && ctest --preset asan-ubsan
```

The lint graph (`lint-local`, pinned Clang 22 + clang-tidy) builds the
non-reflective subset; toolchain pins and acquisition are in `AGENTS.md`.
