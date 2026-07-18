# @bjornpagen/bumbledb

Type-theoretic TypeScript SDK for the [bumbledb](https://github.com/bjornpagen/bumbledb) embedded relational engine.

bumbledb models data as relations judged by statements (functionality, containment, cardinality) and queried with Datalog expressed as plain values — no SQL, no query-string parser. The SDK is a thin, fully typed surface over an in-process native engine (LMDB storage, MVCC snapshots, a single-writer witnessed write loop).

> **Research-grade, one platform.** This is a `0.x` release of an embedded engine under active development. It targets a single platform today (below), the API is not yet frozen across `0.x`, and the FFI ABI is pinned exactly per version. Treat it as an early adopter's tool, not a production datastore.

## Platform support

This release targets **darwin-arm64 (macOS Apple Silicon) only**. The native binary ships as the optional platform package `@bjornpagen/bumbledb-darwin-arm64`, resolved automatically at install on a matching host. Installs on other platforms succeed (the main package is pure JS) but throw a typed, actionable error at first load naming the running platform and that only `darwin-arm64` ships today. More targets are pure addition — one more `os`/`cpu`-gated package plus a CI matrix — not a redesign.

## Install

```sh
pnpm add @bjornpagen/bumbledb
```

## Quick start

Declare a schema, write facts through a transaction, and query with Datalog as
values. Everything is typed end to end — branded ids, inferred query rows, and
rejections that arrive as data rather than exceptions.

```ts
import { Db, relation, schema, contained, on, query, match, u64, str, type Brand, type Scope } from "@bjornpagen/bumbledb"

// Branded, fresh-minted id types.
const HolderId = u64.newtype("HolderId")
const AccountId = u64.newtype("AccountId")

// Relations. `.fresh` marks an engine-minted primary key.
const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Account = relation("Account", { id: AccountId.fresh, holder: HolderId })

// A theory: every Account.holder must reference an existing Holder.id.
const Ledger = schema("Ledger", { Holder, Account }, [contained(on(Account, "holder"), on(Holder, "id"))])

const db = await Db.create("./ledger.db", Ledger)

// Write. The delta is judged against every statement at commit.
let adaId: Brand<bigint, "HolderId"> | undefined
const result = db.write((tx) => {
	const ada = tx.insert(Holder, { name: "ada" }) // ada.id is a branded HolderId
	adaId = ada.id
	tx.insert(Account, { holder: ada.id })
})

// Rejection-as-data: no throw — a rejected commit is a typed value carrying
// every violated statement, cited once, with its canonical spelling and facts.
if (!result.ok) {
	for (const v of result.violations) {
		console.error(v.kind, v.canonical, v.facts)
	}
}

// Query: Datalog as values. Rows are typed from the `select` shape.
const accountsOf = query(Ledger, ($: Scope<(typeof Ledger)["relations"]>) => {
	const acct = $.var(Account.fields.id)
	const holder = $.param("holder", Holder.fields.id)
	return { rules: [[match(Account, { id: acct, holder })]], select: { acct } }
})

const prepared = db.prepare(accountsOf)
const rows = db.execute(prepared, { holder: adaId }) // rows: { acct: AccountId }[]
```

## Surface

- The type kernel — brands, fields, `relation()`, `closed()`.
- The statement algebra — `schema()`, `key`, `contained`, `mirrors`, `window`.
- The `Db` runtime — path-cached stores, transactions, typed violations, scoped snapshot reads, the witnessed write loop.
- The query surface — Datalog as values: scoped vars/params, atoms, negation, conditions, aggregates, engine recursion via predicates, `db.prepare`.
- The exhume surface — `Db.exhume`, the schema-independent read path: a store's self-described shapes and raw facts by name.

## Cookbook

The engine cookbook's 29 modeling recipes, translated to this SDK's structural API: [COOKBOOK.md](./COOKBOOK.md). Every recipe is compile-pinned by `test/cookbook.test.ts` — each schema is admitted by the real engine and every query snippet lowers through `db.prepare` — so the cookbook can never drift from the surface.

## Architecture

The SDK is a typed surface over the native engine; the model (relations,
statement-based judgment, Datalog evaluation, MVCC storage, the witnessed
write loop) is documented in the [bumbledb engine repository](https://github.com/bjornpagen/bumbledb).

## License

0BSD
