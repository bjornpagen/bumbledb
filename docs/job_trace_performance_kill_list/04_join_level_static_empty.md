# 04 Join-Level Static Empty Proof

Priority: P0

## Problem

The first kill-list round added per-atom static-empty proof. It catches queries where a literal-constrained atom has no rows at all. It does not catch queries where each literal atom exists individually, but the join of their constrained domains is empty.

`job_q01_top_production` is the key example:

- Runtime remains `Lftj`, not `StaticEmpty`.
- It is fast now (`145us`) but still does planning/dispatch/build/cache work.
- The query is empty only after intersecting movie sets from `MovieCompanies(company_type)` and `MovieInfoIdx(info_type)`.

## Technical Cause

Current static proof checks each literal atom independently:

`crates/bumbledb-lmdb/src/query.rs:1432-1460`

```rust
for atom in &query.atoms {
    if !atom.fields.iter().any(|field| matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_))) {
        continue;
    }
    ...
    for row in 0..relation.row_count {
        if static_atom_row_matches(...) {
            matched = true;
            break;
        }
    }
    if !matched {
        return Ok(true);
    }
}
```

This proves only `exists(atom) == false`. It cannot prove `exists(atom A join atom B) == false`.

## Required Solution

Add a join-level static-empty proof for no-input literal-constrained queries.

### Target Shape

Support the q01 pattern first:

```datalog
CompanyType(id: ?company_type, kind: "production companies")
InfoType(id: ?info_type, info: "top 250 rank")
MovieCompanies(movie: ?movie, company_type: ?company_type)
MovieInfoIdx(movie: ?movie, info_type: ?info_type)
Title(id: ?movie)
```

Proof plan:

1. Resolve `company_type` literal to small ID set.
2. Resolve `info_type` literal to small ID set.
3. Build or probe movie set for `MovieCompanies.company_type in set`.
4. Build or probe movie set for `MovieInfoIdx.info_type in set`.
5. Intersect movie sets.
6. If empty, return `StaticEmpty`.

### General Algorithm

For no-input queries:

- Identify literal-bound dimension atoms that bind an ID variable.
- Propagate resolved finite domains along equality variables.
- Identify a join variable constrained by two or more finite-domain fact atoms.
- Probe each fact atom into a sorted/hashed set of candidate values for the join variable.
- If the intersection is empty, return `StaticEmpty`.

Keep this conservative. If the shape is not recognized, fall back.

### Counters And Diagnostics

Add:

```rust
static_empty_atoms_checked: u64
static_empty_rows_scanned: u64
static_empty_join_proofs: u64
static_empty_join_values_compared: u64
```

Expose the proving relation/variable in trace fields.

## Strict Passing Criteria

- `job_q01_top_production` runtime becomes `StaticEmpty`.
- `job_q01_top_production` steady average drops from `~145us` to `<40us`.
- `job_q01_top_production` `plan_us=0`, `execute_us=0`, `lftj_build_us=0`.
- `job_q33_linked_series_companies` remains `StaticEmpty` and stays below `40us` after frontend cache work.
- No query result changes.

## Tests

- Two literal dimensions exist independently but their fact join variable sets do not intersect; query is `StaticEmpty`.
- Same shape with one shared join value does not short-circuit.
- Projected variables disable the proof unless result emptiness is certain.
- Aggregate count with no matching join returns same empty-row semantics as current benchmark expectations.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb static_empty --all-targets
cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --query job_q01_top_production --query job_q33_linked_series_companies --format json
```
