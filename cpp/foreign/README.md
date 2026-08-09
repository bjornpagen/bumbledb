# foreign/

Quarantine for the Rust bridge boundary (TODO_CPP §31): the generated C ABI
header `bumbledb_c.h` and the one wrapper module `bridge.cppm` that is the
only translation unit allowed to include it. The header is a foreign
generated/toolchain artifact, so it does not violate the dialect's
preprocessor ban — but nothing above this directory may include it or depend
on preprocessor state transitively.

Rules:

- The wrapper exports a safe named module upward: `std::expected` errors,
  RAII ownership, spans/views — never raw pointers, error codes with
  out-params, or macro configuration.
- No dialect module may include a foreign header.
- Do not move ordinary SDK code here to escape a rule.

Contents: `bumbledb_c.h` (generated, read-only), `bridge.cppm` (the
`bumbledb.foreign` wrapper module re-exporting the raw ABI surface inside
`bdb::foreign`), and the CMake wiring that cargo-builds `cpp/bridge` into
the preset's binary dir and imports the resulting staticlib.
