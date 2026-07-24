## Three divergent integer-literal parsers in one grammar: radix/underscores position-dependent

category: unification | severity: low | verdict: CONFIRMED | finder: macros:core
outcome: fixed 22b89f3f (R8)

### Summary

`crates/bumbledb-macros/src/lib.rs` judges the same integer-literal token shape three different ways depending on grammatical position. Selection/row literals go through `int_magnitude`, which strips underscores and honors `0x`/`0o`/`0b` radix prefixes ("the seam parses what rustc would have"). Window bounds strip underscores but reject radix. `bytes<N>` and `interval<E, w>` widths reject both. So `| n == 0x10` and `| n == 1_000` are legal while `bytes<0x20>`, `bytes<3_2>`, `interval<u64, 1_0>`, and `<={0x2..0x4}` all panic as "malformed", for literals rustc itself lexed happily. The file's own doc comment on `parse_int` promises the unified behavior the ad-hoc sites fail to deliver.

### Evidence

All verified in `crates/bumbledb-macros/src/lib.rs`:

- **lib.rs:412-414** — `bytes<N>` width: `let width: u64 = text.parse().unwrap_or_else(|_| panic!("schema!: malformed bytes<N> width `{text}`"))`. Decimal-only; no underscore stripping, no radix.
- **lib.rs:497-499** — interval width: identical bare `text.parse()` shape ("malformed interval width").
- **lib.rs:907-916** — `parse_window_bound`: `text.replace('_', "").parse()` — a third dialect: underscores yes, radix no.
- **lib.rs:1228-1237** — `int_magnitude`: underscores stripped, `0x/0o/0b` dispatched to `u128::from_str_radix`; doc-commented "the seam parses what rustc would have". Row/selection literals route here via `u64_text`/`i64_text` at lib.rs:1155-1192.
- **lib.rs:644-646** — `parse_int`'s doc comment states the intended design outright: "returning the sign and the raw token text — range and **radix are judged at the token→`Value` seam**, against the field's declared type." All three weak sites call `parse_int` (lines 409, 492, 908) and then judge the text themselves instead of deferring to the seam — the code diverges from its own stated contract.
- **lib.rs:640-642** — `is_int_text` (first char is an ASCII digit, no `.`) passes `0x20` and `3_2` through `parse_int`, so these tokens reach the weak parsers rather than being rejected at tokenization; the failure is a misleading "malformed" panic.
- No test pins the decimal-only behavior: grep for "malformed bytes"/"malformed window bound" across the repo hits only lib.rs itself. Nothing in docs/architecture/10-data-model.md or 70-api.md (the `bytes<N>` grammar, N ∈ 1..=64) declares widths decimal-only.

### Failure scenario

`schema! { ... payload: bytes<0x20>, ... }` or `bytes<3_2>` panics at expansion with "malformed bytes<N> width" even though `| n == 0x20` and `| n == 3_2` are accepted in a selection two lines away. Same for `interval<u64, 1_0>` and window bounds `<={0x2..0x4}`. Not a wrong-output bug — the grammar's integer story is position-dependent, and the "malformed" message is wrong for literals that rustc lexed as well-formed integers.

### Suggested fix

One mechanism already exists. Route all three ad-hoc sites through `int_magnitude` (via `u64_text`), then apply their local range checks (width ≥ 1, N ≤ 64, bound is a count):

- lib.rs:412-414 and lib.rs:497-499: replace `text.parse()` with `u64_text(&text)`.
- lib.rs:907-916: replace `text.replace('_', "").parse()` with `u64_text(&text)`, deleting `parse_window_bound`'s private underscore handling.

This deletes two weaker re-implementations and restores the contract `parse_int`'s doc comment already claims — representation-first: one token→magnitude seam, not three.
