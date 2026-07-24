## Answers re-validates UTF-8 on every string-cell access despite validate-at-materialization

category: inelegance | severity: low | verdict: CONFIRMED | finder: engine:schema-api
outcome: fixed 7406bd74

### Summary

String payloads are UTF-8-validated exactly once when they enter the answer buffer, but the validation result is thrown away: the bytes land in an untyped shared `Vec<u8>` heap, so the consumption API `Answers::get` must re-run `std::str::from_utf8` — an O(len) scan — on every String-cell read, guarded by an `expect` whose message ("validated at materialization") documents its own redundancy. This is the exact pattern the project's own doctrine names as the failure mode: validation "checks a condition and throws away what it learned, so every caller downstream must check again, while parsing returns a type that carries the proof" (docs/design/representation-first.md:54-57). The cause is representational — proven-UTF-8 text and raw `bytes<N>` blobs share one heap, so the type system cannot remember the proof.

### Evidence

All verified against the code:

- `crates/bumbledb/src/api/prepared/answers.rs:58-60` — per-access rescan on the hot read path:
  ```rust
  Cell::String { start, len } => AnswerValue::String(
      std::str::from_utf8(&self.bytes[start..start + len])
          .expect("validated at materialization"),
  ),
  ```
  `Answer::get` (answers.rs:183-185) delegates here, so the `answers()` iterator pays it per cell too.
- `crates/bumbledb/src/api/prepared/resolve_memo.rs:50-53` — the one-time check whose result is discarded:
  ```rust
  std::str::from_utf8(raw)
      .map_err(|_| Error::Corruption(crate::error::CorruptionError::NonUtf8Intern(word)))?;
  let start = buffer.bytes.len();
  buffer.bytes.extend_from_slice(raw);
  ```
  The `Ok(&str)` is dropped; only the raw bytes survive.
- `crates/bumbledb/src/api/prepared.rs:122-127` — the single untyped heap: `Answers { arity: usize, cells: Vec<Cell>, bytes: Vec<u8> }`. `fixed_bytes_cell` (answers.rs:106-116) interleaves raw `bytes<N>` payloads into the same vec, which is why the heap cannot simply become a `String` as-is.
- Doctrine: docs/design/representation-first.md:54-57 (Alexis King's parse-don't-validate, cited by the repo itself) — the code violates its own spec.

Not a refutation, for the record: the codebase's other `expect("validated ...")` UTF-8 sites (api/db/get.rs:80, api/prepared/introspect.rs:426) validate and use in the same function — they are not per-access rescans of a proof established elsewhere.

### Bench impact

No wrong output — pure redundant work plus a panic path. A host consuming a large projection with string columns (`for a in answers.answers() { a.get(col) }`) pays a full UTF-8 scan of every string cell on every read. Because ResolveMemo copies each distinct string into the heap once but `get` rescans per *cell* (K answers sharing one memoized string are scanned K times, and re-scanned on repeated iteration), the read-side validation work can exceed the write-side validation that produced the buffer. For the perf lane this is a hidden O(len)-per-access cost on the API every host uses.

### Suggested fix

Split the heap into two typed carriers on `Answers`:
- `text: String` — String cells slice it. ResolveMemo keeps its existing check but keeps the parsed `&str` (`let s = std::str::from_utf8(raw).map_err(...)?; text.push_str(s);`). Every `(start, start+len)` offset is a char boundary by construction (whole validated strings appended end-to-end), so `&self.text[start..start + len]` in `get` is an O(1) bounds check with no validation, no `expect`, and no unsafe.
- `blob: Vec<u8>` — the existing FixedBytes path unchanged.

`String::clear` retains capacity exactly like `Vec<u8>::clear`, so the zero-alloc warm-reuse contract (answers.rs:14-18, prepared.rs:120-121 "the byte heap is the single sanctioned allocation site of a warm execution") survives as two capacity-retaining heaps. `byte_len()` (answers.rs:41-43) should report `text.len() + blob.len()` (or both separately) to keep the memory-observability contract of docs/architecture/40-execution.md.
