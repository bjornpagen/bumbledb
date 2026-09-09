use super::*;
use crate::ParamId;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q1-source/crates/bumbledb-bench/src/schema.rs"]
mod saved_schema;

const TRIANGLE: [(i64, i64); 4] = [(1, 6), (167, 172), (333, 338), (500, 500)];
const AT_BASE: i64 = 1_700_000_000_000_000;
const SPAN: i64 = 100_000 * 50;

fn query(range: bool) -> Query {
    let v = |id| Term::Var(VarId(id));
    let (finds, atoms, bound) = if range {
        (vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
         vec![Atom { source: AtomSource::Edb(RelationId(4)),
             bindings: vec![(FieldId(0), v(0)), (FieldId(4), v(1)), (FieldId(5), v(2))] }], v(2))
    } else {
        (vec![FindTerm::Var(VarId(0))], vec![
            Atom { source: AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(2), v(0)), (FieldId(3), v(1))] },
            Atom { source: AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(1), v(2)), (FieldId(3), v(1))] },
            Atom { source: AtomSource::Edb(RelationId(4)),
                bindings: vec![(FieldId(1), v(2)), (FieldId(2), v(0))] },
        ], v(0))
    };
    Query::single(Rule { finds, atoms, negated: vec![], conditions: vec![
        ConditionTree::Leaf(Comparison { op: CmpOp::Ge, lhs: bound.clone(), rhs: Term::Param(ParamId(0)) }),
        ConditionTree::Leaf(Comparison { op: CmpOp::Lt, lhs: bound, rhs: Term::Param(ParamId(1)) }),
    ] })
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

fn owners(label: &str, prepared: &PreparedQuery<saved_schema::Ledger>, out: &Answers) {
    let size = std::mem::size_of::<Cell>();
    println!("ANSWER {label} rows={} arity={} cell_size={size} cell_align={} cells={:?} cell_live_bytes={} cell_capacity_bytes={} text={:?} blob={:?}",
        out.len(), out.arity(), std::mem::align_of::<Cell>(),
        (out.cells.len(), out.cells.capacity()), size * out.cells.len(), size * out.cells.capacity(),
        (out.text.len(), out.text.capacity()), (out.blob.len(), out.blob.capacity()));
    let EitherSink::Projection(sink) = &prepared.sink else { panic!("projection expected"); };
    sink.q1_report(label);
    let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else { panic!("Free Join expected"); };
    let mut survivor_capacity = 0;
    for (occ, colt) in rule.memo.colts.iter().enumerate() {
        for (slot, colt, filters) in std::iter::once((0, colt, &rule.resolved_filters[occ]))
            .chain(rule.memo.occs[occ].parked.iter().enumerate().filter_map(|(slot, parked)| {
                parked.as_ref().map(|p| (slot + 1, &p.colt, &p.bound.filters))
            })) {
            let view = colt.q1_view();
            let Some(bound) = view.bound() else { continue; };
            let survivors = match bound {
                crate::image::view::BoundView::Survivors { positions, .. } => {
                    survivor_capacity += 4 * positions.capacity();
                    Some((positions.len(), positions.capacity()))
                }
                crate::image::view::BoundView::All(_) => None,
            };
            let seed = crate::image::view::q1_observer::seed(bound.image(), filters);
            println!("VIEW {label} occ={occ} slot={slot} image_rows={} survivors={survivors:?} seed={seed:?} filters={filters:?}", bound.image().row_count());
        }
        let spare = &rule.memo.occs[occ].spare;
        survivor_capacity += 4 * spare.capacity();
        println!("SPARE {label} occ={occ} len={} capacity={}", spare.len(), spare.capacity());
    }
    println!("SURVIVORS {label} all_active_parked_spare_capacity_bytes={survivor_capacity}");
}

fn execute(label: &str, db: &crate::Db<saved_schema::Ledger>, prepared: &mut PreparedQuery<saved_schema::Ledger>,
           work: &crate::WorkContext, args: &[BindValue], expected: &[Vec<i64>], out: &mut Answers) {
    crate::alloc_counter::reset();
    db.read(work.clone(), |snap| snap.execute(prepared, args, out)).unwrap();
    let alloc = crate::alloc_counter::snapshot().window;
    let mut actual: Vec<Vec<i64>> = (0..out.len()).map(|row| (0..out.arity()).map(|col| match out.get(row, col) {
        AnswerValue::U64(value) => i64::try_from(value).unwrap(),
        AnswerValue::I64(value) => value,
        value => panic!("unexpected {value:?}"),
    }).collect()).collect();
    actual.sort_unstable();
    assert_eq!(actual, expected);
    if label.starts_with("warm-") || label.starts_with("rotation1-") {
        assert_eq!((alloc.allocs, alloc.deallocs, alloc.alloc_bytes, alloc.dealloc_bytes), (0, 0, 0, 0));
    }
    println!("EXEC {label} alloc={alloc:?}");
    owners(label, prepared, out);
    println!("PASS {label}");
}

#[test]
#[ignore = "saved output and survivor ownership; no CPU trace or timing"]
fn saved_output_and_survivors() {
    assert!(cfg!(feature = "alloc-counter"));
    let path = std::env::var_os("BUMBLEDB_Q1_DB").unwrap();
    let path = Path::new(&path);
    let oracle = std::fs::read_to_string(std::env::var_os("BUMBLEDB_Q1_ORACLE").unwrap()).unwrap();
    let mut expected: BTreeMap<(String, usize), Vec<Vec<i64>>> = BTreeMap::new();
    for family in ["range", "triangle"] {
        for i in 0..4 { expected.insert((family.to_owned(), i), Vec::new()); }
    }
    for line in oracle.lines() {
        let fields: Vec<_> = line.split(',').collect();
        expected.get_mut(&(fields[0].to_owned(), fields[1].parse().unwrap())).unwrap()
            .push(fields[2..].iter().map(|v| v.parse().unwrap()).collect());
    }
    let work = crate::WorkContext::new();
    for family in ["range", "triangle"] {
        let range = family == "range";
        let draws: Vec<_> = (0..4).map(|i| if range {
            let lo = AT_BASE + SPAN * (2 * i as i64 + 1) / 16;
            [BindValue::I64(lo), BindValue::I64(lo + SPAN / 50)]
        } else { [BindValue::U64(TRIANGLE[i].0 as u64), BindValue::U64(TRIANGLE[i].1 as u64)] }).collect();
        for (i, args) in draws.iter().enumerate() {
            let db = open(path, &work);
            let mut prepared = db.prepare(&query(range), work.clone()).unwrap();
            let mut out = Answers::new();
            for kind in ["cold", "warm"] {
                execute(&format!("{kind}-{family}-{i}"), &db, &mut prepared, &work, args,
                    &expected[&(family.to_owned(), i)], &mut out);
            }
        }
        let db = open(path, &work);
        let mut prepared = db.prepare(&query(range), work.clone()).unwrap();
        let mut out = Answers::new();
        for cycle in 0..2 {
            for (i, args) in draws.iter().enumerate() {
                execute(&format!("rotation{cycle}-{family}-{i}"), &db, &mut prepared, &work, args,
                    &expected[&(family.to_owned(), i)], &mut out);
            }
        }
    }
    println!("PASS saved output and survivor ownership; 32 exact SQL windows");
}
