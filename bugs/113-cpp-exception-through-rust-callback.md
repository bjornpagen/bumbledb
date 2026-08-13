# A C++ exception escaping a read/write callback unwinds through Rust (UB)
- id: 113
- severity: medium
- confidence: possible
- area: ffi
- components: cpp/bridge/src/db.rs, cpp/bridge/src/lib.rs, cpp/foreign/raii.cc, cpp/AGENTS.md
- status: open (do not fix)

## Summary
`catch_unwind` only catches *Rust* panics. The read/write callbacks are `extern "C"` function pointers invoked from Rust. If a callback is compiled with exceptions enabled and throws, the exception unwinds through `Db::read`/`Db::write` drop glue (including `EscapedIdBurn` and LMDB txn drops) and into `-fno-exceptions` frames — undefined behavior on both sides. The in-tree SDK sets `-fno-exceptions`, so this is a third-party / mixed-TU hazard, not the cookbook path.

## Evidence
- `call_read_callback` / `call_write_callback` `unsafe { callback(context, ptr) }` with no C++ try (impossible in the Rust TU) (`cpp/bridge/src/db.rs`).
- `guard` uses `catch_unwind` only (`cpp/bridge/src/lib.rs`).
- Production C++: `-fno-exceptions` (`cpp/AGENTS.md`). raii trampolines are lambdas in that dialect.
- The public C ABI (`bdb_read_callback`) can be called from any C++ TU. A plugin compiled with exceptions can throw.

## Why this is a bug
Foreign exceptions through Rust are explicitly undefined (nomicon / `extern "C"`). Drop of `WriteTx` / `EscapedIdBurn` during a C++ unwind is particularly dangerous (double-fault abort or skipped burn). The panic wall was built for this class of bug but only covers `panic!`.

## How to trigger / repro sketch
Link a C++ callback compiled *with* exceptions:
```
throw std::runtime_error("nope");
```
from inside `bdb_db_write`. Expect process abort or corrupted LMDB, not `BDB_ERROR_KIND_PANIC`. In-tree presets cannot throw; this needs a throw-enabled TU on purpose.

## Spec / docs notes
`TODO_CPP.md` §30 discusses Rust panic into C++, not C++ throw into Rust. The C ABI is still a published surface.

## Related
- 107 (Rust panic wall holes)
