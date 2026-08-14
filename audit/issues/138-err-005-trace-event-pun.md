# err-005: `TraceEvent` — `dur_ns == 0` ⇒ point event

- **Severity:** low
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F19
- **Depends on:** none
- **Conflicts with:** none

## The bug

`obs.rs:50-64` — "`dur_ns == 0` ⇒ point event." A duration pun (rec-as-`interiors.len()`). `a0`/`a1` are always present; names document unused as `-`. Chrome-trace *wire* is two args — that flattening is essential at export and is **not** this issue (do not sum-type the wire).

## Why it's wrong

Insight 3 — span vs point is a coordinate, not a zero duration. The Rust type matching the wire *before* export is the accidental half.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

```rust
enum TraceEvent {
    Span { name, cat, start_ns, dur_ns, args: (u64, u64) },
    Point { name, cat, start_ns, args: (u64, u64) },
}
```

Export writes `dur: 0` for Point. Args stay two `u64`s — the wire's two words (essential; do not invent per-name payload enums).

## Acceptance criteria

- [ ] Gone: `rg -n 'dur_ns == 0' crates/bumbledb/src/obs.rs` as the point-event definition (export may still write 0).
- [ ] Drain/finish_capture still emits Chrome-trace-compatible events; existing trace tests match on names/args, not on a Rust `dur_ns` pun.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb --features trace`; `./scripts/check.sh`.

## Constraints

- Zero-cost when `trace` is off (ZST guard) unchanged. Tick→ns conversion stays one drain site. `Category::Phase` still excluded from flame containment. Do not change `a0`/`a1` into a sum — wire shape is essential.
