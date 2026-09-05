# bumbledb

Bumbledb is an embedded relational database for Rust and TypeScript. It
replaces SQL strings with typed schemas and queries, checks cross-row
constraints before each commit, and executes complex reads with Free Join over
LMDB-backed data.

The database runs in the application process. Schemas are declared in code,
records are ordinary Rust structs or TypeScript objects, and prepared queries
run without parsing or interpreting SQL. Relations use set semantics, so a
record is either present or absent: duplicate inserts are harmless, deletes
are idempotent, and query results do not need a separate deduplication step.

Here is a complete Rust example. Operations take an explicit finite
`WorkContext`. There is no write/read callback and no unlimited work twin.

```rust
use bumbledb::{ApplyExpected, ApplyOutcome, ChangeSet, ChangeSetBuilder, CloseReport, Db, Fact, WorkContext};

bumbledb::schema! {
    pub Ledger;

    closed relation Region as RegionId = { Na, Eu, Apac, Latam };
    closed relation Status as StatusId = { Open, Frozen, Closed };

    relation Holder {
        id: u64 as HolderId,
        name: str,
        region: u64 as RegionId,
    }
    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        status: u64 as StatusId,
        opened_at: i64,
    }

    Holder(id)   -> Holder;
    Account(id)  -> Account;
    Account(holder) <= Holder(id);
    Holder(region)  <= Region(id);
    Account(status) <= Status(id);
}

fn open_ledger(path: &std::path::Path, work: WorkContext) -> bumbledb::Result<Db<Ledger>> {
    Ok(Db::create(path, Ledger, work)?.expect("empty Ledger admits"))
}

fn insert_fact<'a, F: Fact<'a>>(
    draft: &mut ChangeSetBuilder<'_>,
    fact: &F,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut values = Vec::new();
    fact.append_values(&mut values)?;
    draft.insert(F::RELATION, &values)?;
    Ok(())
}

fn seed(db: &Db<Ledger>, work: &WorkContext) -> Result<ApplyOutcome, Box<dyn std::error::Error>> {
    let holder = HolderId(1);
    let account = AccountId(42);
    let mut draft = ChangeSet::builder(db.schema(), work.clone());
    insert_fact(&mut draft, &Holder { id: holder, name: "alice", region: Region::Eu.id() })?;
    insert_fact(&mut draft, &Account {
        id: account,
        holder,
        status: Status::Open.id(),
        opened_at: 17_000_000,
    })?;
    Ok(db.apply(&draft.finish()?, ApplyExpected::Any, work)?)
}

fn pin_and_close(db: &Db<Ledger>, work: &WorkContext) -> Result<CloseReport, Box<dyn std::error::Error>> {
    let snapshot = db.snapshot(work)?;
    drop(snapshot);
    Ok(db.close())
}
```

`HolderId` and `AccountId` are different Rust types. Passing an account ID
where a holder ID is expected is a compile error, and a record from one schema
cannot be written to a database opened with another schema.

The equivalent TypeScript API is available as
[`@bjornpagen/bumbledb`](ts/README.md). Both language surfaces build the same
schema description and query representation before calling the same engine.

## Installation

Crate publication is not authorized. A downstream Rust consumer uses a path
dependency the way `examples/consumers/rust` does.

```toml
[dependencies]
bumbledb = { git = "https://github.com/bjornpagen/bumbledb", branch = "codex/bumbledb-1-0" }
```

TypeScript packages require Effect `4.0.0-rc.112` exactly and Node 24+:

```sh
pnpm add @bjornpagen/bumbledb @bjornpagen/bumbledb-log effect
```

Packed-consumer qualification uses freshly staged tarballs, not workspace
aliases. That gate is **NotRun** until the final campaign.

## Packages and supported platforms

- Rust: the `bumbledb` core crate (source consumers; crate publication is
  not yet authorized). The internal log crate is not a public Rust API.
- TypeScript: `@bjornpagen/bumbledb` (core) and `@bjornpagen/bumbledb-log`
  (durable named commands, backup/restore, generated migrations), both
  Effect-native with an exact `effect@4.0.0-rc.112` peer.
- One prebuilt native package per platform, shared by both TS packages:
  `darwin-arm64` (macOS 14+), `linux-arm64` and `linux-x64`
  (glibc 2.34 / Amazon Linux 2023 floor). Node 24 is the deployment
  baseline. Edge/browser/mobile runtimes are unsupported.

See `docs/reference/packaging.md` for the staging/pin design.
The worked application example lives in `examples/notes`; packed
consumers live in `examples/consumers`. Next.js/Alchemy is Node-only;
there is no Edge/browser/mobile promise.

## What the database provides

Bumbledb is intended for normalized, read-heavy application data: ledgers,
calendars, graphs, scheduling systems, and other models with many narrow
relations and frequent joins. It provides:

- typed schemas, records, IDs, keys, parameters, and result rows;
- joins, negation, comparisons, parameter sets, aggregates, and recursive
  reachability;
- first-class half-open intervals, including point lookup, overlap tests, all
  thirteen Allen relationships, and merging adjacent ranges;
- unique keys, references, conditional references, exact one-to-one
  relationships, interval exclusions, and count, sum, or duration limits;
- MVCC snapshots with concurrent readers and one serialized writer;
- durable and non-durable stores, structured constraint failures, and explicit
  export/import for schema changes;
- prepared queries that reuse plans, buffers, and indexes under an explicit
  `WorkContext`; large results, new text, and spill paths remain bounded.

There is no server process and no network protocol. Bumbledb opens an LMDB
store directly. Query work runs in the caller's thread for the embedded API;
hosted log and native bridge paths schedule bounded work through their runtime
executors without claiming a separate database server.

## Constraints are part of the schema

