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

Currently README-only: the bridge wrapper module lands in a later phase,
and `add_subdirectory(foreign)` in the top-level CMakeLists.txt with it.
