# 07 Static Atom Pre-Resolution And FK Atom Elimination

Priority: P1

Primary affected queries:

- `job_q01_top_production`: empty literal dimension branch found only after LFTJ builds all atom tries.
- `job_broad_movie_info_star`: dimension atoms add join work but carry no output.
- `job_q33_linked_series_companies`: literal dimension atoms and FK-implied existence atoms add planning/index work.
- `job_q16_character_title_us` and `job_q24_voice_keyword_actor`: literal dimension filters should be resolved before large hash index builds.

## Problem

Many JOB query atoms are dimension lookups or existence checks:

- `InfoType(info: "top 250 rank")`
- `CompanyType(kind: "production companies")`
- `Keyword(keyword: "hero")`
- `RoleType(role: "actor")`
- `CompanyName(id: ?company)` with no predicate or output payload
- `Title(id: ?movie)` used only to prove referenced movie existence

The current planner treats these as ordinary relation atoms. The runtime often builds general-purpose hash/sorted tries for them. In a typed no-null FK schema, many existence atoms are redundant. Literal dimension atoms can be resolved to IDs before the join.

## Trace Evidence

`job_q01_top_production`:

- Query is empty because one literal dimension branch yields no candidate in the loaded subset.
- LFTJ still builds all five atom tries.
- Cold `lftj_build_us=4.33ms`, `91.8%` of prepare.

`job_q16_character_title_us`:

- Literal `Keyword(keyword="character-name-in-title")` and `CompanyName(country_code="[us]")` are handled through hash tries.
- Cold builds include `Keyword 134,170` rows and `CompanyName 200,000` rows.

`job_q24_voice_keyword_actor`:

- Query dies after `2` probes.
- Downstream dimension/existence hash tries still built before probing.

## Current Technical Cause

Normalized relation atoms preserve all atoms. There is no simplification pass between normalization and planning that resolves static atoms, substitutes singleton bindings, or removes FK-implied atoms.

`execute_query` normalizes then immediately plans:

`crates/bumbledb-lmdb/src/query.rs:1220-1258`

```rust
let mut normalized = normalize_query(self, schema, query)?;
...
let mut plan = plan_query(schema, &mut normalized, image.as_ref(), query_image_cache)?;
```

`plan_query` considers all relation atoms:

`crates/bumbledb-lmdb/src/query.rs:3008-3016`

```rust
let relation_atoms = query.atoms.iter().collect::<Vec<_>>();
let comparisons = query.predicates.iter().collect::<Vec<_>>();
let stats = PlannerStats::collect(schema, image, &relation_atoms)?;
```

The current missing-index code can recommend indexes but does not simplify atoms:

`crates/bumbledb-lmdb/src/query.rs:3480-3529`

It sees static predicates and recommends leading indexes, but the query still executes the atom generically.

## Desired End State

Before physical planning, run a semantics-preserving simplification pass:

1. Resolve static literal atoms with indexed equality predicates.
2. If static atom has zero rows, replace query with an empty plan.
3. If static atom binds exactly one variable to one ID, substitute that encoded constant into all uses.
4. If static atom binds a small set, create an efficient finite-domain binding node or input-side temp relation.
5. Remove existence-only FK-implied atoms that cannot affect multiplicity or output.

## Proposed Technical Solution

Add `simplify_normalized_query` after input encoding and before `plan_query`.

```rust
let mut normalized = normalize_query(...)?;
let encoded_inputs = encode_inputs(...)?;
let image = self.query_images.get_or_build(...)?;
simplify_normalized_query(schema, image.as_ref(), &encoded_inputs, &mut normalized)?;
let mut plan = plan_query(...)?;
```

### Static Literal Atom Resolution

For each atom:

- Identify fields bound to literals or inputs.
- Find an access path whose leading fields are all bound.
- Probe existing relation/image index if available.
- Return matching row set or encoded variable bindings.

Cases:

```text
0 matches: query is statically empty.
1 match and exactly one unbound variable: substitute variable with literal encoded value.
N small matches: create a finite-domain constraint for variable.
N large matches: keep atom; generic runtime may be better.
```

### Constant Substitution

Introduce normalized constant bindings:

```rust
struct StaticBinding {
    variable: VarId,
    value: EncodedValue,
}
```

Apply substitutions to:

- Relation atom fields.
- Comparison operands.
- Output aggregate variables where safe.

If an output projects a substituted variable, output its constant value.

### FK Atom Elimination

An atom can be removed if:

- It has only `id`/primary key field bound to a variable.
- It has no static predicates.
- It contributes no projected or aggregate variable not already bound elsewhere.
- Its existence is guaranteed by a foreign-key relation atom that binds the same variable.

Example:

```datalog
MovieKeyword(movie: ?movie, keyword: ?keyword)
Keyword(id: ?keyword)
```

If `MovieKeyword.keyword` is a typed ref to `Keyword`, then `Keyword(id: ?keyword)` adds no filter and no multiplicity. It can be removed.

The JOB schema now models these as refs, so this is valid for loaded BumbleDB data.

### Multiplicity Rules

Removal must not change aggregate multiplicity. Entity primary-key atoms with exactly one row per ID are safe to eliminate as existence checks. Relations with non-unique keys are not safe unless uniqueness is known.

Use schema primary key and relation kind:

- `RelationDescriptor.primary_key == ["id"]` and field is `id`.
- Referencing field is a typed `ValueType::Ref` to that relation.

### Empty Query Plan

If simplification proves empty, skip planning/execution and return an empty aggregate/project output directly.

For aggregate count with current semantics, note that SQL `HAVING COUNT(*) > 0` comparisons in the benchmark mean zero-row aggregate queries return zero rows. Preserve current BumbleDB behavior for now.

## Implementation Plan

1. Add `StaticQuerySimplifier` module in `query.rs` or a new `query_simplify.rs`.
2. Implement static atom analysis and index lookup for single-row dimension atoms.
3. Implement empty-query short-circuit.
4. Implement constant substitution.
5. Implement FK existence atom elimination.
6. Add diagnostics: `static_atoms_resolved`, `static_empty_short_circuits`, `fk_atoms_eliminated`.
7. Teach planner/benchmark output to report simplification diagnostics.

## Tests

- Empty literal dimension atom short-circuits without building LFTJ/hash indexes.
- Singleton dimension atom substitutes ID and removes the dimension atom.
- FK existence atom removal preserves results.
- Non-FK or non-primary relation atom is not removed.
- Aggregate multiplicity remains unchanged.
- Queries with projected dimension fields do not remove needed atoms.

## Acceptance Criteria

- `job_q01_top_production` cold LFTJ build drops sharply when literal `InfoType` is empty.
- `job_broad_movie_info_star` atom count reduces by eliminating pure dimension existence atoms where FK-safe.
- `job_q16` and `job_q24` do not build full `Keyword`/`RoleType` dimension indexes for singleton literal lookups.
- Simplification diagnostics are visible in trace/profile output.

## Risks

- Incorrect FK elimination can silently change query semantics.
- Constant substitution must preserve Datalog type information.
- Queries that project dimension fields or rely on aggregate multiplicity need conservative handling.

## Rollout Plan

1. Start with empty literal atom short-circuit only.
2. Add singleton literal substitution for ID variables.
3. Add FK existence atom elimination for simple primary-key entity atoms.
4. Extend to small finite domains.
5. Re-run JOB trace and compare atom counts, build rows, and output equality.
