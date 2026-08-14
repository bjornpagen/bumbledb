# engine-028: `derived_count` / rec-id restated as arithmetic at every layer instead of stored once

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F28
- **Depends on:** engine-005 (the witness is where the values are stored)

## The bug

`interiors.len() + usize::from(rec.is_some())` is computed independently as: the validator's address space (`ir/validate.rs:297-299` — `derived_count: usize` on `InteriorSignatures`, documented as that formula), the execution bind (`reach.rs:133-134`), the rec id (`reach.rs:303`, `build.rs:373-375`), the render id (`render.rs:153`), the naive set index (`naive/query.rs:269`), the SQL CTE id (`translate/reach.rs:43`). The overflow proof is judged once (`InteriorIdOverflow`, `error.rs:848-853`) and re-`expect`ed twice at prepare:

```rust
// build.rs:374
u32::try_from(witness.interiors().len()).expect("overflow judged at validate"),
// introspect.rs:384
u32::try_from(i).expect("InteriorIdOverflow screened at validate")
```

## Why it's wrong

A derived fact recomputed at N sites is N chances to drift and N readers who must know the formula (Insight 9); an `expect` quoting the validator's error name is the proof being *narrated* instead of carried (Insight 6). This is engine-003's coordinate observed at the count rather than the id.

## The fix

Per `audit/CONTRACT.md §C2`: the witness stores, at validate time (where the overflow check already runs): `derived_count: u32` and — on the Reach arm (engine-005) — `rec_id: InteriorId`. Prepare copies both into the pipeline; execute/render/introspect read the stored values; the `expect`s delete. Bench oracles read the boundary object and may compute the boundary formula ONCE per entry (engine-019/021 own their sites).

## Acceptance criteria

- [ ] Stored once: `rg -n 'usize::from\(.*is_some|len\(\) \+ usize::from' crates/bumbledb/src` → no matches; `rg -n 'expect\("overflow judged at validate"\)|expect\("InteriorIdOverflow' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: `InteriorIdOverflow` adversarial test UNCHANGED; all suites green.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- u32 id width locked (id-width, not a product cap — no new caps). Lands with engine-005.
