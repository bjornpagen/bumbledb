# docs-017: 70-api teaches `PreparedQuery::predicate()` — "the predicate the query defines"

- **Severity:** high
- **Tree:** docs
- **Status:** OPEN
- **Lane note (`audit/docs`, 2026-08-14):** blocked until engine-041 lands on another branch. Left OPEN; `PreparedQuery::predicate()` is unchanged.
- **Source:** audit/docs.md F17
- **Depends on:** engine-041 (`Predicate` → `Signature`, `predicate()` → `signature()`) — the doc cites the method by name

## The bug

`docs/architecture/70-api.md:760` — "column metadata via `PreparedQuery::predicate()` — the predicate the query defines (`20-query-ir.md` § the query shape) is the **buffer-typing authority**".

## Why it's wrong

Dual vocabulary at the embedding API (Insight 1): the sealed main SIGNATURE is spelled "predicate" both as the method name and as the teaching sentence; docs-002 fixes the referenced section, but this sentence re-teaches the word at the API surface where SDK authors copy it.

## The fix

Per `audit/CONTRACT.md §C7` + the §C3 signature-naming amendment: after engine-041 lands, "column metadata via `PreparedQuery::signature()` — the sealed main signature (answer columns + folds) is the buffer-typing authority." One coordinated change with the rename so doc and code never disagree.

## Acceptance criteria

- [ ] Gone: `rg -n 'predicate\(\)|the predicate the query defines' docs/architecture/70-api.md` → no matches.
- [ ] Buffer-typing-authority claim unchanged; the cross-reference to `20-query-ir.md` points at docs-002's rewritten section.

## Constraints

- Blocked by engine-041; lands in or immediately after its commit. Prose only.
