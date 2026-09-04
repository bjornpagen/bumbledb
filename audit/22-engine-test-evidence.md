# Preserved engine audit harness and observations

Audit date: 2026-09-04. This document preserves the exact final external harness and package manifest used for the engine audit. It is evidence, not a production source change and not a proposed implementation.

The temporary project was `/tmp/bumbledb-engine-audit.6iFaKq`. Its source and databases stayed outside the repository. The manifest and source below were read back from that directory for preservation; the tests were **not rerun** when this document was written.

## What this establishes

| Finding | Observed behavior | Correct regression direction after the contract/fix is chosen |
|---|---|---|
| [ENG-001: unchecked interval construction](20-engine-semantics.md#eng-001--safe-unchecked-interval-construction-can-commit-invalid-intervals) | A reversed interval was admitted, then ordinary scan returned `Corruption::InvalidInterval` | Reject invalid construction/ingestion before persistence; retain valid prior state |
| [ENG-002: extensible fact-codec boundary](20-engine-semantics.md#eng-002--fact-is-an-extensible-safe-trait-but-insertion-treats-it-as-a-trusted-codec-proof) | A custom safe codec's bool byte `2` committed; scan returned `Corruption::InvalidBool(2)` | A supported custom codec cannot successfully admit noncanonical persisted facts |
| [ENG-003: compaction snapshot race](20-engine-semantics.md#eng-003--concurrent-compaction-associates-data-with-the-wrong-generation) | All ten concurrent copies had row count different from generation in a fixture where those must agree | Snapshot metadata and copied content must describe the same source snapshot |
| [ENG-004: escaped fresh-ID durability](20-engine-semantics.md#eng-004--escaped-fresh-ids-are-not-crash-durable-reservations) | Child printed `EntryId(0)` before abrupt exit; reopen reserved `EntryId(0)` again | Either make externally usable reservations durable or explicitly document/test their provisional pre-acknowledgment status |
| [ENG-005: complete rejection diagnostics](20-engine-semantics.md#eng-005--complete-key-rejection-diagnostics-are-incomplete-for-refused-fresh-landings) | Fresh-ID-conflicting proposed rows also shared an email key, but rejection cited only statement `1` | Under the complete-key-diagnostics contract, report both auto-ID and email-key violations (statements `1` and `2` here) |
| [ENG-006: dictionary retention](20-engine-semantics.md#eng-006--deletion-and-compaction-do-not-erase-dictionary-text) | Logical text count was zero, but the deleted string's dictionary ID remained after compaction/reopen | Test the explicitly chosen erasure operation; do not silently redefine ordinary compaction as secure erasure |
| [QRY-001: partial aggregate output](21-query-runtime.md#qry-001--an-aggregate-error-leaves-partial-answers-visible) | Aggregate execution returned an overflow error while the output buffer contained 100 rows | Failed execution must satisfy the chosen atomic output contract, rather than expose a partial current result |

The harness demonstrates undesirable behavior; it is **not** a ready-to-merge regression suite. In particular, its anomaly checks are printed observations, not `assert!` statements. Its `unwrap` calls mainly enforce fixture setup/success assumptions. Do not turn the observed defects into desired golden behavior. Convert each scenario into an independent regression test with the expectation inverted or otherwise aligned with the explicit corrected contract.

## Invocation and filesystem assumptions

The successful final invocation, from the repository working directory `/Users/bjorn/Documents/bumbledb`, was:

```sh
cargo run --manifest-path /tmp/bumbledb-engine-audit.6iFaKq/Cargo.toml
```

That working directory selected the repository's pinned `nightly-2026-08-15` toolchain. Cargo built a separate executable under the temporary project's own `target/debug` directory. No workspace test target or benchmark source was modified.

The program uses fixed absolute temporary paths and create-only destination operations. The final run created `store2`, `crash-store`, `compact-dictionary`, `concurrent-source`, and `race-copy-0` through `race-copy-9` under the temporary project. Therefore the exact program is **one-shot against an empty set of those destinations**: rerunning it against the existing evidence directories is not expected to reproduce the run successfully. For a future isolated reproduction, use a fresh temporary project directory and change the source's `base` path accordingly. Do not delete the repository or reuse a real tenant directory. No cleanup or deletion instructions are embedded in this harness.

## Exact `Cargo.toml`

```toml
[package]
name = "bumbledb-engine-audit-repro"
version = "0.0.0"
edition = "2024"

[dependencies]
bumbledb = { path = "/Users/bjorn/Documents/bumbledb/crates/bumbledb" }

```

## Exact `src/main.rs`

```rust
use bumbledb::*;

schema! {
    pub TestSchema;
    relation Entry { id: u64 as EntryId, fresh, grp: u64, amount: i64 }
    relation Flag { ok: bool }
    relation Window { during: interval<u64> }
    relation Person { id: u64 as PersonId, fresh, email: u64 }
    Person(email) -> Person;
    relation Text { text: str }
}

struct InvalidFlag;
impl<'a> Fact<'a> for InvalidFlag {
    type Schema = TestSchema;
    const RELATION: RelationId = RelationId(1);
    fn encode_insert<C: CodecWrite<TestSchema>>(&self, _: &mut C, out: &mut Vec<u8>) -> Result<()> { out.push(2); Ok(()) }
    fn encode_probe<C: CodecRead<TestSchema>>(&self, _: &C, out: &mut Vec<u8>) -> Result<Probe> { out.push(2); Ok(Probe::Encoded) }
    fn decode<C: CodecRead<TestSchema>>(_: &'a C, _: &[u8]) -> Result<Self> { Ok(Self) }
}

fn sum_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Aggregate { op: FoldOp::Sum, over: VarId(1) }],
        atoms: vec![Atom { source: AtomSource::Edb(RelationId(0)), bindings: vec![(FieldId(0), Term::Var(VarId(2))), (FieldId(1), Term::Var(VarId(0))), (FieldId(2), Term::Var(VarId(1)))] }],
        negated: vec![], conditions: vec![],
    })
}

fn main() -> Result<()> {
    let base = std::path::Path::new("/tmp/bumbledb-engine-audit.6iFaKq");
    if std::env::args().nth(1).as_deref() == Some("reserve-crash") {
        let db = Db::create(&base.join("crash-store"), TestSchema)?.unwrap();
        db.write(|tx| -> Result<()> {
            let id = tx.reserve::<EntryId>(1)?.start().unwrap();
            println!("escaped_id={id:?}");
            std::process::exit(7);
        })?;
        unreachable!();
    }
    let db = Db::create(&base.join("store2"), TestSchema)?.unwrap();
    let mut rows = (0..100).map(|i| Entry { id: EntryId(i), grp: i, amount: 1 }).collect::<Vec<_>>();
    rows.extend([Entry { id: EntryId(100), grp: 500, amount: i64::MAX }, Entry { id: EntryId(101), grp: 500, amount: 1 }]);
    db.write(|tx| tx.insert(&rows).map(|_| ()))?.unwrap();
    db.read(|snap| {
        let mut q = snap.prepare(&sum_query())?;
        let mut out = Answers::new();
        let result = snap.execute(&mut q, &[] as &[BindValue], &mut out);
        println!("aggregate result={result:?} leftover_rows={}", out.len());
        Ok(())
    })?;
    let bad = Interval::<u64>::__ground_axiom(9, 1);
    println!("invalid interval bounds={:?}", bad.bounds());
    let result = db.write(|tx| tx.insert([&Window { during: bad }]).map(|_| ()));
    println!("invalid interval admission={result:?}");
    db.read(|snap| { println!("invalid interval scan={:?}", snap.scan(RelationId(2))?.collect::<Vec<_>>()); Ok(()) })?;
    let result = db.write(|tx| tx.insert([&InvalidFlag]).map(|_| ()));
    println!("custom invalid bool admission={result:?}");
    db.read(|snap| { println!("invalid bool scan={:?}", snap.scan(RelationId(1))?.collect::<Vec<_>>()); Ok(()) })?;
    db.write(|tx| tx.insert([&Person { id: PersonId(1), email: 10 }, &Person { id: PersonId(2), email: 20 }]).map(|_| ()))?.unwrap();
    let result = db.write(|tx| tx.insert([&Person { id: PersonId(1), email: 99 }, &Person { id: PersonId(2), email: 99 }]).map(|_| ()))?;
    if let Admission::Rejected(violations) = result {
        println!("key conflict statement ids={:?}", violations.as_slice().iter().map(|(v,_)| v.statement_id(db.schema()).0).collect::<Vec<_>>());
        println!("key conflicts={violations:?}");
    }
    let child = std::process::Command::new(std::env::current_exe().unwrap()).arg("reserve-crash").output().unwrap();
    println!("abrupt child={} output={}", child.status, String::from_utf8_lossy(&child.stdout));
    let reopened = Db::open(&base.join("crash-store"), TestSchema)?;
    reopened.write(|tx| { println!("id_after_reopen={:?}", tx.reserve::<EntryId>(1)?.start()); Ok(()) })?.unwrap();
    let marker = Text { text: "AUDIT-PRIVATE-UNIQUE-DELETED-STRING-631921" };
    db.write(|tx| tx.insert([&marker]).map(|_| ()))?.unwrap();
    db.write(|tx| tx.delete([&marker]).map(|_| ()))?.unwrap();
    db.read(|snap| { println!("deleted_text_count={} dictionary_id={:?}", snap.count(RelationId(4))?, CodecRead::<TestSchema>::lookup_str(snap, marker.text)?); Ok(()) })?;
    db.compact(&base.join("compact-dictionary"))?;
    let compacted = Db::open(&base.join("compact-dictionary"), TestSchema)?;
    compacted.read(|snap| { println!("compacted_deleted_text_count={} dictionary_id={:?}", snap.count(RelationId(4))?, CodecRead::<TestSchema>::lookup_str(snap, marker.text)?); Ok(()) })?;
    let live = Db::create(&base.join("concurrent-source"), TestSchema)?.unwrap();
    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut i = 0;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                live.write(|tx| tx.insert([&Entry {id: EntryId(i), grp: i, amount: 1}]).map(|_| ())).unwrap().unwrap();
                i += 1;
            }
        });
        for i in 0..10 {
            let path = base.join(format!("race-copy-{i}"));
            live.compact(&path).unwrap();
            let copy = Db::open(&path, TestSchema).unwrap();
            let count = copy.read(|snap| snap.count(RelationId(0))).unwrap();
            let generation = copy.generation().unwrap().value();
            println!("concurrent_compact #{i} rows={count} generation={generation} mismatch={}", count != generation);
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    Ok(())
}
```

## Recorded final runtime output

The final `cargo run` exited **0**. Its child deliberately exited **7** before destructors ran. This is the runtime output captured from that successful final run:

```text
aggregate result=Err(Overflow(Aggregate { find: FindIndex(1) })) leftover_rows=100
invalid interval bounds=(9, 1)
invalid interval admission=Ok(Accepted(Committed { value: (), generation: GenerationId(2) }))
invalid interval scan=[Err(Corruption(InvalidInterval([0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 1])))]
custom invalid bool admission=Ok(Accepted(Committed { value: (), generation: GenerationId(3) }))
invalid bool scan=[Err(Corruption(InvalidBool(2)))]
key conflict statement ids=[1]
key conflicts=Violations { citations: [(Functionality { statement: Key(KeyId(1)), fact: [0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 99], conflict: Scalar }, [CitedFact { relation: RelationId(3), values: [U64(1), U64(99)] }])] }
abrupt child=exit status: 7 output=escaped_id=EntryId(0)

id_after_reopen=Some(EntryId(0))
deleted_text_count=0 dictionary_id=Some(InternId(0))
compacted_deleted_text_count=0 dictionary_id=Some(InternId(0))
concurrent_compact #0 rows=2 generation=0 mismatch=true
concurrent_compact #1 rows=14 generation=12 mismatch=true
concurrent_compact #2 rows=25 generation=21 mismatch=true
concurrent_compact #3 rows=42 generation=38 mismatch=true
concurrent_compact #4 rows=63 generation=56 mismatch=true
concurrent_compact #5 rows=79 generation=76 mismatch=true
concurrent_compact #6 rows=98 generation=94 mismatch=true
concurrent_compact #7 rows=117 generation=108 mismatch=true
concurrent_compact #8 rows=144 generation=134 mismatch=true
concurrent_compact #9 rows=160 generation=152 mismatch=true
```

Counts in the concurrent section depend on scheduling; those particular numbers are an observed trace, not a deterministic reproduction promise. The invariant under test is equality of count and generation in this fresh fixture, because each successful source transaction inserts exactly one distinct fact and nothing else changes its logical state.

## Compiler warning and preparation history

The final harness emitted one warning from its own source:

```text
warning: unused `bumbledb::Admission` that must be used
  --> src/main.rs:34:9
   |
34 | /         db.write(|tx| -> Result<()> {
35 | |             let id = tx.reserve::<EntryId>(1)?.start().unwrap();
36 | |             println!("escaped_id={id:?}");
37 | |             std::process::exit(7);
38 | |         })?;
   | |___________^
   |
   = note: `#[warn(unused_must_use)]` (part of `#[warn(unused)]`) on by default
```

That code path exits before `write` returns. The warning is preserved rather than silently editing the evidence source; it is not a production engine warning or a failing assertion.

An earlier smaller harness version successfully reproduced the aggregate, interval, custom-codec, and key-diagnostic cases. When the child-process and compaction cases were added, one intermediate compilation failed with `E0282` because the always-exiting closure lacked an explicit return type. Adding `-> Result<()>` yielded the final source above. That failed compilation did not run the database program. An earlier successful version used `store`; the final version intentionally used fresh `store2` instead.

## Dependency provenance and limits

- The harness compiled against the working-tree `bumbledb` path dependency, version `0.20.3`, and its path dependencies `bumbledb-macros` / `bumbledb-theory`. It did not copy engine source into the temporary project.
- The temporary project generated its **own** `/tmp/bumbledb-engine-audit.6iFaKq/Cargo.lock` (lockfile format 4). Cargo reported locking 70 packages to versions compatible with `Rust 1.99.0-nightly`; it did not reuse the repository workspace's lockfile. Consequently this test run is not evidence that every dependency exactly matched the workspace-locked build.
- Observed dependency versions included `heed 0.22.1`, `lmdb-master-sys 0.2.6`, `blake3 1.8.7`, `cc 1.4.5`, and `find-msvc-tools 0.1.12`. This document preserves the exact project manifest/source, not the complete generated dependency lockfile or vendored dependencies.
- These were small public-API reliability tests in temporary stores. They establish the reported behavior under the observed build and schedule, not exhaustive theorem coverage or absence of other faults.
- The abrupt child exit tests missing destructor execution; it is not a physical power-loss experiment, failed-fsync experiment, or demonstration of reusing an already-committed ID.
- Dictionary lookup after deletion/compaction confirms a live dictionary entry, but no claim is made that ordinary compaction promised secure erasure. ENG-006 is an architectural retention/erasure-contract decision.
- The two intentionally noncanonical-value cases concern persistence integrity. No Rust undefined behavior, memory exploitation, unauthorized access, production database input, or destructive resource-exhaustion experiment was involved.
- This final combined harness keeps the noncanonical interval/bool rows in one temporary store while subsequent unrelated checks run. That is acceptable for recording the observed small scenarios; permanent regressions should isolate each finding in a fresh database and make each expected postcondition explicit.
- ENG-007, ENG-008, QRY-002, and QRY-003 are not runtime-tested by this harness. Their confidence remains the static-review status stated in their respective reports.
