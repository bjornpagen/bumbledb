# Execute clears (and may partially refill) the answers carrier before the call can fail
- id: 110
- severity: medium
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/answers.rs, crates/bumbledb/src/api/prepared/execute.rs, cpp/src/db/snapshot.cc, cpp/AGENTS.md
- status: open (do not fix)

## Summary
`bdb_snapshot_execute` `clear()`s the caller’s `bdb_answers` then calls `execute_args`, which `clear()`s again and may write rows before returning `Err`. The C++ dialect wraps this as `std::expected<void, Error>` / `execute_into`. Project law is that a failing call leaves caller-visible outputs as the caller left them. Here a bind/type/`ForeignPrepared` error wipes a previously filled reusable carrier (and a mid-execution error can leave a partial buffer).

## Evidence
- Bridge: `carrier.answers.clear(); snap.execute_args(..., &mut carrier.answers)?` (`cpp/bridge/src/answers.rs`).
- Engine: `execute_args` does `out.clear(); out.arity = …; bind_param_args?; run_bound` (`crates/bumbledb/src/api/prepared/execute.rs`). Bind errors happen after clear; `run_bound` errors can happen after rows are appended.
- C++ `Snapshot::execute_into` returns `expected<void, Error>` (`cpp/src/db/snapshot.cc`).
- `cpp/AGENTS.md` §26: “On the `unexpected` path every caller-visible output — out-buffers, referenced targets, partially built state — is exactly as the caller left it, unless the surface’s documentation says otherwise in so many words.”
- The C header does say the carrier is “cleared first”; the dialect `execute_into` documentation does not restate that prior answers are destroyed on failure.

## Why this is a bug
Warm-path reuse is the point of `bdb_answers`. A failed execute (wrong param type, foreign prepared, bind miss) drops the previous result set. Callers that keep one carrier and branch on `expected` will iterate empty/partial rows after an error, or lose the last good result. This is a lifetime/ownership bug at the FFI boundary: the carrier is mutated before the operation is committed.

## How to trigger / repro sketch
1. Execute a valid query into `AnswersRaw` (len > 0).
2. Execute again with a param type mismatch or a prepared query from another db.
3. Observe `answers.len() == 0` (or partial) *and* an error. The previous rows are gone.

## Spec / docs notes
C ABI: documented clear-first (so the raw export is a documented exception). C++ dialect `execute_into` inherits AGENTS.md failure-transparency without restating the exception — that is the dialect bug. Engine `execute_args` is the shared implementation.

## Related
- 105 (side effect then MISUSE)
- 111 (views into the carrier after clear)
