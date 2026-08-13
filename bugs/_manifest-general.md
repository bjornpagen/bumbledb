# General correctness audit — finding manifest

Auditor range: 300–399. One bug per file. No INDEX.md.

| file | severity | confidence | one-line summary |
|---|---|---|---|
| 301-escaped-fresh-id-flush-swallowed.md | high | confirmed | Abort-path `flush_escaped_fresh_ids` errors are discarded, so `alloc()` ids can be reissued after a failed Q burn |
| 302-fresh-f-conflict-skips-other-keys.md | medium | confirmed | Occupied `F` on a fresh-keyed insert records only the auto-key and skips other Functionality keys |
| 303-query-macro-interval-literal-arity.md | medium | confirmed | `query!` emits `Value::IntervalU64(start, end)` but the variant takes `Interval<T>` — `start..end` does not compile |
| 304-commit-rejected-masked-by-decode.md | medium | confirmed | Citation decode uses `?` on a new read txn, so `CommitRejected` can become `ReadersFull` / `Corruption` |
| 305-wrong-fact-width-reports-scan-ordinal.md | low | confirmed | Image `WrongFactWidth.row_id` is the scan position, not the `F` key row id |
| 306-cpp-violations-silent-truncate.md | low | likely | C++ `Error::violations()` breaks on a mid-list empty fetch and returns a prefix with no error |
| 307-s-row-count-overflow-mislabeled.md | info | confirmed | `checked_add_signed` failure on `S` is always reported as “underflow” |

Counts: high 1, medium 3, low 2, info 1. Total 7.

Not filed (looked at, not bugs or not worth a file): intern `assert!` on `u64::MAX` (documented panic, `assert!` is release-live); row-id `+= 1` wrap at 2^64; dict forward `put` without `NO_OVERWRITE` (needs prior corruption); C++ schema `owned_literal_of` interval→string fallthrough (`where()` cannot bind interval fields today); exec `debug_assert` slot freshness (plan-validation invariant, not a user path).
