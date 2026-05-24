# PRD 06: Durable Value Accelerators

## Purpose

Add correctness-optional durable value accelerators so equality filters and common join keys can retrieve candidate row IDs without scanning full relations.

## Rosetta Alignment

Accelerators are private physical structures. They must never be public schema semantics and must never be required for correctness. If an accelerator is absent or disabled, the engine must fall back to column scans and return identical results.

## Paper Alignment

The paper's discussion says future work must make Free Join use existing indices. This PRD introduces LMDB-backed indices while preserving the GHT/COLT interface.

## Required Accelerator

Maintain this logical key shape for every indexed field:

```text
A | relation_id | field_id | encoded_value | row_id -> empty
```

If v6 keeps fact handles instead of row IDs, use fact handles only until row IDs exist. Do not let that temporary choice leak past PRD 05 completion.

## Required Index Coverage

Initial automatic coverage:

- all generated serial `id` fields;
- all foreign-key serial fields;
- all enum fields;
- all string fields used in JOB filters;
- all `I64` fields used in JOB range filters may use separate ordered accelerators if equality layout is insufficient.

Aggressive option:

- maintain equality accelerators for every fixed-width persistent field by default.

No runtime DDL or user-declared index API is allowed in this PRD.

## Write Semantics

Insert must update accelerators atomically with canonical fact, live row, columns, stats, unique guards, and reverse FK guards.

Delete must remove accelerator entries atomically.

Duplicate insert must not change accelerators.

Absent delete must not change accelerators.

Failed writes must leave no partial accelerator entries.

## Read Semantics

Accelerator lookup returns candidate row IDs or handles visible to the current LMDB snapshot.

The query path must verify candidates against source predicates unless the accelerator maintenance proof makes false positives impossible.

False positives are allowed only if they are filtered before COLT emits tuples.

False negatives are forbidden.

## Tests Required

- Insert creates accelerator entries.
- Delete removes accelerator entries.
- Duplicate insert is a no-op for accelerator count.
- Absent delete is a no-op for accelerator count.
- Failed write rolls back accelerator writes.
- Query results are identical with accelerators enabled and disabled.
- String filter dictionary lookup plus accelerator returns expected candidates.
- Serial FK accelerator supports fact-relation narrowing.

## Trace Requirements

Add counters:

- accelerator lookups
- accelerator hits
- accelerator misses
- accelerator candidate rows
- accelerator false positives
- accelerator fallback scans

## Benchmark Passing Criteria

Run full traced JOB sample with accelerators enabled and with accelerators disabled.

Required evidence:

- Exact SQLite comparisons pass in both modes.
- Filtered queries use accelerators for `CompanyName.country_code`, `Name.gender`, `RoleType.role`, `Keyword.keyword`, `KindType.kind`, and `LinkType.link` where applicable.
- Candidate rows for selective filters are materially smaller than live relation rows.
- Total `BaseImageLoad` and `colt_offsets_scanned` improve on filtered queries.
