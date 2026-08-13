# General-audit rebuttals (2026-08-12)

Findings that did not hold against source. Files deleted; ids not reused.

## 305 — Image decode reports scan ordinal as LMDB row id in `WrongFactWidth`

`fill_columns` is only called with `read::scan` / `read::scan_from` (`image/build.rs:243-251` and `:377-386`). Both store iterators run `parse_facts`, which calls `check_width(schema, rel, row_id, bytes)` with the `F`-key row id **before** the iterator yields (`storage/read/scan.rs:141-145`). A wrong-width fact therefore becomes `WrongFactWidth` with the real `row_id` at the scan boundary; `fill_columns`'s `entry?` fails before `decode_fact`. The image-path width check in `decode_fact` (`image/decode.rs:208-214`) is unreachable for LMDB data. Closed relations never take that fill: `build`/`append` debug-assert they are not closed, and `synthesize_closed` uses declaration index, which *is* the row id. `storage/read/tests.rs` (`corrupted_fact_width_is_an_error_never_a_skip`) already asserts scan reports the victim id. The discarded `_row_id` in `fill_columns` is unused because the ordinal is the image slab index, not the corruption label.

## 306 — C++ `Error::violations()` silently truncates a partial citation list

Dialect `Error::violations()` (`cpp/src/error.cc:250-267`) breaks when `handle_.violation(index)` is empty. That handle is `foreign::error_handle`. Its `violation` (`cpp/foreign/raii.cc:159-176`) returns `nullopt` only when `index >= violation_count()`; an in-range `bdb_error_get_violation` failure is `unreachable_boundary_state()` → `std::abort()`, not an empty optional. The C ABI counts `error.violations.len()` and `get`s the same vec (`cpp/bridge/src/error.rs:332-351`). Sequential `0..count` on a stable handle cannot observe a mid-list miss. The `break` is dead; the incomplete-set failure mode does not exist against this bridge.
