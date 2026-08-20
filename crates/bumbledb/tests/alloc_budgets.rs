//! Ordinary-tier allocation-law budgets (audit 28): [`AllocWindow`] /
//! [`AllocAbsolute`] pins on the named hot paths. One test function —
//! the counting allocator is process-global; a sibling `#[test]` would
//! race the window.
//!
//! The lib registers the counter only under `alloc-counter`. This
//! binary registers it when that feature is off so `cargo test
//! --workspace` has eyes.

use bumbledb::alloc_counter::{self, AllocWindow};
use bumbledb::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId};
use bumbledb::schema::FieldId;
use bumbledb::{Answers, BindValue, Db, Fact, InstanceBuilder, ParamArg, ParamId, PreparedQuery};

#[cfg(not(feature = "alloc-counter"))]
use bumbledb::alloc_counter::CountingAllocator;

mod common;

#[cfg(not(feature = "alloc-counter"))]
#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

bumbledb::schema! {
    pub Budget;

    relation Holder {
        id: u64 as HolderId, fresh,
        tag: u64,
    }
    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        bal: i64,
    }

    Account(holder) <= Holder(id);
}

/// Steady-batch width for [`ALLOCS_PER_COMMITTED_FACT`].
const COMMIT_BATCH: u64 = 64;

/// Host allocations allowed per fact in a warmed same-shape batch
/// commit.
///
/// Derivation from the arena design (`arena.rs`): a `WriteDelta` is
/// born empty every commit. The bump arena hands out 64 KiB chunks —
/// one heap allocation for the first fact, then zero arena allocs
/// while the batch stays under the chunk (`COMMIT_BATCH` × a two-word
/// row is well under 2 KiB). The fact `HashMap` grows logarithmically.
/// Each unique fresh key inserts one `BTreeMap` determinant node.
/// Plan/judgment collect exact-size `Vec`s once per commit (amortized
/// 1/N). Measured on this fixture after three warmup commits: 172
/// window allocs / 64 facts = 2.69. `K = 3` is that ceiling. Issue
/// 29's hunt may lower it; it must not rise.
const ALLOCS_PER_COMMITTED_FACT: u64 = 3;

