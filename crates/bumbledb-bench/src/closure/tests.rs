use bumbledb::{Answers, Db, Theory as _, Value};

use crate::compare::{self, Owned};
use crate::corpus_gen::{GenConfig, Scale};
use crate::families::param_args;
use crate::naive::{Delta, NaiveDb, ParamValue};

use super::{ClosSizes, Reachability, all, closure_program, ids, load_stores, relation_rows};

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::Tiny,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-closure-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The corpus shape is what the closed forms say: node/edge counts, the
/// chain prefix, the tree's heap layout.
#[test]
fn the_corpus_shape_is_closed_form() {
    let sizes = ClosSizes::of(Scale::Tiny);
    assert_eq!(sizes.tree_nodes(), (4u64.pow(4) - 1) / 3); // 85
    assert_eq!(sizes.nodes(), 64 + 1 + 85);
    let edges: Vec<Vec<Value>> = relation_rows(sizes, ids::EDGE).collect();
    assert_eq!(edges.len() as u64, sizes.edges());
    assert_eq!(edges[0], vec![Value::U64(0), Value::U64(1)]);
    let base = sizes.tree_base();
    // The first tree edge: root -> first child.
    assert_eq!(
        edges[usize::try_from(sizes.chain).expect("fits")],
        vec![Value::U64(base), Value::U64(base + 1)]
    );
}

/// Naive parity — the semantic oracle for the recursion surface: the
/// engine's fixpoint answers equal [`NaiveDb::program`]'s stratified
/// fixpoint on the Tiny corpus, every family x draw (misses included).
#[test]
fn the_engine_agrees_with_the_naive_fixpoint() {
    let dir = scratch("naive");
    let sizes = ClosSizes::of(CFG.scale);
    let db = Db::create(&dir, Reachability).expect("create");
    for rel in [ids::NODE, ids::EDGE] {
        db.bulk_load(rel, relation_rows(sizes, rel)).expect("load");
    }
    let mut naive = NaiveDb::new(&Reachability.descriptor());
    for rel in [ids::NODE, ids::EDGE] {
        let delta = Delta {
            deletes: vec![],
            inserts: relation_rows(sizes, rel).map(|row| (rel, row)).collect(),
        };
        naive.apply(&delta).expect("naive load");
    }

    let program = closure_program();
    let mut prepared = db.prepare_program(&program).expect("prepare_program");
    let types: Vec<bumbledb::schema::ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    let mut buffer = Answers::new();
    for family in all() {
        for draw in (family.params)(&CFG) {
            let args = param_args(&draw);
            db.read(|snap| snap.execute_args(&mut prepared, &args, &mut buffer))
                .expect("execute");
            let mut ours = compare::from_answers(&buffer, &types);
            ours.sort();
            let model = naive.program(&program, &draw).expect("naive program");
            let mut theirs: Vec<Vec<Owned>> = model
                .into_iter()
                .map(|tuple| {
                    tuple
                        .0
                        .iter()
                        .map(|value| match value {
                            Value::U64(v) => Owned::U64(*v),
                            other => panic!("closure answers are U64, got {other:?}"),
                        })
                        .collect()
                })
                .collect();
            theirs.sort();
            assert_eq!(ours, theirs, "{}: draw {draw:?}", family.name);
        }
    }
    drop(prepared);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// SQLite parity — the recursive CTE mirror is row-identical on the
/// Tiny corpus (the bench lane's verify-before-time, exercised as a
/// test so a mirror bug fails fast, not at bench time).
#[test]
fn the_recursive_cte_mirror_is_row_identical() {
    let dir = scratch("mirror");
    let (db, conn) = load_stores(&dir, CFG, crate::storemode::StoreMode::Durable).expect("stores");
    for family in all() {
        let draws = (family.params)(&CFG);
        super::verify_family(&db, &conn, family, &draws).expect("verify");
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The depth and fanout shapes produce the counts their closed forms
/// promise: closure from the chain head is the whole chain; closure
/// from the tree root is every tree node but the root.
#[test]
fn closure_cardinalities_match_the_shapes() {
    let dir = scratch("counts");
    let sizes = ClosSizes::of(CFG.scale);
    let (db, _conn) = load_stores(&dir, CFG, crate::storemode::StoreMode::Durable).expect("stores");
    let program = closure_program();
    let mut prepared = db.prepare_program(&program).expect("prepare_program");
    let mut buffer = Answers::new();
    let count = |db: &Db<Reachability>,
                 prepared: &mut bumbledb::PreparedQuery<'_, Reachability>,
                 buffer: &mut Answers,
                 anchor: u64| {
        let draw = vec![ParamValue::Scalar(Value::U64(anchor))];
        let args = param_args(&draw);
        db.read(|snap| snap.execute_args(prepared, &args, buffer))
            .expect("execute");
        buffer.len() as u64
    };
    assert_eq!(
        count(&db, &mut prepared, &mut buffer, 0),
        sizes.chain,
        "the chain head reaches the whole chain"
    );
    assert_eq!(
        count(&db, &mut prepared, &mut buffer, sizes.tree_base()),
        sizes.tree_nodes() - 1,
        "the tree root reaches every non-root tree node"
    );
    assert_eq!(
        count(&db, &mut prepared, &mut buffer, sizes.nodes() + 1_000_000),
        0,
        "the miss closes empty"
    );
    drop(prepared);
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
