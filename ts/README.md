# @bjornpagen/bumbledb

Type-theoretic TypeScript SDK for the [bumbledb](https://github.com/bjornpagen/bumbledb) embedded relational engine.

bumbledb models data as relations judged by statements (functionality, containment, cardinality) and queried with Datalog expressed as plain values — no SQL, no query-string parser. The SDK is a thin, fully typed surface over an in-process native engine (LMDB storage, MVCC snapshots, a single-writer witnessed write loop).

The surface is structural to the bone. Relation declarations are pure structure — kind, width, element, fresh, nothing else — and domains are never declared anywhere: **the laws type the columns**. `schema()` computes every field's equivalence class from the statement list itself, so the containments and mirrors you already write ARE the typing, at compile time and again at construction. Values stay bare (`bigint`, `string`, …); identity lives in the class the laws compute, not in a wrapper.

> **Research-grade, one platform.** This is a `0.x` release of an embedded engine under active development. It targets a single platform today (below), the API is not yet frozen across `0.x`, and the FFI ABI is pinned exactly per version. Treat it as an early adopter's tool, not a production datastore.

## Platform support

This release targets **darwin-arm64 (macOS Apple Silicon) only**. The native binary ships as the optional platform package `@bjornpagen/bumbledb-darwin-arm64`, resolved automatically at install on a matching host. Installs on other platforms succeed (the main package is pure JS) but throw a typed, actionable error at first load naming the running platform and that only `darwin-arm64` ships today. More targets are pure addition — one more `os`/`cpu`-gated package plus a CI matrix — not a redesign.

## Install

```sh
pnpm add @bjornpagen/bumbledb
```

## Quick start

Declare relations as pure structure, let the statement list type every column,
write facts through a transaction, and query with Datalog as values.
Everything is typed end to end — bare structural values in law-computed
classes, inferred query rows, and rejections that arrive as data rather than
exceptions.

```ts
import { bool, closed, contained, Db, gt, key, on, query, relation, schema, u64 } from "@bjornpagen/bumbledb"

// A closed relation: a sealed roster of axioms with typed payload columns.
const Kind = closed(
	"Kind",
	{ mastered: bool, rank: u64 },
	{
		DirectPass: { mastered: true, rank: 30n },
		JudgedPass: { mastered: true, rank: 20n },
		Failed: { mastered: false, rank: 10n }
	}
)

// Relations are pure structure — no domain is declared anywhere.
// `u64.fresh` marks an engine-minted primary key.
const Attempt = relation("Attempt", { id: u64.fresh, kind: Kind.id })
const Certificate = relation("Certificate", { attempt: u64, kind: Kind.id })

// THE LAWS TYPE THE COLUMNS: schema() computes every field's class FROM this
// statement list — the containments are the typing. The last statement uses
// ψ-selection: a certificate may only ever cite a mastered kind.
const Review = schema("Review", { Kind, Attempt, Certificate }, [
	contained(on(Attempt, "kind"), on(Kind, "id")),
	key(Certificate, ["attempt"]),
	contained(on(Certificate, "attempt"), on(Attempt, "id")),
	contained(on(Certificate, "kind"), on(Kind.where({ mastered: true }), "id"))
])

const db = await Db.create("./review.db", Review)

// Write. The delta is judged against every statement at commit.
const result = db.write((tx) => {
	const attempt = tx.insert(Attempt, { kind: Kind.DirectPass }) // attempt.id minted, a bare bigint
	tx.insert(Certificate, { attempt: attempt.id, kind: Kind.DirectPass })
})

// Rejection-as-data: no throw — a rejected commit is a typed value carrying
// every violated statement, cited once, with its canonical spelling and facts.
if (!result.ok) {
	for (const v of result.violations) {
		console.error(v.kind, v.canonical, v.facts)
	}
}

// Query: Datalog as values. Vars are named and typed by the class of their
// first binding; params are typed by use; rows are typed from the select.
// `gt` is one of the free comparison exports.
const certifiedAbove = query(Review).rule((r) => {
	const { a, k, rank } = r.vars("a", "k", "rank")
	return r
		.match(Certificate, { attempt: a, kind: k })
		.match(Kind, { id: k, mastered: true, rank }) // ψ on the read side too
		.where(gt(rank, r.param("floor")))
		.select("a", "rank")
})

const prepared = db.prepare(certifiedAbove)
const rows = db.execute(prepared, { floor: 15n }) // rows: { a: bigint; rank: bigint }[]
console.log(rows)

// Host dispatch over the sealed roster — exhaustive by construction; each
// arm receives its axiom row.
const label = Kind.match(Kind.JudgedPass, {
	DirectPass: (row) => `mastered, rank ${row.rank}`,
	JudgedPass: (row) => `mastered, rank ${row.rank}`,
	Failed: () => "not mastered"
})
console.log(label) // "mastered, rank 20"
```

Every `ts` fence in this README is extracted and type-checked against the
real surface by `test/readme.test.ts` — the examples cannot drift.

## Surface

- The structural type kernel — fields as pure structure (`bool`, `bytes`, `i64`, `u64`, `str`, `interval`, `span`), `relation()`, and `closed()` sealed rosters with typed axiom payloads and exhaustive host dispatch via `.match`. Domains are never declared: `schema()` computes every field's class from the statement list.
- The statement algebra — `schema()`, `key`, `contained`, `mirrors`, `window`; faces via `on`/`oneOf`; counts via `exactly`, `atLeast`, `atMost`, `between`, `none`; ψ-selection via `.where` on relations and closed rosters.
- The `Db` runtime — `Db.create`/`Db.open`, path-cached stores, transactions, typed violations, scoped snapshot reads, the witnessed write loop with `abandon`.
- The query surface — Datalog as values, `query(S).rule(r => ...)`: named vars, params typed by use, negation, aggregates, and the free comparison/connective exports (`eq`, `ne`, `lt`, `le`, `gt`, `ge`, `and`, `or`, `not`, `allen`/`ALLEN`, `pointIn`, `covers`); stratified recursion via `program()`; `db.prepare` as a plain value.
- The exhume surface — `Db.exhume`, the schema-independent read path: a store's self-described shapes and raw facts by name, with typed refusals (`ErrExhumeNoDescriptor`, `ErrExhumeFormatMismatch`, `ErrExhumeCorruption`).

## Cookbook

The engine cookbook's 29 modeling recipes, translated to this SDK's structural API: [COOKBOOK.md](./COOKBOOK.md). Two referees hold it: `test/cookbook-doc.test.ts` extracts the document's own `ts` fences and type-checks them against the real surface (the doc itself cannot drift), and `test/cookbook.test.ts` runs compiled copies of the recipes — each schema admitted by the real engine, its fingerprint asserted against the cross-host goldens the Rust cookbook suite also pins, every query snippet lowered through `db.prepare`.

## Architecture

The SDK is a typed surface over the native engine; the model (relations,
statement-based judgment, Datalog evaluation, MVCC storage, the witnessed
write loop) is documented in the [bumbledb engine repository](https://github.com/bjornpagen/bumbledb).

## License

0BSD
