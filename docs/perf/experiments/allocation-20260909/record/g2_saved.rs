use super::*;
use crate::ParamId;
use crate::exec::colt::m1_observer::{Snapshot, clones, reset_clones};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/m1-source/crates/bumbledb-bench/src/schema.rs"]
mod saved_schema;

const DRAWS: [(u64, u64); 4] = [(1, 6), (167, 172), (333, 338), (500, 500)];

fn query() -> Query {
    let v = |id| Term::Var(VarId(id));
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom { source: AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(2), v(0)), (FieldId(3), v(1))] },
            Atom { source: AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(1), v(2)), (FieldId(3), v(1))] },
            Atom { source: AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(1), v(2)), (FieldId(2), v(0))] },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison { op: CmpOp::Ge, lhs: v(0), rhs: Term::Param(ParamId(0)) }),
            ConditionTree::Leaf(Comparison { op: CmpOp::Lt, lhs: v(0), rhs: Term::Param(ParamId(1)) }),
        ],
    })
}

fn open(path: &Path, work: &crate::WorkContext) -> crate::Db<saved_schema::Ledger> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match crate::Db::open(path, saved_schema::Ledger, work.clone()) {
            Ok(db) => return db,
            Err(crate::Error::EnvironmentLocked) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("saved database open: {error:?}"),
        }
    }
}

fn report(label: &str, prepared: &PreparedQuery<saved_schema::Ledger>) -> BTreeMap<(usize, usize), Snapshot> {
    let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else { panic!("triangle uses Free Join"); };
    let mut snapshots = BTreeMap::new();
    for (occ, colt) in rule.memo.colts.iter().enumerate() {
        snapshots.insert((occ, 0), colt.m1_saved_snapshot());
        for (slot, parked) in rule.memo.occs[occ].parked.iter().enumerate() {
            if let Some(parked) = parked {
                snapshots.insert((occ, slot + 1), parked.colt.m1_saved_snapshot());
            }
        }
    }
    let mut total = [0; 4];
    for ((occ, slot), snap) in &snapshots {
        println!("COLT {label} occ={occ} slot={slot} rows={} maps={} owners={:?} table_bytes={} arena_bytes={} capacity_bytes={} retained={} histogram={:?}",
            snap.rows, snap.map_count, snap.owners, snap.table_bytes, snap.arena_bytes,
            snap.capacity_bytes, snap.all_pool_retained, snap.histogram);
        println!("SCRATCH {label} occ={occ} slot={slot} len={} capacity={} bytes={}",
            snap.scratch.0, snap.scratch.1, snap.scratch.1 * 8);
        total[0] += snap.table_bytes;
        total[1] += snap.arena_bytes;
        total[2] += snap.capacity_bytes;
        total[3] += snap.all_pool_retained;
    }
    println!("TOTAL {label} colts={} table_bytes={} arena_bytes={} capacity_bytes={} retained={} retired={}",
        snapshots.len(), total[0], total[1], total[2], total[3], total[1] - total[0]);
    snapshots
}

fn execute(label: &str, db: &crate::Db<saved_schema::Ledger>, prepared: &mut PreparedQuery<saved_schema::Ledger>,
           work: &crate::WorkContext, draw: (u64, u64), expected: &[u64], out: &mut Answers) -> BTreeMap<(usize, usize), Snapshot> {
    let args = [BindValue::U64(draw.0), BindValue::U64(draw.1)];
    reset_clones();
    crate::alloc_counter::reset();
    db.read(work.clone(), |snap| snap.execute(prepared, &args, out)).unwrap();
    let alloc = crate::alloc_counter::snapshot().window;
    let clone_cost = clones();
    let mut actual: Vec<_> = (0..out.len()).map(|row| {
        let AnswerValue::U64(value) = out.get(row, 0) else { panic!("u64 account"); };
        value
    }).collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
    println!("EXEC {label} lo={} hi={} answers={} alloc={alloc:?} clones={clone_cost:?}", draw.0, draw.1, out.len());
    if label.starts_with("warm-") || label.starts_with("repeat-") {
        assert_eq!(alloc.allocs, 0);
        assert_eq!(alloc.deallocs, 0);
        assert_eq!(alloc.alloc_bytes, 0);
        assert_eq!(alloc.dealloc_bytes, 0);
        assert_eq!(clone_cost, (0, 0, 0));
    }
    let snapshots = report(label, prepared);
    println!("PASS {label}");
    snapshots
}

#[test]
#[ignore = "saved triangle map ownership; no timings or full CPU trace"]
fn saved_triangle_ownership() {
    assert!(cfg!(feature = "alloc-counter"));
    let path = std::env::var_os("BUMBLEDB_M1_DB").unwrap();
    let path = Path::new(&path);
    let oracle = std::fs::read_to_string(std::env::var_os("BUMBLEDB_M1_ORACLE").unwrap()).unwrap();
    let mut expected: BTreeMap<u64, Vec<u64>> = DRAWS.iter().map(|&(lo, _)| (lo, Vec::new())).collect();
    for line in oracle.lines() {
        let (lo, account) = line.split_once(',').unwrap();
        expected.get_mut(&lo.parse().unwrap()).unwrap().push(account.parse().unwrap());
    }
    let work = crate::WorkContext::new();
    for draw in DRAWS {
        let db = open(path, &work);
        let mut prepared = db.prepare(&query(), work.clone()).unwrap();
        let mut out = Answers::new();
        let cold = execute(&format!("cold-{}", draw.0), &db, &mut prepared, &work, draw, &expected[&draw.0], &mut out);
        let warm = execute(&format!("warm-{}", draw.0), &db, &mut prepared, &work, draw, &expected[&draw.0], &mut out);
        assert_eq!(cold, warm);
        let repeat = execute(&format!("repeat-{}", draw.0), &db, &mut prepared, &work, draw, &expected[&draw.0], &mut out);
        assert_eq!(warm, repeat);
    }
    let db = open(path, &work);
    let mut prepared = db.prepare(&query(), work.clone()).unwrap();
    let mut out = Answers::new();
    for cycle in 0..2 {
        for draw in DRAWS {
            execute(&format!("rotation{cycle}-{}", draw.0), &db, &mut prepared, &work, draw, &expected[&draw.0], &mut out);
        }
    }
    println!("PASS saved triangle ownership; 20 exact SQL windows");
}
