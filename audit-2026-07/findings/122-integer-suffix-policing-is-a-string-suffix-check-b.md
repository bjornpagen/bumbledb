## Integer-suffix policing is a string suffix check: bare hex refused, suffixed hex accepted — and the schema macro's grammar is exactly inverted

category: incoherence | severity: low | verdict: CONFIRMED | finder: query:crates

### Summary

`parse_int` in the `query!` macro classifies integer literals by raw string shape. The two `ends_with` suffix arms accept *any* token text unconditionally, while the digits-and-underscores shape check guards only the unsuffixed branch. The result is a branch-shaped grammar nobody designed: `0x10u64` and `0b101i64` are accepted and spliced verbatim into the emitted `Value`, while bare `0x10` is refused — with the message "unsupported integer suffix on `0x10`", though `x10` is not a suffix. Verification against the sibling schema macro makes it worse: `schema!`'s integer grammar is *exactly inverted* (radix prefixes honored, type suffixes refused), contradicting the "one notation, schema to query" doctrine stated in the renderer.

### Evidence

- `crates/bumbledb-query-macros/src/lib.rs:389-391` — `is_int_text` requires only a leading ASCII digit and no `.`, so `0x10`, `0x10u64`, `0b101i64` all pass into the classification ladder.
- `crates/bumbledb-query-macros/src/lib.rs:411-427` — the ladder: `text.ends_with("i64")` → signed, accepted; `text.ends_with("u64")` → unsigned, accepted; else digits/underscores only → accepted; else `fail(..., "unsupported integer suffix on `{text}` — the value types are u64 and i64")`. `0x10` falls into the last arm; `0x10u64` never reaches the shape check.
- `crates/bumbledb-query-macros/src/lib.rs:1237-1251` — `lit` splices the raw token text verbatim: `Value::U64(0x10u64)` compiles, so the suffixed-hex spelling validates end to end.
- `crates/bumbledb/src/ir/render.rs:487-491` — the statement renderer emits integers with plain `write!("{v}")` (decimal only), under the header comment at :475: "the statement renderer's value formats (one notation, schema to query)". A source spelling of `0x64u64` renders as `100`.
- `crates/bumbledb-macros/src/lib.rs:1228-1236` — the *schema* macro's `int_magnitude` honors `0x`/`0o`/`0b` radix prefixes ("the seam parses what rustc would have; type suffixes are not part of the grammar"): `from_str_radix("10u64", 10)` fails, so `schema!` accepts bare hex and refuses suffixes — the mirror image of `query!`.
- No test coverage: grep for "unsupported integer suffix" hits only the source; the query-macros crate has no tests directory.

### Failure scenario

`amount == 0x64u64` in `query!` compiles, validates, and executes, but any diagnostic or rendered surface prints `100` — a different spelling than the source (spelling-canonicality broken; the decimal output does still reparse). `amount == 0x64` is refused with an error about a suffix that is not there. Meanwhile the same user writing `schema!` finds the acceptance rule inverted: `0x64` fine, `100u64` refused. Per the project doctrine (representation over control flow; a special case handled by a branch that a better representation would erase), the grammar here is an emergent property of branch ordering rather than a decided representation.

### Suggested fix

Decide the radix rule once and represent it as one shape check: strip an optional trailing `u64`/`i64` first, then apply a single rule to the remaining digits — either digits-and-underscores only (uniform refusal of non-decimal, matching what the renderer can round-trip), or the rustc-lexable radix set that `bumbledb-macros::int_magnitude` already implements (uniform acceptance, aligning the two macros). Given render.rs's "one notation, schema to query" claim, the two macros should share the same answer; reusing `int_magnitude`'s prefix table (with the suffix stripped beforehand) erases the branch asymmetry and fixes the misleading error message in one move.
