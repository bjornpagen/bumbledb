# engine-035: tests and comments still say "empty program", "multi-rule program", "degenerate program"

- **Severity:** low
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F35
- **Depends on:** none (prose; parallel-safe)

## The bug

Zombie "program" as the name of a Query across test prose and comments: `api/prepared/tests/statically_empty.rs:206-210`, `tests/folded.rs:251-254`, `tests/rules.rs:59,85`, `ir/validate/tests/rules.rs:1,230,262`, `tests/api.rs:1534-1538`, `tests/adversarial_ir.rs:545,654-775` (the `TooManyCtes`-must-not-return pin is GOOD — keep the regression pin, reframe the prose from "the CTE cap's ghost" to "deleted cap stays deleted"), `exec/wordmap/clear.rs:47` ("a non-recursive program cannot observe it" — the watermark comment; say "the watermark lives on the rec sink's seen-set; cq queries have no such sink").

## Why it's wrong

Comment vocabulary is the cheapest representation to fix and the most-read (Insight 1); every "program" trains the next contributor in the coordinate system all the high-severity findings are deleting.

## The fix

Mechanical prose sweep in the listed files (and any siblings a grep finds): "program" → "query" where it names a Query; "empty program" → "statically-empty query"; wordmap comment per above; adversarial framing per above. `TooManyCtes` assertions unchanged (the pin is the point). Rust `stats.rs` doc uses of "program" (e.g. "single-rule program") sweep too.

## Acceptance criteria

- [ ] Gone: `rg -inw 'program' crates/bumbledb/src crates/bumbledb/tests --glob '!**/target/**'` → no matches naming a Query (allowed survivors: none expected; if a genuine non-Query sense exists, list it in the commit message).
- [ ] Unchanged tests: prose-only; all green, zero assertion edits; `TooManyCtes` absence pin intact.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Prose only. Do not touch string literals that appear in rendered snapshots (those belong to engine-033's versioned change) — check each hit before editing.
