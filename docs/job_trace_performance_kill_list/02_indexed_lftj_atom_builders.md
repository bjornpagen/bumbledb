# 02 Indexed LFTJ Atom Builders

Priority: P0

## Problem

Cold LFTJ build time is still dominated by temp relation construction, not by final sorted trie construction. The current `build_lftj_sorted_trie` scans entire relation images, filters literals row-by-row, clones encoded bytes into temp column vectors, then builds a sorted trie.

Post-kill trace examples:

| Query | Cold `lftj.build` | Nested `sorted_trie.build` | Source Rows Scanned | Retained Temp Rows |
|---|---:|---:|---:|---:|
| `job_q16_character_title_us` | `27.3ms` | `2.49ms` | `251,652` | `117,483` |
| `job_q24_voice_keyword_actor` | `12.8ms` | `208us` | `144,170` | `10,001` |
| `job_q09_voice_us_actor` | `73.8ms` | partial `49.7ms` across planner/build | large | large |

The source scan/filter/copy work is the largest part of cold LFTJ build for q16/q24.

## Technical Cause

`build_lftj_sorted_trie` always loops all source rows:

`crates/bumbledb-lmdb/src/query.rs:3657-3666`

```rust
for row in 0..source.row_count {
    let row = RowId(row as u32);
    let Some(values) = atom_row_values(source, query, inputs, atom, row, variables)? else {
        continue;
    };
    included_rows += 1;
    for (column, bytes) in values.into_iter().enumerate() {
        raw_columns[column].push(bytes);
    }
}
```

Literal and input filters are applied by comparing every row:

`crates/bumbledb-lmdb/src/query.rs:3766-3816`

```rust
NormTerm::Literal(literal) => {
    if literal.as_bytes() != bytes {
        return Ok(None);
    }
}
```

This ignores available schema indexes such as:

- `Keyword.by_keyword`
- `CompanyName.by_country`
- `Name.by_gender`
- `Title.by_episode`
- `Title.by_year`
- `RoleType.by_role`

## Required Solution

Add indexed LFTJ atom builders that enumerate only candidate row IDs for bound literal/input prefixes and range predicates.

### Access Selection

For each atom, detect whether static fields or single-atom comparisons can be served by a leading access path.

Example:

```datalog
Keyword(id: ?keyword, keyword: "hero")
```

Should use `Keyword.by_keyword` to enumerate only rows under encoded literal `"hero"`.

Example:

```datalog
Title(id: ?movie, episode_nr: ?episode)
?episode >= 50
?episode < 100
```

Should use `Title.by_episode` range enumeration if the comparison is local to that atom.

### Candidate Row Iterator

Introduce:

```rust
enum AtomRowSource<'a> {
    Full { end: usize },
    IndexedPrefix { rows: Box<dyn Iterator<Item = RowId> + 'a> },
    IndexedRange { rows: Box<dyn Iterator<Item = RowId> + 'a> },
}
```

The initial implementation can use QueryImage sorted/hash trie caches to fetch row IDs. Later versions can use durable segment index row order directly.

### Build Path

Replace:

```rust
for row in 0..source.row_count { ... }
```

with:

```rust
for row in atom_row_source(...) { ... }
```

The remainder of `atom_row_values` can stay initially, but it will run over far fewer rows.

### Single-Atom Predicate Pushdown

Extend predicate depth metadata so atom-local range predicates can be consumed by the atom builder.

Safe criteria:

- Predicate references exactly one variable.
- That variable appears in the atom.
- The field binding for that variable is known in this atom.
- The predicate uses encoded-order-supported comparison.

## Strict Passing Criteria

- `job_q16_character_title_us` cold `lftj.build` drops from `~27.3ms` to `<5ms`.
- `job_q24_voice_keyword_actor` cold `lftj.build` drops from `~12.8ms` to `<3ms`.
- `atom_temp_relation_source_rows` for q16/q24 drops by at least `80%`.
- The trace shows explicit `lftj_atom.indexed_prefix` or equivalent diagnostics for literal-indexed atoms.
- No JOB query result changes.

## Tests

- Atom with leading literal uses indexed row source and scans fewer source rows.
- Atom with local range predicate uses range row source.
- Atom with unsupported predicate falls back to full scan.
- Repeated-variable atoms preserve equality semantics.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb lftj --all-targets
cargo test -p bumbledb-bench --all-targets
cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --format json
```
