# bumbledb

A production-ready embedded, set-semantic relational database for application data. Rust does
the database work; TypeScript uses Effect. LMDB provides durable storage and
snapshot isolation, and Free Join executes the joins.

The bet is simple: a good data model, a good backend, and a performance-aware
core. The target is a database per user or tenant—not an analytics warehouse
or a SQL compatibility layer.

**1.0 production scope:** the embedded core and local `bumbledb-log`, with
Rust and Effect TypeScript APIs. Hosted history APIs are available, but real
S3/IAM and AWS Graviton qualification are deferred. Generated migrations are
local-only. Those hosted paths are not included in the production-ready claim.

[1.0 release](https://github.com/bjornpagen/bumbledb/releases/tag/v1.0.0)
· [Release notes](docs/release-1.0.md)
· [Benchmark results](docs/perf/results.md)

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
bumbledb = { git = "https://github.com/bjornpagen/bumbledb", tag = "v1.0.0" }
```

Build with the repository's pinned `nightly-2026-08-15`. Rust is distributed
from Git, not crates.io. TypeScript core, log, and all native packages use
**1.0.0** in lockstep:

```sh
pnpm add @bjornpagen/bumbledb@1.0.0 @bjornpagen/bumbledb-log@1.0.0 effect@4.0.0-rc.112
```

The GitHub release carries the prepared npm tarballs. Registry installation
requires their separate npm publication; a GitHub tag alone does not publish
npm packages. See the [release notes](docs/release-1.0.md).

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

**Hosted limitation:** generated migration execution is local-only through
the TypeScript/native bridge. Hosted migration orchestration is not supported
in 1.0. A hosted cache must never be treated as the authoritative database for
a local migration.

## Performance

The September 6, 2026 full local suite completed on an **Apple M2 Max**:
32 read families, 34 scenario queries, durable writes, constraints, storage,
scale curves, lifecycle costs, and two 10,000-cycle churn workloads.
Its independent pre-timing oracle checked 2,879 cases.

Selected full-suite medians: point lookup **0.50 µs**, range query **4.83 µs**,
warm application query **1.75 µs**, and complete construction/delivery of a
100,000-row native result **69.34 ms**. Separate alternating comparisons
measured that large-result path at **65–66 ms**, versus **796–801 ms** on the
previous measured checkpoint, with the same output work.

![Read latency against indexed SQLite](assets/bench-vs-sqlite.svg)

![Compacted database storage](assets/bench-storage.svg)

These are **shared-host, scheduler-boosted measurements**, not quiet-host
guarantees. The measured engine is `3ed3303e`, immediately before the 1.0
version/documentation/package cutover; raw reports retain its `0.20.3`
version label. Not every workload meets the informational latency budget,
and this run does not establish a win over every historical engine benchmark.
Compacted stores occupy **1.67–1.80× indexed SQLite** in the measured S/M
ledger/calendar workloads. There is no claim of cross-target or
larger-than-memory performance qualification.

The [complete results and caveats](docs/perf/results.md) include the chart
catalog and evidence coverage. The [measurement runbook](docs/perf/measurement-plan.md)
explains reproducible runs and profiling. Apple Silicon is the first performance
target; Linux ARM64 and x64 have correctness CI, not equivalent performance evidence.

For bottleneck analysis, use the [native profiling workflow](docs/perf/measurement-plan.md#native-stack-profiling).
The `profiling` build retains release optimization plus full inline/source
debug information. Samply captures native stacks without a production tracing
dependency or hand-maintained span hierarchy.
Every registered read and individual scenario query can be sampled by name;
generated worlds pass their ordinary SQLite oracle before the sampling window.
Displaced reads preserve their actual foreign-memory interleaving. Attribution
includes source sites, sample support and explicit unresolved costs; matched
captures can compare sampled CPU per completed draw without confusing profile
duration with performance.
Profiled runs explain costs. Unprofiled full-suite runs establish performance.
Native exports can be reanalyzed from preserved address-specific symbols,
with full caller paths, sample support, and unwinding-coverage diagnostics.
An explicit-roster survey compares caller costs across queries while keeping
missing workloads and per-query costs visible; it is not a release verdict.

## Development

```sh
(cd ts && pnpm install --frozen-lockfile)
(cd ts-log && pnpm install --frozen-lockfile)
scripts/battery.sh
```

The battery checks Rust, the native bridge, TypeScript, Lean correspondence,
and isolated packaged consumers. It is not a benchmark or permission to
publish. The historical machine-readable audit inventory lives in
[`.config/obligation-inventory.json`](.config/obligation-inventory.json);
[`scripts/release-results.mjs`](scripts/release-results.mjs) checks evidence.
It is not a substitute for actual reports or a blanket statement that every
historical audit obligation has been qualified. The [1.0 release scope](docs/release-1.0.md)
records what ships and what remains deferred; missing evidence stays missing.

## Repository

- `crates/bumbledb/`: embedded engine.
- Theory, macro, and query crates: structural language.
- `crates/bumbledb-log/`: internal durable-history implementation.
- `ts/`, `ts-log/`: Effect SDKs and the shared native bridge.
- `crates/bumbledb-bench/`, `docs/perf/`, `assets/`: benchmarks and current charts.
- `examples/`: runnable consumers and the Notes application.
- `lean/`: executable specification and correspondence checks.

Historical audits, proposals, and design discussions belong in Git history,
not parallel specifications beside the code.

## License

[0BSD](LICENSE).
