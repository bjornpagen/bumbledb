# Several extern "C" entry points skip catch_unwind despite the panic-into-C++ policy
- id: 107
- severity: medium
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/lib.rs, cpp/bridge/src/answers.rs, cpp/bridge/src/error.rs, cpp/bridge/src/db.rs
- status: open (do not fix)

## Summary
The bridge’s stated panic policy is that every extern entry runs under `guard` (`catch_unwind`) because unwinding into `-fno-exceptions` C++ is UB. Accessors that return a scalar instead of `bdb_status` — and `bdb_answers_new` — call `ref_in` / `box_out` with no `catch_unwind`. A panic there (future code in `Answers::new`, a debug assertion in `len`, allocator hooks) unwinds the C++ stack.

## Evidence
- Policy: “EVERY extern entry point routes through `guard`” (`cpp/bridge/src/lib.rs`).
- Unguarded: `bdb_answers_new`, `bdb_answers_len`, `bdb_answers_arity`, `bdb_row_set_len`, `bdb_row_set_arity`, `bdb_error_get_kind`, `bdb_error_violation_count`.
- These still dereference caller pointers via `ref_in` (null is MISUSE-shaped for some, but a *dangling* pointer is UB, not a catchable panic). The gap is panics *after* a successful `ref_in` (e.g. inside `Answers::len`) or inside `box_out`.

## Why this is a bug
The C++ SDK is built `-fno-exceptions`. Rust panic = `longjmp`-like unwind through foreign frames = undefined behavior, which is exactly why `guard` exists. The policy is already violated for the “infallible” accessors. Today `len` is unlikely to panic; `bdb_answers_new` is a heap allocation (OOM typically aborts rather than panics, but the policy still names catch_unwind as the wall).

## How to trigger / repro sketch
Not easily triggerable without injecting a panic in `Answers::len` or `Answers::new`. The defect is the missing wall, not a current panic site. A unit test analog to `test_only_trigger_panic` on `bdb_answers_new` would demonstrate unwind.

## Spec / docs notes
`TODO_CPP.md` §30 / `cpp/bridge/src/lib.rs` panic policy. C++ is `-fno-exceptions` (`cpp/AGENTS.md` §11).

## Related
- 103 (callback return is another unguarded UB class)
