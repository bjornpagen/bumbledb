# bumbledb

An embedded, set-semantic relational database for application data. Rust does
the database work; TypeScript uses Effect. LMDB provides durable storage and
snapshot isolation, and Free Join executes the joins.

The bet is simple: a good data model, a good backend, and a performance-aware
core. The target is a database per user or tenant—not an analytics warehouse
or a SQL compatibility layer.

## The model

Relations are sets: inserting an existing fact and deleting an absent fact
are idempotent. Schemas declare keys, containment, interval relationships, and
capacity constraints. Writes are judged against their final state.

Rust macros and TypeScript builders construct schemas and query ASTs directly.
There is no SQL parser. Prepared queries support joins, negation, aggregates,
and reachability, with reusable execution buffers for the warmed fast path.

Values include integers, floats, booleans, text, fixed-size bytes, UUIDs, and
intervals, including float endpoints. UUIDs occupy 16 bytes and sort by their
unsigned bytes. Rust uses `uuid::Uuid`; TypeScript uses structural UUID
strings. Generate IDs in the application, including UUIDv7. Host-language
newtypes do not become nominal engine types.

LMDB supports databases larger than RAM. Performance is best with the working
set in memory; disk, address-space, and explicit work limits still apply.
The embedded database has concurrent snapshot readers and a serialized writer.
Set semantics do not make arbitrary concurrent writes conflict-free.

## Rust and TypeScript

The public surfaces are Rust core, TypeScript core, and TypeScript
`bumbledb-log`. There is no C API or public Rust log SDK.

TypeScript schema/query construction is synchronous metadata. Database work
is lazy Effect with scoped ownership and explicit budgets; there is no
parallel Promise or synchronous database API. The current packages require
**Node 24+** and pin **Effect 4.0.0-rc.112**.

Start with the [TypeScript guide](ts/README.md), the
[log guide](ts-log/README.md), or the executable
[Rust and TypeScript consumers](examples/consumers/README.md).
The [cookbook](docs/cookbook.md) contains thirty-two worked schemas.

Rust source consumers can use:

```toml
[dependencies]
bumbledb = { git = "https://github.com/bjornpagen/bumbledb", branch = "main" }
```

Build with the repository's pinned nightly. For reproducible deployments,
replace the moving branch with a tested `rev`. The working-tree version is
**0.20.3**; this checkout is being prepared for 1.0, not advertised as an
already-qualified 1.0 release. Published packages must not be assumed to
contain unreleased changes on `main`.

## A Rust model

This example admits related facts together and explicitly releases its
snapshot. Its code is compiled and exercised by the README tests.

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
    Ok(db.close(work))
}
```

Closed relations can also carry fixed data. Inside a schema declaration:

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

The `as NewType` declarations provide host-language nominal safety. The engine
sees structural value types and relational constraints.

## Durable application commands

`bumbledb-log` adds named, retryable commands, retained outcomes, checkpoints,
backup/restore, and generated migration workflows above the core. Local
history uses durable local storage; hosted history uses S3 authority and an
LMDB materialization. It reuses the core schema, changes, queries, and runtime.

The [Notes example](examples/notes/README.md) exercises a server-side Next.js
application with tenant isolation. Node deployments are the target; browser
and Edge runtimes are unsupported.

**Release limitation:** generated migration execution is local-only through
the TypeScript/native bridge. Hosted migration orchestration remains
unfinished. A hosted cache must never be treated as the authoritative
database for a local migration.

## Next benchmark round

There are no fresh 1.0 benchmark claims here. Historical charts in
[`assets/`](assets/) are not measurements of the current code.

The implementation is [`crates/bumbledb-bench/`](crates/bumbledb-bench/).
The [measurement runbook](docs/perf/measurement-plan.md) covers correctness,
warm reads, cold opens, first reads after writes, large results, tenant churn,
storage cost, and hash probes.

Preview the runner without building or timing anything:

```sh
scripts/bench-night.sh bench-out/next-round --plan
```

Once builds, tests, and other CPU-heavy work have stopped, run on a quiet host
with a **new** output directory:

```sh
scripts/bench-night.sh bench-out/quiet-round-YYYYMMDD-HHMM
```

The measurement lock serializes benchmark processes; it does **not** establish
that the machine is idle. Do not use `--shared` for the quiet-host baseline.
A successful local run is not Graviton, x86, real-S3, or larger-than-memory
qualification. Preserve raw reports and refusals, not just winning charts.

Apple Silicon is the first performance target. Linux ARM64/Graviton and Linux
x64 Node deployments must be measured separately. Correctness gates and a
zero-allocation test do not prove a throughput improvement.

## Development

```sh
(cd ts && pnpm install --frozen-lockfile)
(cd ts-log && pnpm install --frozen-lockfile)
scripts/battery.sh
```

The battery checks Rust, the native bridge, TypeScript, Lean correspondence,
and isolated packaged consumers. It is not a benchmark or permission to
publish. Machine-readable release requirements live in
[`.config/obligation-inventory.json`](.config/obligation-inventory.json);
[`scripts/release-results.mjs`](scripts/release-results.mjs) checks evidence.
Missing required evidence remains missing.

## Repository

- `crates/bumbledb/`: embedded engine.
- Theory, macro, and query crates: structural language.
- `crates/bumbledb-log/`: internal durable-history implementation.
- `ts/`, `ts-log/`: Effect SDKs and the shared native bridge.
- `crates/bumbledb-bench/`, `docs/perf/`, `assets/`: benchmarks and historical charts.
- `examples/`: runnable consumers and the Notes application.
- `lean/`: executable specification and correspondence checks.

Historical audits, proposals, and design discussions belong in Git history,
not parallel specifications beside the code.

## License

[0BSD](LICENSE).
