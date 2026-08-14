# bench-007: closure lane still teaches delta-variants and "one program"

- **Severity:** medium
- **Tree:** bench
- **Status:** OPEN
- **Source:** audit/bench.md F7
- **Depends on:** none (prose; parallel-safe). The `exec: None` skip at line 502 is engine-011 / engine-008 — not this id.

## The bug

`crates/bumbledb-bench/src/closure.rs` module doc (lines 1–9):

> driven through `Db::prepare` (`AtomSource::Interior`, the delta-variant plans, the finished-image slot)

Registry (lines 265–266): "two families, one program, two corpus shapes."

The query itself is a correct boundary `Query` (empty interiors, one rec, identity main). `InteriorId(0)` is C2 at the untrusted layer when interiors is empty — not a defect. The teaching around it is k-variant Program (engine-007's vocabulary).

## Why it's wrong

Insight 1: names are the representation readers execute. "Delta-variant plans" is the deleted k-variant mint (engine-007). "One program" is the deleted IR. The measured object is one Query; the comment says a program.

## The fix

Per `audit/CONTRACT.md` §C7: present-tense. One Query, two corpus shapes selected by anchor (depth vs fanout). Prepare through the reach pipeline — no "delta-variant." Keep `InteriorId(0)` and the rec construction. Do not touch line 502 (engine-011).

## Acceptance criteria

- [ ] Gone: `rg -in 'delta-variant|delta variant|one program' crates/bumbledb-bench/src/closure.rs` → no matches.
- [ ] The two-family registry still shares `closure_query`; the comment names one Query, two corpus shapes.
- [ ] Unchanged: closure answers, SQL (`CLOSURE_SQL`), and timing protocol. Line 502 skip remains until engine-008/011 land.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb-bench --lib closure`.

## Constraints

- Prose only. Do not retarget `InteriorId(0)`. Do not fold the profile skip into this commit.
