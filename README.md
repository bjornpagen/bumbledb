# bumbledb

An embedded, typed, **set-semantic** relational database for Rust, built on
LMDB, executing conjunctive queries with **Free Join** — and tuned, one
measured PRD at a time, for Apple Silicon.

There is no SQL and no interpreter in the hot path. You declare a schema with
a macro, write plain structs, and run conjunctive queries (joins, negation,
interval membership, comparisons, aggregates) that are planned once and
executed over columnar in-memory images with a lazy trie join. Results are
sets. Invariants are dependency statements — functional and inclusion
dependencies, judged at commit against the final state. Everything the
engine claims about performance is a pinned, reproducible measurement with
two differential oracles standing behind it.

```rust
bumbledb::schema! {
    pub Ledger;

    relation Holder {
        id: u64 as HolderId, serial,
        name: str,
        region: enum Region { Na, Eu, Apac, Latam },
    }
    relation Account {
        id: u64 as AccountId, serial,
        holder: u64 as HolderId,
        status: enum Status { Open, Frozen, Closed },
        opened_at: i64,
    }

    // Everything relational is a statement between the blocks — there are
    // no field-level modifiers. `serial` auto-materializes R(id) -> R.
    Account(holder) <= Holder(id);   // containment: every account's holder exists
}

let db = bumbledb::Db::create(path, Ledger)?;

// Writes are set arithmetic on an in-memory delta; every statement is
// judged at commit against the final state — an abort never touched disk.
db.write(|tx| {
    let holder: HolderId = tx.alloc()?;
    tx.insert(&Holder { id: holder, name: "alice", region: Region::Eu })?;
    let account: AccountId = tx.alloc()?;
    tx.insert(&Account { id: account, holder, status: Status::Open, opened_at: 17_000_000 })?;
    Ok(())
})?;

// Queries are prepared once, executed on snapshots into a reusable buffer —
// zero allocations per execution after warmup.
let mut q = db.prepare(&query)?;   // ir::Query: conjunctive atoms + predicates + finds
db.read(|snap| {
    snap.execute(&mut q, &params, &mut results)?;
    Ok(())
})?;
```

Newtypes are the nominal-safety layer: `HolderId` and `AccountId` are
distinct host types, and mixing them is a **compile error** — the database's
type discipline is enforced by rustc, not by runtime checks.

## The numbers

Same corpus, same queries, results verified identical against SQLite — and
every write judged identically by an independent naive model — across a
2,586-case differential oracle before any timing is believed:

![read families vs SQLite](assets/bench-vs-sqlite.svg)

The same data as multiples — the ledger's fifteen read families, point
lookups through negation, interval probes, and the triangle join:

![speedup over SQLite](assets/bench-speedup.svg)

Latency is a distribution, not a number. p50 → p95 → p99 per family, both
engines — the bimodal families show their true tails and still sit an order
of magnitude inside SQLite's:

![tail behavior](assets/bench-tails.svg)

Beyond the ledger, the suite runs four non-ledger scenario worlds —
JOB-shaped joins, a social graph, an OLAP star, and point-lookup surfaces —
22 queries, each oracle-gated before timing. Geomean across all 22: **16×**:

![scenario worlds](assets/bench-scenarios.svg)

And the honest chart — durable writes are an fsync-latency product on both
engines, and bulk load favors SQLite's write path; we publish it anyway:

![writes and cold](assets/bench-writes.svg)

**Context that keeps these numbers honest:** S-scale ledger corpus (10⁵-row
fact table), Apple M2 Max, engine-favorable workload class (point lookups
through multi-way joins and aggregates — exactly what a set-semantic Free
Join engine is built for). SQLite is measured warm, prepared, and
well-indexed on the identical data. This is a research engine validated at
this scale, not a production database. Regenerate everything yourself:

```sh
cargo build --release -p bumbledb-bench
target/release/bumbledb-bench gen && target/release/bumbledb-bench verify
target/release/bumbledb-bench bench --out bench-out/run1   # ×3
target/release/bumbledb-bench scenarios --out bench-out/scen
python3 scripts/bench_viz.py bench-out/run1 bench-out/run2 bench-out/run3 \
        --scenarios bench-out/scen/scenarios.md
```

## Why it's fast

Three design decisions do most of the work; deliberate microarchitecture
does the rest.

1. **Representation over control flow.** Relations live as columnar images
   (decoded once per generation, cached); queries run over a lazy trie
   (COLT) that materializes exactly the levels a join actually probes.
   Nothing is interpreted per row.