SQL databases expose related integrity rules through several separate
features: unique indexes, foreign keys, checks, exclusion constraints, and
triggers. Bumbledb expresses the supported forms as statements between
relations and checks them against the transaction's final state.

`R(id) -> R` declares a unique key. `A(x) <= B(y)` says that every `x` in
`A` must match an existing `y` in `B`. Filters on either side make the
reference conditional. `A(x) == B(y)` requires the relationship in both
directions, which is useful for representing sum types as a parent record and
one exact variant record.

Intervals participate in the same rules. If the last field of a unique key is
an interval, records with the same preceding fields may not overlap. A
reference between intervals means that the source interval must be completely
covered by the target intervals. Capacity constraints can limit a related
count, sum, or total duration.

Fixed sets can carry data as well as names:

```rust
closed relation Status as StatusId = { Open, Frozen, Closed };

closed relation Kind as KindId {
    mastered: bool,
    rank: u64,
} = {
    DirectPass { mastered: true,  rank: 30 },
    JudgedPass { mastered: true,  rank: 20 },
    Failed     { mastered: false, rank: 10 },
};

Attempt(kind) <= Kind(id);
Certificate(kind) <= Kind(id | mastered == true);
```

`Kind` behaves like an enum in application code, while its `mastered` and
`rank` columns remain available to schemas and queries. The final statement
allows certificates to refer only to kinds whose `mastered` value is true.
These fixed records live in the schema and occupy no rows in the store.

Writes are accumulated in memory and checked once before LMDB is modified.
This means an update can delete an old record and insert its replacement in
either order. If the finished transaction satisfies the schema, it commits. If
not, nothing is written and the returned error identifies the constraint and
records involved.

The [cookbook](docs/cookbook.md) contains thirty-two worked schemas covering
sum types, optional attributes, vocabularies, trees, graphs, state machines,
calendars, effective-dated configuration, tax brackets, ledgers, derived data,
recursive closure, point reads, and resource limits. Every example is compiled
as part of the test suite.

## Measurement

1.0 qualification measures the successor tree on Apple Silicon, real
Graviton ARM64, and x86 Node. Historical 0.17.0 night numbers are not 1.0
evidence and are not restated here. L20 owns the measurement inputs;
execution is **NotRun** until that campaign.

## Schemas and queries

The schema macro supports six stored value representations plus fixed
relations:

| Type | Use | Comparisons and operations |
|---|---|---|
| `u64` | IDs, counts, and unsigned values | equality, ordering, parameter sets, numeric aggregates |
| `i64` | signed values, timestamps, and money under a host type | equality, ordering, parameter sets, numeric aggregates |
| `bool` | true or false | equality |
| `str` | UTF-8 text that may repeat | equality and parameter sets |
| `bytes<N>` | fixed-size hashes and binary identities | equality and parameter sets |
| `interval<E>` | nonempty half-open ranges | point membership, overlap and Allen relationships, merging |
| `interval<E, w>` | fixed-width half-open ranges | the same interval operations |
| `closed relation` | a fixed enum-like set, optionally with columns | equality, parameter sets, filtered references, joins |

Text stays inline in durable values. Fixed-size byte values stay inline.
Intervals store ordered endpoints and may use the largest endpoint to
represent an open-ended range.

The `as NewType` field modifier emits a Rust newtype for nominal safety.
Entity and record IDs are application-owned values declared in the schema;
the engine does not mint or reserve identifiers.

Queries are plain data after macro or builder expansion. They can be stored,
composed, prepared once, and executed repeatedly with different parameters.
The engine supports multiple rules whose results are combined as a set,
negated records, comparisons, set-valued parameters, `Count`, `Sum`, `Min`,
`Max`, interval merging, named intermediate results, and one linear recursive
query for reachability.

The raw query representation remains public for language bindings and tools.
The Rust `query!` macro and TypeScript builder are conveniences over that same
representation rather than separate query engines.

## Architecture

The laws live in the code at the site each governs; decision history
lives in git. Worked schemas are [`docs/cookbook.md`](docs/cookbook.md).

The implementation of Free Join follows Wang, Willsey, and Suciu,
*Free Join: Unifying Worst-Case Optimal and Traditional Joins* (SIGMOD 2023),
with the engine's differences documented alongside the code.

The Rust engine and TypeScript packages use the shared schema and query
definitions in this repository. [`lean/`](lean/README.md) contains an
executable specification of the admitted language. Correspondence with
current Rust is a qualification obligation, not a claim that this tree
already passed.

## Qualification

Discriminators for D01–D29 and G00–G16 are authored now and executed only
in the final post-retirement campaign. A successful local import is not
all-platform evidence. Missing real S3 or Graviton cells remain **NotRun**.

Public specimens: `examples/notes` (Next.js + Alchemy, Node only) and
`examples/consumers` (Rust / core TS / log TS / native-ledger).

## Repository layout

```text
crates/bumbledb/               database engine
crates/bumbledb-log/           durable command / lifecycle crate
ts/                            TypeScript core package and native bridge
ts-log/                        TypeScript log package
examples/notes/                Next.js + Alchemy application
examples/consumers/            packed Rust / TS / native-ledger specimens
lean/                          executable specification
docs/                          architecture, cookbook, and references
scripts/                       qualification runners
```

## Current target

Branch `codex/bumbledb-1-0` targets **1.0**: application-owned entity IDs,
Effect-only TypeScript operations, generated migrations, no public C API,
and no public Rust log SDK. Predecessor 0.17.0 tags are not this product.

Bumbledb uses one writer and concurrent snapshot readers. The embedded API
owns no server port. Schema changes within the selected format family use
generated migrations; incompatible predecessor stores refuse before mutation.

## License

[0BSD](LICENSE). Use Bumbledb for any purpose without an attribution
requirement.
