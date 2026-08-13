# Moved-from C++ error_handle::kind() returns Panic instead of aborting
- id: 117
- severity: low
- confidence: confirmed
- area: ffi
- components: cpp/foreign/raii.cc, cpp/bridge/src/error.rs
- status: open (do not fix)

## Summary
`error_handle` documents that a moved-from handle is inert and that every accessor is the unreachable boundary state (`std::abort`). `kind()` calls `bdb_error_get_kind(raw_)` with no null check. Rust maps a null error to `BDB_ERROR_KIND_PANIC` (comment: “stop trusting this process’s bridge state”). A moved-from C++ error therefore looks like a caught Rust panic / poisoned store.

## Evidence
- `error_handle::kind()`: `return bdb_error_get_kind(raw_);` with no `raw_ == nullptr` test (`cpp/foreign/raii.cc`).
- `message()`, `generation_moved()`, `violation()` either abort on non-OK or treat misuse as nullopt; `kind()` cannot return status.
- Move sets `other.raw_ = nullptr`.
- `bdb_error_get_kind` null → `bdb_error_kind::Panic` (`cpp/bridge/src/error.rs`).
- Class comment: “never hand a moved-from error onward.”

## Why this is a bug
Panic is a semantic signal the C++ SDK is supposed to treat as process-level poison. Moved-from is a C++ ownership mistake and should abort (the module’s rule) or be unreachable, not impersonate a Rust panic. Callers that `switch` on `kind()` after a move (or after `std::move` into `unexpected`) can take the poison path spuriously.

## How to trigger / repro sketch
```
auto e = /* failing open */;
auto k1 = e.kind(); // real kind
auto e2 = std::move(e);
auto k2 = e.kind(); // BDB_ERROR_KIND_PANIC, no abort
```

## Spec / docs notes
raii.cc: moved-from is unreachable_boundary_state. `kind()` is the one accessor that does not honor it.

## Related
- 107 (Panic kind is also the catch_unwind mapping)
