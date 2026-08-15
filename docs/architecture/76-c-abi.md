# 76 — C ABI

The native foreign surface is `bdb_*`: a C header and a Rust staticlib/cdylib.
Hosts lower to `SchemaSpec` and query IR; the engine judges. There is no
reflective C++ frontend.

## Where the symbols live

**Decision:** leaf crate `crates/bumbledb-c` (`libbumbledb_c`), path-depending
on `bumbledb`, excluded from the engine workspace — the `ts/crate` law.
**Alternative:** feature `c-abi` on `crates/bumbledb` (one crate, cbindgen from
the engine). **Why it lost:** the workspace stays heed+blake3-pure. C types,
cbindgen, and `staticlib`/`cdylib` are FFI ceremony; putting them in the engine
crate would compile them into every engine build. The marshal is a full hostile
boundary (handles, lexical callbacks, tagged IR/schema views), not a handful of
`extern "C"` wrappers. The engine's only `extern "C"` remains `fastclock`'s
`sysctlbyname`. **Reverses if:** the C ABI shrinks to a thin re-export of engine
types with no view structs of its own.

Dumb-bridge law, verbatim from `ts/crate`: no logic beyond marshaling. No
schema knowledge beyond schema-directed rendering of rejections, no validation,
no name resolution, no retries. Anything smart belongs in the host or in
`bumbledb`.

## Linkage — C only

The crate links with **no C++ runtime**. Callbacks are `extern "C" fn` invoked
directly from Rust. Every export runs under `catch_unwind`; a Rust panic
becomes `BDB_ERROR_KIND_PANIC` (store poisoned). A C++ exception thrown through
a callback is **unsupported** — it would unwind through Rust, which is
undefined behavior. Catch C++ exceptions in the host before they enter `bdb_*`,
or do not throw.

**Alternative:** keep `callback_trampoline.cc` (exceptions ON) so a throw
becomes `BDB_CALLBACK_CONTROL_ABORT`. **Why it lost:** the default build must
be C-only linkable; an exceptions-ON TU is a second language in the link.
**Reverses if:** a supported C++ host product must throw through callbacks.

## What is C, what is Rust

**C structs** (the hostile boundary): tagged POD views (`bdb_value`, schema
spec, query IR), opaque handles (`bdb_db`, `bdb_prepared`, `bdb_answers`,
`bdb_row_set`), lexical refs (`bdb_snapshot_ref`, `bdb_tx_ref`), the
`bdb_status` + `bdb_error**` protocol. Tags are `u32` so an out-of-range C enum
is `BDB_STATUS_MISUSE`, not UB. Most payloads are flat parallel fields keyed
by `kind`. `bdb_query` is the one C union (`Cq | Reach`).

**Rust only:** `SchemaSpec::descriptor()`, `SchemaDescriptor::validate`,
fingerprint, prepare, execute, commit judgment. The crate copies inbound views
into owned engine values before any call returns; outbound string/bytes views
borrow the named carrier and die with it.

The header is generated: pinned cbindgen 0.29.4, committed at
`crates/bumbledb-c/include/bumbledb_c.h`. `bdb_version()` is the crate version
string (program lifetime, NUL-terminated). `bdb_abi_version()` is `1` — bump
on a layout-visible change.

## Host lowering

A host (TypeScript today; any C caller) builds a `SchemaSpec` / query IR and
hands it across. Fingerprint parity is an engine obligation: identical specs
lower to identical descriptors. Goldens live at
`fixtures/cookbook-fingerprints.txt` (TS reads them). Construction-time walls
(class algorithm, implied-key rejection, `mirrors` as one bidirectional
statement) are host sugar; the engine remains the wall.

## Holes

Not this cut:

- **Flattened tags, not C unions** (except `bdb_query`). `bdb_value` and the
  IR/spec views carry every payload field; only `kind` selects which are live.
  Packing those as unions is an ABI bump (`bdb_abi_version`).
- **No exhume.** Archival open is a Rust/TS surface (`70-api.md`).
- **No explain / staleness.** Harness-only per `70-api.md`; not embedding API.
