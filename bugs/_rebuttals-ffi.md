# FFI finding rebuttals (2026-08-12)

Deleted ids from the 100–118 pass. Surviving findings live in `_manifest-ffi.md`.

## 110 — Execute clears (and may partially refill) the answers carrier before the call can fail

Deleted. The original claim was that a failing `execute` / `execute_into` violates AGENTS.md §26 failure-transparency because the reusable carrier is wiped (or left partial) while the C++ dialect omitted the exception. That dialect gap is false: `Snapshot::execute_into` documents “the carrier is cleared first, capacity retained” (`cpp/src/db/snapshot.cc:103-106`), and the C ABI says the same (`cpp/foreign/bumbledb_c.h:647-649`). AGENTS.md §26 allows documented exceptions “in so many words.” Engine `execute_args` clears after `check_snapshot`; projection `finalize` truncates on error (`crates/bumbledb/src/api/prepared/finalize.rs:54-60`). Bind-failure wipe of a previous result set is the documented warm-path contract, not a bug.

## 118 — Inbound `bdb_string_view::as_str` fabricates an unbounded lifetime from a raw pointer

Deleted. `as_str` / `slice_in` do take a caller-chosen `'a`, but every consumer copies before returning to C: `value_in` does `to_vec()` (`cpp/bridge/src/value.rs:145-150`), `schema_spec_in` / field names `.into()` `String`s (`schema.rs`), `open_with` uses the `&str` only for `Path::new` during the call (`db.rs:300-310`), `program_in` copies the IR into an owned `Program` (`query.rs:406-434`). The header contract (“Views handed IN are copied before the call returns”) holds today. An unbounded helper lifetime with no current escape is a signature smell, not a live UAF.
