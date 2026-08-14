# foreign/

Quarantine for the C ABI and any external interface whose headers, macros,
or native APIs may not leak into dialect code.

The ownership law still applies: every project-owned allocation has one RAII
owner; raw pointers are transient ABI borrows; no raw ownership crosses the
module boundary.

Current residents:

- `bumbledb_foreign.cc` is the primary interface of module `bumbledb_foreign`.
- `bridge.cc` re-exports the cbindgen C surface as a named-module partition.
- `raii.cc` is the only translation unit that includes the generated C header
  and owns the FFI resources.
- `query_view.cc` is quarantine code but a partition of module `bumbledb`
  (`:query_view`): it consumes the query IR partitions, and a
  `bumbledb_foreign` partition could not import them without a module cycle.
- `../bridge/` is the Rust staticlib cargo builds; CMake treats cargo as the
  dependency tracker.

The Clang lint graph covers these translation units with the quarantine
profile in `.clang-tidy`. The generated C header is excluded from tidy
(ABI enum widths are the C surface, not a performance choice). Dialect
rules stay with GCC and review until Clang parses the reflection-bearing
module.
