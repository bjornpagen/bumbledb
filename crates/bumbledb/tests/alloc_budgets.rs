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
        id: u64 as HolderId,
        tag: u64,
    }
    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        bal: i64,
    }

    Account(holder) <= Holder(id);
    Account(id) -> Account;
    Holder(id) -> Holder;
}

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

// Store-backed prepared images reuse their generation. Heap instances have
// no cache identity and rebuild bulk images on every call: their allocation
// count may grow geometrically, but never one allocation per row.
fn measured(f: impl FnOnce()) -> AllocWindow {
    alloc_counter::reset();
    f();
    alloc_counter::snapshot().window
}

fn heap_query(
    heap: &bumbledb::OwnedInstance<Budget>,
    prepared: &mut PreparedQuery<Budget>,
    params: &[ParamArg<'_>],
    expected: usize,
) -> AllocWindow {
    let mut out = Answers::new();
    for _ in 0..3 {
        heap.execute(prepared, params, &mut out).expect("warm heap");
    }
    let counts = measured(|| {
        heap.execute(prepared, params, &mut out).expect("heap");
    });
    assert_eq!(out.len(), expected);
    counts
}

fn store_query(
    db: &Db<Budget>,
    prepared: &mut PreparedQuery<Budget>,
    params: &[BindValue<'_>],
    expected: usize,
) -> AllocWindow {
    db.read(common::work(), |snap| {
        let mut out = Answers::new();
        for _ in 0..3 {
            snap.execute(prepared, params, &mut out)?;
        }
        let counts = measured(|| {
            snap.execute(prepared, params, &mut out).expect("store");
        });
        assert_eq!(out.len(), expected);
        Ok(counts)
    })
    .expect("read")
}

fn fixture_counts(rows: usize) -> [AllocWindow; 8] {
    let dir = common::TempDir::new(&format!("alloc-scaling-{rows}"));
    let db = Db::create(dir.path(), Budget, common::work())
        .expect("create")
        .unwrap();
    let mut builder = InstanceBuilder::new(Budget, common::work()).expect("builder");
    db.write(common::work(), |tx| {
        for n in 0..u64::try_from(rows).expect("small fixture") {
            let holder = Holder {
                id: HolderId(n),
                tag: n,
            };
            let account = Account {
                id: AccountId(n),
                holder: HolderId(n),
                bal: 11,
            };
            tx.insert([&holder])?;
            tx.insert([&account])?;
            builder.load([&holder])?;
            builder.load([&account])?;
        }
        Ok(())
    })
    .expect("populate")
    .unwrap();
    let heap = builder.admit().expect("admit").unwrap();
    let mut hs = heap.prepare(&scan_query()).expect("scan");
    let mut hj = heap.prepare(&join_query()).expect("join");
    let mut hp = heap.prepare(&key_probe_query()).expect("probe");
    let mut ds = db.prepare(&scan_query(), common::work()).expect("scan");
    let mut dj = db.prepare(&join_query(), common::work()).expect("join");
    let mut dp = db
        .prepare(&key_probe_query(), common::work())
        .expect("probe");
    let hs = heap_query(&heap, &mut hs, &[], rows);
    let hj = heap_query(&heap, &mut hj, &[], rows);
    let hp = heap_query(&heap, &mut hp, &[ParamArg::Scalar(BindValue::U64(0))], 1);
    let ds = store_query(&db, &mut ds, &[], rows);
    let dj = store_query(&db, &mut dj, &[], rows);
    let dp = store_query(&db, &mut dp, &[BindValue::U64(0)], 1);
    let work = common::work();
    let fact = Account {
        id: AccountId(0),
        holder: HolderId(0),
        bal: 11,
    };
    let heap_get = measured(|| {
        assert_eq!(
            heap.get(AccountById { id: fact.id }, &work).expect("get"),
            Some(fact)
        );
        assert!(heap.contains(&fact, &work).expect("contains"));
    });
    let store_get = db
        .read(common::work(), |snap| {
            Ok(measured(|| {
                assert_eq!(
                    snap.get(AccountById { id: fact.id }).expect("get"),
                    Some(fact)
                );
                assert!(snap.contains(&fact).expect("contains"));
            }))
        })
        .expect("read");
    [hs, hj, hp, ds, dj, dp, heap_get, store_get]
}

#[test]
fn allocations_follow_cached_store_and_uncached_heap_cost_models() {
    let small = fixture_counts(64);
    let large = fixture_counts(4096);
    for ((label, baseline), scaled) in [
        "heap scan",
        "heap join",
        "heap probe",
        "store scan",
        "store join",
        "store probe",
        "heap get",
        "store get",
    ]
    .into_iter()
    .zip(small)
    .zip(large)
    {
        if label == "heap scan" || label == "heap join" {
            // 64x cardinality means six doublings, at most two bulk image
            // growth events per doubling (this fixture joins two relations).
            assert!(
                scaled.allocs <= baseline.allocs + 2 * 6,
                "{label}: per-row heap allocation growth"
            );
            assert!(
                scaled.alloc_bytes <= baseline.alloc_bytes * 64,
                "{label}: superlinear image storage"
            );
            continue;
        }
        if label == "store scan" || label == "store join" {
            assert_eq!(
                scaled.allocs, 0,
                "{label}: warmed execution must reuse buffers"
            );
        }
        assert!(
            scaled.allocs <= baseline.allocs,
            "{label}: 64x more rows grew warmed allocations {} -> {}",
            baseline.allocs,
            scaled.allocs
        );
        assert!(
            scaled.alloc_bytes <= baseline.alloc_bytes,
            "{label}: 64x more rows grew warmed allocated bytes {} -> {}",
            baseline.alloc_bytes,
            scaled.alloc_bytes
        );
    }
}
