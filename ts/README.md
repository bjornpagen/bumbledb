# @bjornpagen/bumbledb

This package is the TypeScript interface to the
[Bumbledb](https://github.com/bjornpagen/bumbledb) embedded relational
database. Schemas and queries are typed TypeScript values rather than SQL
strings, while storage, admitted instances, transactions, and query execution run in
the native engine.

Relation declarations describe their fields, and the statements passed to
`schema()` connect those fields into typed keys and references. Values remain
ordinary `bigint`, `string`, boolean, byte, and interval values; queries infer
their parameter and result types from how those values are used.

## Platform support

The TypeScript package currently ships a native binary for **darwin-arm64**
(macOS on Apple Silicon). The optional
`@bjornpagen/bumbledb-darwin-arm64` package is selected automatically during
installation. On another platform, importing the package returns an error
that identifies the running platform and the available binary.

## Install

```sh
pnpm add @bjornpagen/bumbledb
```

## Quick start

Declare relations, connect their fields with keys and references, write
records in a transaction, and query them with the typed builder. Parameters
and result rows are inferred, and a failed constraint check is returned as
structured data rather than thrown as an exception.

```ts
import { bool, closed, contained, Db, gt, type Infer, key, on, query, relation, schema, u64, v } from "@bjornpagen/bumbledb"

// A fixed set can carry typed columns as well as names.
// Its ID type is the union "DirectPass" | "JudgedPass" | "Failed".
const Kind = closed(
	"Kind",
	{ mastered: bool, rank: u64 },
	{
		DirectPass: { mastered: true, rank: 30n },
		JudgedPass: { mastered: true, rank: 20n },
		Failed: { mastered: false, rank: 10n }
	}
)

// Relations describe stored records.
// `u64.fresh` marks an engine-minted primary key.
const Attempt = relation("Attempt", { id: u64.fresh, kind: Kind.id })
const Certificate = relation("Certificate", { attempt: u64, kind: Kind.id })

// These statements declare the key and references. The final reference is
// conditional: a certificate may cite only a mastered kind.
const Review = schema("Review", { Kind, Attempt, Certificate }, [
	contained(on(Attempt, "kind"), on(Kind, "id")),
	key(Certificate, ["attempt"]),
	contained(on(Certificate, "attempt"), on(Attempt, "id")),
	contained(on(Certificate, "kind"), on(Kind.where({ mastered: true }), "id"))
])

const created = await Db.create("./review.db", Review)
if (created.tag !== "accepted") {
	throw new Error("create rejected")
}
const db = created.value

// All writes are checked together before the transaction commits. A fixed-set
// column takes its name, and a wrong string is rejected by TypeScript and
// again if an untyped value reaches the native boundary.
const result = db.write((tx) => {
	const id = tx.reserve(Attempt, "id", 1n).at(0n)!
	tx.insert(Attempt, [{ id, kind: "DirectPass" }])
	tx.insert(Certificate, [{ attempt: id, kind: "DirectPass" }])
})

// A failed constraint check is returned as typed data rather than thrown.
if (result.tag === "rejected") {
	for (const v of result.violations) {
		console.error(v.kind, v.canonical, v.facts)
	}
}

// v(R) creates a typed variable for each column. Reusing one across records
// creates the join, result rows follow the find keys, and parameters are typed
// from where they are used.
const certifiedAbove = query(Review).rule((r) => {
	const { attempt: a, kind: k } = v(Certificate)
	const { rank } = v(Kind)
	return r
		.match(Certificate, { attempt: a, kind: k })
		.match(Kind, { id: k, mastered: true, rank }) // reusing k at Kind.id creates the join
		.where(gt(rank, r.param("floor")))
		.find({ a, rank })
})

const prepared = db.prepare(certifiedAbove)
const rows = db.execute(prepared, { floor: 15n }) // rows: { a: bigint; rank: bigint }[]
console.log(rows)

// A store read is one callback. The instance is invalid when the callback
// returns; the witness is a clone and may escape.
db.read((instance) => {
	console.log(instance.generation, instance.execute(prepared, { floor: 15n }))
})

// Dispatch over the fixed set uses native `switch` narrowing.
// `satisfies never` checks that every possible name is handled.
function describe(kind: Infer<typeof Kind.id>): string {
	switch (kind) {
		case "DirectPass":
		case "JudgedPass":
			return `mastered, rank ${Kind.axioms[kind].rank}`
		case "Failed":
			return "not mastered"
		default:
			return kind satisfies never
	}
}
console.log(describe("JudgedPass")) // "mastered, rank 20"
```

Every `ts` fence in this README is extracted and type-checked against the
real surface by `test/readme.test.ts` — the examples cannot drift.

## Surface

The SDK translates TypeScript values directly into the engine's shared schema
and query representations.

- Fields use `bool`, `bytes`, `i64`, `u64`, `str`, `interval`, and
  `span`. `relation()` declares stored records, while `closed()` declares a
  fixed enum-like set whose values may carry typed columns. `Infer` exposes
  the resulting TypeScript value type.
- `schema()` accepts `key`, `contained`, `mirrors`, and `capacity`
  statements. `.where` makes a reference conditional, `within` sets a count
  or measurement range, and `weigh` chooses a numeric field or interval
  duration to measure.
- `Db.create` and `Db.open` manage embedded stores. Create returns an
  `Admission`. Reads are a synchronous callback `db.read((instance, witness) => …)` —
  the instance cannot escape; the witness may. Writes use `write` or
  `writeFrom(witness, …)` and may return `abandon(payload)` to roll back
  explicitly. `insert` and `delete` report how many submitted records
  changed the set, and `reserve` returns never-reused IDs.
- `query(S).rule(...)` builds typed queries. Reusing a variable created by
  `v(R)` joins records through that value. The builder supports named result
  rows, typed parameters, negation, comparisons, boolean conditions, set
  parameters, interval operations, aggregates, named intermediate results,
  and linear recursive reachability.
- `Db.exhume` opens a store without its original application schema and
  exposes its stored relation descriptions and records by name. The returned
  handle uses `using` so the exclusive store lock is released at scope exit.

## Cookbook

The engine cookbook's 32 modeling recipes are translated to the TypeScript API
in [COOKBOOK.md](./COOKBOOK.md). `test/cookbook-doc.test.ts` extracts and
type-checks the document's TypeScript examples, while
`test/cookbook.test.ts` opens every schema and prepares every query. The Rust
and TypeScript versions are also checked to ensure that they describe the same
schema.

## Architecture

The SDK is a typed interface to the native engine. Storage, transactions,
queries, constraints, performance results, and the Rust implementation are
documented in the
[Bumbledb repository](https://github.com/bjornpagen/bumbledb).

## License

0BSD
