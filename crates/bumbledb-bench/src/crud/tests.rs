use bumbledb::FieldId;

use crate::compare::Owned;
use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::poststate;

use super::{CrudSizes, ids};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-crud-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The declared world seals: two relations whose id constants match
/// descriptor order, and both scalar key statements — `Doc(key)` and
/// `Counter(key)` — present in the sealed statement roster (the upsert
/// lane's `ON CONFLICT` targets on the mirror).
#[test]
fn the_crud_schema_validates_and_names_its_ids() {
    let schema = super::schema();
    assert_eq!(schema.relations().len(), 2, "two crud relations");
    assert_eq!(schema.relation(ids::DOC).name(), "Doc");
    assert_eq!(schema.relation(ids::COUNTER).name(), "Counter");
    let keys = schema.keys();
    assert!(
        keys.iter()
            .any(|key| key.relation == ids::DOC && *key.projection == [FieldId(1)]),
        "Doc(key) -> Doc is sealed"
    );
    assert!(
        keys.iter()
            .any(|key| key.relation == ids::COUNTER && *key.projection == [FieldId(0)]),
        "Counter(key) -> Counter is sealed"
    );
}

/// Both durability lanes load value-identical twins at `Tiny`, judged
/// by the shared post-state comparator — the exact fold every write
/// lane will reuse.
#[test]
fn the_twin_stores_load_value_identical_at_tiny() {
    let sizes = CrudSizes::of(Scale::Tiny);
    for lane in duralane::ALL {
        let dir = scratch(&format!("twin-{}", lane.label()));
        let (db, conn) = super::corpus::load_stores(&dir, 7, sizes, lane).unwrap_or_else(|e| {
            panic!("{}: {e}", lane.label());
        });
        for (rel, expected) in [
            (ids::DOC, sizes.docs + sizes.delete_pool),
            (ids::COUNTER, sizes.counters),
        ] {
            let name = super::schema().relation(rel).name();
            let ours = poststate::engine_rows(&db, rel).expect("engine rows");
            let theirs =
                poststate::sqlite_rows(&conn, super::schema().relation(rel)).expect("mirror rows");
            assert_eq!(ours.len() as u64, expected, "{name}: engine row count");
            assert_eq!(theirs.len() as u64, expected, "{name}: mirror row count");
            poststate::assert_identical("crud", name, ours, theirs).expect(name);
        }
        drop((db, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// A cross-matched twin is caught by the parity readback, naming the
/// pragma: a Durable-configured mirror judged as `Nosync` errs on
/// `synchronous`, and vice versa.
#[test]
fn the_lane_parity_assertion_catches_a_mismatched_synchronous() {
    let dir = scratch("parity-mismatch");
    let conn = rusqlite::Connection::open(dir.join("durable.sqlite")).expect("open");
    DurabilityLane::Durable.configure(&conn).expect("configure");
    let err = DurabilityLane::Nosync
        .assert_parity(&conn)
        .expect_err("a durable mirror is not a nosync twin");
    assert!(err.contains("synchronous"), "{err}");
    drop(conn);

    let conn = rusqlite::Connection::open(dir.join("nosync.sqlite")).expect("open");
    DurabilityLane::Nosync.configure(&conn).expect("configure");
    let err = DurabilityLane::Durable
        .assert_parity(&conn)
        .expect_err("a nosync mirror is not a durable twin");
    assert!(err.contains("synchronous"), "{err}");
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A one-row post-state divergence is loud: the error names the world
/// and the relation before rendering the multiset diff.
#[test]
fn poststate_divergence_is_loud() {
    let ours = vec![
        vec![Owned::U64(1), Owned::I64(10)],
        vec![Owned::U64(2), Owned::I64(20)],
    ];
    let theirs = vec![
        vec![Owned::U64(1), Owned::I64(10)],
        vec![Owned::U64(2), Owned::I64(21)],
    ];
    let err = poststate::assert_identical("crud", "Doc", ours, theirs)
        .expect_err("the post-states diverge");
    assert!(err.contains("crud/Doc"), "{err}");
    assert!(err.contains("POST-STATES DIVERGE"), "{err}");
}
