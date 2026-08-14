# exec-002: `DedupRegime` is parsed at construction, then discarded into four independent fields

- **Severity:** high
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F2
- **Depends on:** none (sink state; rides exec-001 if `union_span` moves with `AggSpec`)

## The bug

`crates/bumbledb/src/exec/sink/aggregate/new.rs:195-275` already has the sum (`DedupRegime::Bindings | Union | DnfUnion | Elided`). `build` immediately flattens it onto `AggregateSink` (`exec/sink.rs:328-408`):

```rust
distinct_witness: Option<DistinctWitness>,
seen: Option<WordMap<()>>,
union_spans: Option<Vec<(usize, usize)>>,
dnf_rekey: bool,   // "Meaningless without union_spans"
```

`aim` (`new.rs:321-330`) re-tests `self.dnf_rekey` beside `union_spans.as_mut()`. `distinct_seen` `debug_assert`s `seen.is_none() == distinct_witness.is_some()`. Representable: DNF rekey with no spans, a witness *and* a seen-set, elision on a multi-rule sink.

## Why it's wrong

Insight 6: the constructor parsed the regime and threw the proof away; every method re-derives which arm it is from a bool-and-Option product (Insight 4). The debug_assert is the typechecker. R2's four regimes are real; four independent fields are the packaging leftover.

## The fix

Per `audit/CONTRACT.md` §C1 (trusted layer is a sum) and the R2 split already named in `DedupRegime`:

```rust
enum DedupState {
    Bindings { seen: WordMap<()> },
    Union { seen: WordMap<()>, spans: Vec<(usize, usize)> },    // head projection
    DnfUnion { seen: WordMap<()>, spans: Vec<(usize, usize)> }, // VarId-ordered slots
    Elided { witness: DistinctWitness },
}
```

- `aim` matches the union arms; DNF vs head is which arm, not `dnf_rekey`.
- `distinct_seen` is `Some` on every arm but `Elided`. The debug_assert deletes.
- Constructors (`with_capacity_hint` / `for_union` / `for_dnf_union` / `without_seen_set`) stay; they mint the arm directly.

## Acceptance criteria

- [ ] Gone: `rg -nw 'dnf_rekey' crates/bumbledb/src/exec` → no matches; `rg -n 'distinct_witness: Option' crates/bumbledb/src/exec/sink.rs` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb` green; elision observable (`seen_elided`) and DNF rekey tests (`dnf_rekey_transparent` suite) pass with zero assertion edits.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`. R2 semantics locked (hand-written union keys the head; DNF keys VarId-ordered slots; elision only under `DistinctWitness`).

## Constraints

- Observable behavior identical: spanning seen-set still reset once per execution, never per rule; `for_union` / `for_dnf_union` / `without_seen_set` names and signatures stay. Lands after or with exec-001 if `union_span` matches `AggSpec`.