fn scan_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Holder::RELATION),
            bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn join_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(Account::RELATION),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: AtomSource::Edb(Holder::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn key_probe_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(Account::RELATION),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn window() -> AllocWindow {
    alloc_counter::snapshot().window
}

fn assert_zero(window_name: &str, shape: &str, w: AllocWindow) {
    assert_eq!(
        w.allocs, 0,
        "{window_name} window={shape} count={}",
        w.allocs
    );
}

fn seed_store(db: &Db<Budget>) -> (HolderId, AccountId, Holder, Account) {
    db.write(|tx| {
        let hid = tx.reserve::<HolderId>(1)?.start().expect("holder id");
        let aid = tx.reserve::<AccountId>(1)?.start().expect("account id");
        let holder = Holder { id: hid, tag: 7 };
        let account = Account {
            id: aid,
            holder: hid,
            bal: 11,
        };
        tx.insert([&holder])?;
        tx.insert([&account])?;
        Ok((hid, aid, holder, account))
    })
    .expect("seed")
    .unwrap()
    .value
}

fn seed_heap() -> (
    bumbledb::OwnedInstance<Budget>,
    HolderId,
    AccountId,
    Holder,
    Account,
) {
    let mut builder = InstanceBuilder::new(Budget).expect("valid");
    let hid = builder
        .reserve::<HolderId>(1)
        .expect("reserve")
        .start()
        .expect("holder id");
    let aid = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("account id");
    let holder = Holder { id: hid, tag: 7 };
    let account = Account {
        id: aid,
        holder: hid,
        bal: 11,
    };
    builder.load([&holder]).expect("load holder");
    builder.load([&account]).expect("load account");
    let instance = builder.admit().expect("admit").expect("accepted");
    (instance, hid, aid, holder, account)
}

fn warm_query_store(
    label: &str,
    db: &Db<Budget>,
    prepared: &mut PreparedQuery<Budget>,
    params: &[BindValue<'_>],
) {
    db.read(|snap| {
        let mut out = Answers::new();
        snap.execute(prepared, params, &mut out).expect(label);
        alloc_counter::reset();
        snap.execute(prepared, params, &mut out).expect(label);
        assert_zero("allocs_per_warm_query", label, window());
        assert!(!out.is_empty(), "{label}: fixture produced rows");
        Ok(())
    })
    .expect(label);
}

fn warm_query_heap(
    label: &str,
    instance: &bumbledb::OwnedInstance<Budget>,
    prepared: &mut PreparedQuery<Budget>,
    params: &[ParamArg<'_>],
) {
    let mut out = Answers::new();
    instance.execute(prepared, params, &mut out).expect(label);
    alloc_counter::reset();
    instance.execute(prepared, params, &mut out).expect(label);
    assert_zero("allocs_per_warm_query", label, window());
    assert!(!out.is_empty(), "{label}: fixture produced rows");
}

fn point_read_store(db: &Db<Budget>, id: AccountId, fact: &Account) {
    db.read(|snap| {
        let _ = snap.get(id)?.expect("present");
        assert!(snap.contains(fact)?);
        alloc_counter::reset();
        let got = snap.get(id)?.expect("present");
        assert_eq!(got.bal, fact.bal);
        assert!(snap.contains(fact)?);
        assert_zero("allocs_per_point_read", "store", window());
        Ok(())
    })
    .expect("store point read");
}

fn point_read_heap(instance: &bumbledb::OwnedInstance<Budget>, id: AccountId, fact: &Account) {
    let _ = instance.get(id).expect("get").expect("present");
    assert!(instance.contains(fact).expect("contains"));
    alloc_counter::reset();
    let got = instance.get(id).expect("get").expect("present");
    assert_eq!(got.bal, fact.bal);
    assert!(instance.contains(fact).expect("contains"));
    assert_zero("allocs_per_point_read", "heap", window());
}

fn committed_fact_budget(db: &Db<Budget>) {
    let mut next_tag = 1_000u64;
    for _ in 0..3 {
        insert_batch(db, &mut next_tag);
    }
    alloc_counter::reset();
    insert_batch(db, &mut next_tag);
    let w = window();
    let per = w.allocs.div_ceil(COMMIT_BATCH);
    assert!(
        per <= ALLOCS_PER_COMMITTED_FACT,
        "allocs_per_committed_fact window=steady-batch-{COMMIT_BATCH} count={} per_fact={per} budget={ALLOCS_PER_COMMITTED_FACT}",
        w.allocs
    );
}

fn insert_batch(db: &Db<Budget>, next_tag: &mut u64) {
    let base = *next_tag;
    *next_tag += COMMIT_BATCH;
    common::expect_admitted(db.write(|tx| {
        for i in 0..COMMIT_BATCH {
            let hid = tx.reserve::<HolderId>(1)?.start().expect("holder id");
            tx.insert([&Holder {
                id: hid,
                tag: base + i,
            }])?;
        }
        Ok(())
    }));
}

/// Admission-phase peak: the proposal's `max(A+I+R, A+R+F+J)` is not a
/// named counter on `HeapStage` / `admit_catalog` (those files are not
/// this lane). [`AllocAbsolute::peak_live_bytes`] is the honest
/// instrument — pin that admit's peak delta stays inside three arena
/// chunks (fact + dict + runs) plus the frozen catalog `F` plus one
/// chunk of judgment scratch.
fn admission_peak_bound() {
    const CHUNK: u64 = 64 * 1024;
    let mut builder = InstanceBuilder::new(Budget).expect("valid");
    let hid = builder
        .reserve::<HolderId>(1)
        .expect("reserve")
        .start()
        .expect("holder id");
    let aid = builder
        .reserve::<AccountId>(1)
        .expect("reserve")
        .start()
        .expect("account id");
    builder.load([&Holder { id: hid, tag: 1 }]).expect("load");
    builder
        .load([&Account {
            id: aid,
            holder: hid,
            bal: 1,
        }])
        .expect("load");
    let before = alloc_counter::snapshot();
    let instance = builder.admit().expect("admit").expect("accepted");
    let after = alloc_counter::snapshot();
    let f = u64::try_from(instance.retained_bytes()).expect("fits");
    let peak_delta = after
        .absolute
        .peak_live_bytes
        .saturating_sub(before.absolute.peak_live_bytes);
    let bound = CHUNK * 4 + f;
    assert!(
        peak_delta <= bound,
        "admission peak window=admit count={peak_delta} bound={bound} (A+I+R / A+R+F+J stand-in)"
    );
}

#[test]
fn alloc_law_budgets() {
    // Admit first, while process peak-live is still close to this window.
    admission_peak_bound();

    let (heap, _hid, heap_aid, _hh, heap_acct) = seed_heap();
    let mut heap_scan = heap.prepare(&scan_query()).expect("prepare heap scan");
    warm_query_heap("heap/scan", &heap, &mut heap_scan, &[]);
    let mut heap_join = heap.prepare(&join_query()).expect("prepare heap join");
    warm_query_heap("heap/join", &heap, &mut heap_join, &[]);
    let mut heap_probe = heap
        .prepare(&key_probe_query())
        .expect("prepare heap probe");
    warm_query_heap(
        "heap/key_probe",
        &heap,
        &mut heap_probe,
        &[ParamArg::Scalar(BindValue::U64(heap_aid.0))],
    );
    point_read_heap(&heap, heap_aid, &heap_acct);

    let dir = common::TempDir::new("alloc-budgets");
    let db = Db::create(dir.path(), Budget)
        .expect("create")
        .expect("accepted");
    let (_hid, aid, _holder, account) = seed_store(&db);

    let mut scan = db.prepare(&scan_query()).expect("prepare scan");
    warm_query_store("store/scan", &db, &mut scan, &[]);
    let mut join = db.prepare(&join_query()).expect("prepare join");
    warm_query_store("store/join", &db, &mut join, &[]);
    let mut probe = db.prepare(&key_probe_query()).expect("prepare probe");
    warm_query_store("store/key_probe", &db, &mut probe, &[BindValue::U64(aid.0)]);

    point_read_store(&db, aid, &account);
    committed_fact_budget(&db);
}
