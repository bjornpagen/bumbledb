# 29 — The hot-path allocation hunt

- **Status:** **fixed this pass** — every lead site classified (fix or
  rule). Per-fact `Checker` copies of `self.key` / U values / fresh-row
  facts deleted; remaining catalog GAT copies ruled. Gate: 28's
  `alloc_law_budgets` plus `storage::commit::tests` (151).
- **Severity:** performance.

## Principle

Insight 15: a hot-path allocation is a branch's cost profile wearing a
data costume — free to write, paid on every row forever. The budgets (28)
make each one a red test instead of a review opinion.

## Leads (indicative census, 2026-08-19 — verdicts come from the budgets,
not the grep)

- `exec/run/execute.rs` + `exec/colt.rs`: 26 `Vec::new()` / `to_vec()` /
  `.clone()` sites — classify each as construction-time (fine),
  cold-arm (fine, comment it), or per-row (fix).
- `storage/commit/judgment.rs`: 12 sites — same classification; judgment
  is per-fact hot.
- `api/prepared/bind.rs`: 1 site.
- `Binding::Bound { filters: Vec<FilterPredicate> }` movement on
  park/unpark (fresh from fix 07) — confirm park is a move, never a clone.
- `run_join` epilogue / finalize: per-answer `dict_resolve` into `Answers`
  — `Answers` retains capacity by design; confirm no per-row intermediate.
- The intern-resolver closure environment (24) — confirm the genericized
  form captures by reference, no boxed state.

## Protocol per offender

Fix it (reuse a scratch, hoist to construction, move instead of clone) or
**rule it** with a one-line recorded reason at the site — the same
discipline the codebase already uses. No third option; every site in the
lead list gets one of the two.

## Acceptance

- All 28 budgets green.
- Every lead site either changed or carries its recorded ruling.
- Scenario lanes byte-identical answers; warm p50s not worse (attribution
  note per lane in the commit body).

## Classifications

### `exec/run/execute.rs` + `exec/colt.rs` (26 + 0)

All 26 `Vec::new` / `.clone()` sites in `execute.rs` are
**construction-time** (`Executor::with_batch_size` / `NodePrecompute::of`).
Warm `execute` clears retained-capacity pools. Block comment at the
scratch table; the four `FilterPredicate` clones are construction-time
(31 extracts sides so the clones die).

`colt.rs`: **ruled** — refill/advance truncate to a `PoolMark` and
reuse; no `Vec::new` / `to_vec` / `.clone()` on those paths.

### `storage/commit/judgment.rs`

| Site | Verdict |
| --- | --- |
| `violations = Vec::new` | construction-time (one collector / commit) |
| `FieldCheck` `bytes.clone` / `alternatives.clone` / `Box::new(encode_u64)` | construction-time (`Selections::encode`) |
| worklist `collect` | construction-time (already: one exact sort alloc / commit) |
| `membership.check.clone` | construction-time (plan-owned `Check`; closed-target, not the edge loop) |
| `closed_source_survivor` `row.fact.clone` | cold-arm (violation payload) |
| `Probe::unsatisfied` / capacity conviction `into()` | cold-arm (citation payload) |
| `check_scalar` U `to_vec` | **fixed** — `decode_row_id` copies the word; GAT lives long enough |
| `fresh_row_fact` `to_vec` | **fixed** — scalar arm `check_fact`s the GAT; capacity fills `parent_scratch` |
| `row_fact` `to_vec` | **fixed** — `load_row` into retained `fact_scratch` / `parent_scratch` |
| coverage `seek`/`group` `to_vec` | **fixed** — already in `self.key` |
| capacity `prefix_owned` `to_vec` | **fixed** — already in `self.key` |
| `locate_coverage_entry` / `collect_coverage_segments` `to_vec` | **ruled** — catalog GAT ends at the next get; `Continuation<_,_,Vec<u8>>` owns segment values |

### `api/prepared/bind.rs` (1)

`_ => Vec::new()` in `bind_set_slot`: **construction-time** — first
set-bind constructs the pool; warm re-bind hits the `WordSet` arm.

### `Binding::Bound` park/unpark

**Ruled (confirm):** park/unpark is `mem::swap` / `mem::replace` of
`Bound` (a move). First bind of `Unbound` does `filters.to_vec()` inside
the sanctioned view-rebuild window (`view_memo.rs:set_bound`). File is
not this lane's; the moves are at `view_memo.rs` bind.

### finalize / `dict_resolve`

**Ruled (confirm):** `ResolveMemo::resolve` hits the persistent text
arena after the first distinct intern; `Answers::begin` retains
capacity. No per-row intermediate. File is not this lane's.

### Intern-resolver captures (24)

**Ruled (confirm):** the three `F: FnMut` monomorphs capture `delta` /
`view` / `stage` by reference. No boxed resolver state.