2. **Batched, two-phase execution.** The executor probes in batches of ~128:
   phase one computes all hashes (pure ALU), phase two issues all bucket
   loads as independent chains that fill the M-series' ~28 outstanding-miss
   lanes. Misses become branchless survivor compaction, never per-tuple
   control flow.
3. **Set semantics end to end.** No duplicate bookkeeping, no ordering
   obligations, idempotent writes — the algebra removes work before the
   machine ever sees it.

On top of that sit six microarchitectural mechanisms, each earning its
complexity with a measured, cited win at its site: bucket-of-8 tag-byte maps
at occupancy-invariant load factors, SWAR window probes, const-generic key
monomorphization, one software-prefetch pass, alias-hoisted loops, and a
single run-coherence memo. Nothing else made the cut — an optimization that
cannot cite its number does not ship.

## Architecture

The design is documented before it is code, and the docs are normative:
when code and these docs disagree, one of them is wrong and the repo is
broken until they agree.

| doc | what it owns |
|---|---|
| [00 — Product](docs/architecture/00-product.md) | what bumbledb is and refuses to be; the deleted vocabulary; the unsafe policy |
| [10 — Data Model](docs/architecture/10-data-model.md) | the seven structural types, the interval denotation, set semantics, identity |
| [20 — Query IR](docs/architecture/20-query-ir.md) | queries as data: atoms, negation, membership, param sets, aggregates |
| [30 — Dependencies](docs/architecture/30-dependencies.md) | the two judgments, statements, pointwise lifting, the acceptance gate |
| [40 — Execution](docs/architecture/40-execution.md) | Free Join, COLT, anti-probes, batching, the Apple Silicon model |
| [50 — Storage](docs/architecture/50-storage.md) | LMDB layout, guards as judgment accelerators, the delta write path |
| [60 — Validation](docs/architecture/60-validation.md) | the two oracles, the bench ledger, measurement discipline |
| [70 — Embedding API](docs/architecture/70-api.md) | the `schema!` grammar, `Db`, transactions, point reads, prepared queries |

The algorithmic reference is Wang, Willsey & Suciu, *Free Join: Unifying
Worst-Case Optimal and Traditional Joins* (arXiv:2301.10841), vendored in
[`docs/free-join-paper/`](docs/free-join-paper/).

## Measurement discipline

The part of this repo most worth stealing. Performance claims here are gated
by machinery, not judgment:

- **Two differential oracles before every timing run**: 2,586 cases —
  family queries and randomized queries against SQLite, plus a randomized
  write stream whose every commit verdict (accept or abort, and the
  violated statement) must match an independent brute-force naive model;
  the bench binary refuses to time against an unverified build (per-binary
  stamps).
- **A machine-wide measurement lock** (`scripts/measure.sh`) so two agents'
  runs never overlap, and **clock-proxy bracketing** around every timed block
  — blocks that ran during a DVFS sag or co-tenant interference are flagged
  and excluded, with optional per-sample normalization to adjudicate.
- **Disassembly gates** (`scripts/check-asm.sh`): properties like "the probe
  loop contains no calls and no `bcmp`" are asserted against `objdump`
  output — an `#[inline(always)]` that silently stopped working fails a
  gate, not a code review.
- **Microbench pins**: load-bearing mechanisms carry `#[ignore]`d in-tree
  benchmarks that re-assert their measured margins on demand.
- **Refutation is a result.** A mechanism that measures as a loss is
  reverted, and the record keeps the numbers and the failure mechanism —
  deletion is gated exactly like addition.

## Repository layout

```
crates/bumbledb/         the engine (LMDB via heed + blake3 are the only deps)
  src/exec/              executor, COLT, sinks, wordmap, NEON kernels
  src/storage/           LMDB env, deltas, commit, interning
  src/api/               Db, transactions, prepared queries
  src/plan/, src/ir/     planner and query IR
crates/bumbledb-macros/  the schema! proc macro (hand-rolled, no syn/quote)
crates/bumbledb-bench/   the oracle + benchmark suite (gen/verify/bench/trace)
docs/                    the normative architecture + pinned measurement records
scripts/                 measure.sh, check-asm.sh, check.sh, bench_viz.py
```

The gate suite (run `scripts/check.sh`, or by hand):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --features alloc-counter --test alloc_gate --release
scripts/check-asm.sh          # machine-property gates (needs a release bench build)
```

## Status

Research-grade and honest about it: validated at S scale on one platform
(Apple Silicon; portable scalar fallbacks compile everywhere but carry no
performance promises). No network layer, no SQL, no in-place migrations —
by design. See [00 — Product](docs/architecture/00-product.md) for the full
list of things this database refuses to become.

## License

[0BSD](LICENSE) — use it for anything; no attribution required.
