# exec-005: `carried_col` is an Option-padded reverse index of `carried`

- **Severity:** medium
- **Tree:** exec
- **Status:** OPEN
- **Source:** audit/plan-exec.md F8
- **Depends on:** exec-004 (same `PipeTables`; land with or after)

## The bug

`crates/bumbledb/src/exec/run.rs:687-692` and `run/pipe_tables.rs:27-43`:

```rust
carried: Vec<Vec<usize>>,              // occs this node carries
carried_col: Vec<Vec<Option<usize>>>,  // dense in occ-id; None = not carried
```

Construction fills `cols = vec![None; n_occ]` then `cols[occ] = Some(occs.len())` for the sparse carried set. Readers (`pump.rs:81,122`; `probe_pass.rs:333,417`) `match tables.carried_col[node_idx][occ]`. Two encodings of one sparse set; None in every non-carried slot.

## Why it's wrong

Hoare: null in every occurrence slot (Insight 5). Dijkstra: the empty range is the empty list, not a vec of Nones. `carried` already *is* the list; `carried_col` is a reverse index that reintroduces absence as Option.

## The fix

Per `audit/CONTRACT.md` §C1:

- Keep `carried: Vec<Vec<usize>>` (occs in column order).
- Replace `carried_col` with a dense map *over that list* (column → occ is `carried[node][col]`; occ → column is a small search over the — tiny — carried list, or a `Vec<(occ, col)>` built once). Not `Vec<Option>` sized to all occurrences including those that never appear.
- Readers match "is this occ in `carried[node]`" without a None hole.

## Acceptance criteria

- [ ] Gone: `rg -n 'carried_col' crates/bumbledb/src/exec` → no matches; `rg -n 'vec!\[None; n_occ\]' crates/bumbledb/src/exec/run` → no matches.
- [ ] Unchanged tests: pipeline / carried-cursor suites green; answers identical.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Carried-cursor routing semantics identical. Land with exec-004 if `Drive::Pipeline(PipeTables)` is being reshaped anyway.
