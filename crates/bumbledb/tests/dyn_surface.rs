//! The schema-generic (dyn) surface and the rejection wire, pinned from
//! the public API (`docs/architecture/70-api.md` § the dyn lane;
//! `docs/architecture/30-dependencies.md` § rendering the rejection):
//!
//! - **The no-panic sweep over the dyn write/read surface** — malformed
//!   arity, wrong value types, non-UTF-8 strings, unknown relation ids,
//!   mis-aimed key-statement ids, and out-of-roster closed handles are
//!   all typed [`bumbledb::FactShapeError`]s (or honest misses), never
//!   panics: ids at this surface are data.
//! - **The dyn fresh-mint lane** — `Db::fresh_field` + `alloc_at`
//!   returns minted ids to the caller; explicit re-supply preserves
//!   identity (the delete+insert idiom).
//! - **The violation wire** — a rejected commit carries decoded cited
//!   facts (str fields minted by the REJECTED transaction included) and
//!   renders, via public API only, into
//!   `[{statement id, canonical spelling, kind, offending facts as named
//!   decoded values}]` — one commit violating a containment and a window
//!   (one rejection, both cited), one violating an FD (keys preempt the
//!   statement phase, so an FD violation is always its own rejection).

mod common;

use bumbledb::schema::render_rejection;
use bumbledb::{
    Error, FactShapeError, RelationId, StatementId, StatementKind, Theory, Value, Violation,
};

bumbledb::schema! {
    pub Graph;

    closed relation Kind as KindId = { Lesson, Assessment };

    relation Node {
        id: u64 as NodeId, fresh,
        title: str,
        kind: u64 as KindId,
    }

    relation Edge {
        src: u64 as NodeId,
        dst: u64 as NodeId,
    }

    Edge(src) <= Node(id);
    Edge(dst) <= Node(id);
    Node(kind) <= Kind(id);
    Node(id) <={0..2} Edge(src);
}

/// Materialized statement ids of the `Graph` theory (fresh auto-keys,
/// closed auto-keys, then declared statements in declaration order).
const NODE_KEY: StatementId = StatementId(0);
const KIND_KEY: StatementId = StatementId(1);
const EDGE_DST_CONTAINMENT: StatementId = StatementId(3);
const OUTDEGREE_WINDOW: StatementId = StatementId(5);

fn node_row(id: u64, title: &str, kind: KindId) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::String(Box::from(title.as_bytes())),
        Value::U64(kind.0),
    ]
}

fn edge_row(src: u64, dst: u64) -> Vec<Value> {
    vec![Value::U64(src), Value::U64(dst)]
}

/// A database seeded entirely through the dyn lane: resolve the fresh
/// witness once, mint per row, insert dynamically — the ETL access
/// pattern, ids returned to the caller.
fn seeded(dir: &common::TempDir, nodes: usize) -> (bumbledb::Db<Graph>, Vec<u64>) {
    let db = bumbledb::Db::create(dir.path(), Graph).expect("create");
    let fresh = db
        .fresh_field(Graph::NODE, Graph::NODE_ID)
        .expect("Node.id is fresh");
    let ids = db
        .write(|tx| {
            (0..nodes)
                .map(|n| {
                    let id = tx.alloc_at(fresh)?;
                    tx.insert_dyn(
                        Graph::NODE,
                        &node_row(id, &format!("node-{n}"), Kind::Lesson.id()),
                    )?;
                    Ok(id)
                })
                .collect::<bumbledb::Result<Vec<u64>>>()
        })
        .expect("seed commit");
    (db, ids)
}

#[test]
fn dyn_fresh_minting_returns_ids_and_explicit_resupply_preserves_identity() {
    let dir = common::TempDir::new("dyn-fresh-mint");
    let (db, ids) = seeded(&dir, 2);
    assert_eq!(ids.len(), 2);
    // Explicit re-supply: the delete+insert identity idiom, entirely dyn.
    db.write(|tx| {
        assert!(tx.delete_dyn(Graph::NODE, &node_row(ids[0], "node-0", Kind::Lesson.id()))?);
        assert!(tx.insert_dyn(
            Graph::NODE,
            &node_row(ids[0], "renamed", Kind::Assessment.id())
        )?);
        Ok(())
    })
    .expect("identity rewrite commits");
    let renamed = db
        .write(|tx| tx.get_dyn(Graph::NODE, NODE_KEY, &[Value::U64(ids[0])]))
        .expect("point read")
        .expect("the row survived under its identity");
    assert_eq!(renamed[1], Value::String(Box::from("renamed".as_bytes())));
    // Minting is monotone past explicit values: the next mint is fresh.
    let next = db
        .write(|tx| {
            let fresh = tx.alloc_at(db.fresh_field(Graph::NODE, Graph::NODE_ID).expect("fresh"))?;
            tx.insert_dyn(Graph::NODE, &node_row(fresh, "next", Kind::Lesson.id()))?;
            Ok(fresh)
        })
        .expect("mint past explicit ids");
    assert!(!ids.contains(&next), "never re-issues an observable id");
}

#[test]
fn a_non_fresh_field_earns_no_witness() {
    let dir = common::TempDir::new("dyn-not-fresh");
    let (db, _) = seeded(&dir, 1);
    let err = db
        .fresh_field(Graph::NODE, Graph::NODE_TITLE)
        .expect_err("title is not fresh");
    assert!(matches!(err, FactShapeError::NotAFreshField { .. }));
}

/// The adversarial sweep over every dyn WRITE entry: unknown relation
/// ids, malformed arity, wrong value types, non-UTF-8 strings — each a
/// typed error, never a panic, on both the insert and delete lanes.
#[test]
fn dyn_writes_refuse_malformed_input_typed_never_panicking() {
    let dir = common::TempDir::new("dyn-write-sweep");
    let (db, ids) = seeded(&dir, 1);
    db.write(|tx| {
        let unknown = RelationId(99);
        let wrong_arity = vec![Value::U64(ids[0])];
        let wrong_type = vec![Value::Bool(true), Value::U64(1), Value::U64(0)];
        let bad_utf8 = vec![
            Value::U64(ids[0]),
            Value::String(Box::from(&[0xFF, 0xFE][..])),
            Value::U64(0),
        ];
        for (values, expect) in [
            (&wrong_arity, "arity"),
            (&wrong_type, "type"),
            (&bad_utf8, "utf8"),
        ] {
            let insert = tx.insert_dyn(Graph::NODE, values).expect_err(expect);
            assert!(matches!(insert, Error::FactShape(_)), "{insert:?}");
            let delete = tx.delete_dyn(Graph::NODE, values).expect_err(expect);
            assert!(matches!(delete, Error::FactShape(_)), "{delete:?}");
        }
        for outcome in [
            tx.insert_dyn(unknown, &[]).expect_err("unknown relation"),
            tx.delete_dyn(unknown, &[]).expect_err("unknown relation"),
        ] {
            assert!(matches!(
                outcome,
                Error::FactShape(FactShapeError::UnknownRelation { .. })
            ));
        }
        // A closed relation refuses writes at entry, typed.
        let closed = tx
            .insert_dyn(Graph::KIND, &[Value::U64(0)])
            .expect_err("ground axioms are never written");
        assert!(matches!(closed, Error::ClosedRelationWrite { .. }));
        Ok(())
    })
    .expect("the sweep commits nothing");
}

/// The same sweep over every dyn READ entry, write-transaction side and
/// snapshot side: point reads take ids as data and answer typed errors
/// or honest misses — including out-of-roster closed handles, which are
/// ABSENT, not errors (an unknown word is a miss; the roster is the
/// relation's extension).
#[test]
fn dyn_point_reads_refuse_malformed_input_and_miss_honestly() {
    let dir = common::TempDir::new("dyn-read-sweep");
    let (db, ids) = seeded(&dir, 1);
    db.write(|tx| {
        assert!(tx.contains_dyn(Graph::NODE, &node_row(ids[0], "node-0", Kind::Lesson.id()))?);
        assert!(!tx.contains_dyn(
            Graph::NODE,
            &node_row(ids[0], "never-interned", Kind::Lesson.id())
        )?);
        // Closed relations answer from the sealed extension.
        assert!(tx.contains_dyn(Graph::KIND, &[Value::U64(Kind::Assessment.id().0)])?);
        assert!(
            !tx.contains_dyn(Graph::KIND, &[Value::U64(7)])?,
            "out of roster = absent"
        );
        let unknown = tx
            .contains_dyn(RelationId(99), &[])
            .expect_err("unknown relation");
        assert!(matches!(
            unknown,
            Error::FactShape(FactShapeError::UnknownRelation { .. })
        ));
        // get_dyn: a mis-aimed statement id is typed — out of range, a
        // containment, or another relation's key.
        for statement in [StatementId(40), EDGE_DST_CONTAINMENT, KIND_KEY] {
            let err = tx
                .get_dyn(Graph::NODE, statement, &[Value::U64(ids[0])])
                .expect_err("not a key of Node");
            assert!(matches!(
                err,
                Error::FactShape(FactShapeError::NotAKeyStatement { .. })
            ));
        }
        let arity = tx
            .get_dyn(Graph::NODE, NODE_KEY, &[])
            .expect_err("empty key tuple");
        assert!(matches!(
            arity,
            Error::FactShape(FactShapeError::ArityMismatch { .. })
        ));
        let ty = tx
            .get_dyn(Graph::NODE, NODE_KEY, &[Value::Bool(true)])
            .expect_err("a bool is no node id");
        assert!(matches!(
            ty,
            Error::FactShape(FactShapeError::TypeMismatch { .. })
        ));
        Ok(())
    })
    .expect("reads commit nothing");

    db.read(|snap| {
        assert!(snap.contains_dyn(Graph::NODE, &node_row(ids[0], "node-0", Kind::Lesson.id()))?);
        assert!(!snap.contains_dyn(
            Graph::NODE,
            &node_row(ids[0], "never-interned", Kind::Lesson.id())
        )?);
        assert!(snap.contains_dyn(Graph::KIND, &[Value::U64(0)])?);
        assert!(!snap.contains_dyn(Graph::KIND, &[Value::U64(7)])?);
        let unknown = snap
            .contains_dyn(RelationId(99), &[])
            .expect_err("unknown relation");
        assert!(matches!(
            unknown,
            Error::FactShape(FactShapeError::UnknownRelation { .. })
        ));
        // The snapshot point read: committed state, decoded values out.
        let row = snap
            .get_dyn(Graph::NODE, NODE_KEY, &[Value::U64(ids[0])])?
            .expect("seeded row");
        assert_eq!(row[1], Value::String(Box::from("node-0".as_bytes())));
        assert_eq!(
            snap.get_dyn(Graph::NODE, NODE_KEY, &[Value::U64(555)])?,
            None
        );
        // A closed relation's key resolves against the sealed extension.
        let kind = snap
            .get_dyn(Graph::KIND, KIND_KEY, &[Value::U64(1)])?
            .expect("Assessment is row 1");
        assert_eq!(kind, vec![Value::U64(1)]);
        assert_eq!(snap.get_dyn(Graph::KIND, KIND_KEY, &[Value::U64(9)])?, None);
        for statement in [StatementId(40), EDGE_DST_CONTAINMENT] {
            let err = snap
                .get_dyn(Graph::NODE, statement, &[Value::U64(ids[0])])
                .expect_err("not a key of Node");
            assert!(matches!(
                err,
                Error::FactShape(FactShapeError::NotAKeyStatement { .. })
            ));
        }
        Ok(())
    })
    .expect("snapshot sweep");
}

/// One commit violating a containment AND a window: ONE rejection, both
/// cited (the statement phase is scan-complete), every offending fact
/// decoded — including the parent node whose `title` was interned BY the
/// rejected transaction (the provisional-id case that forces decode at
/// rejection time), and the whole set rendered through
/// [`render_rejection`] with canonical spellings and named values.
#[test]
fn a_rejection_renders_statement_spelling_kind_and_decoded_facts() {
    let dir = common::TempDir::new("dyn-rejection-render");
    let (db, ids) = seeded(&dir, 3);
    let fresh = db.fresh_field(Graph::NODE, Graph::NODE_ID).expect("fresh");
    let err = db
        .write(|tx| {
            let hub = tx.alloc_at(fresh)?;
            tx.insert_dyn(
                Graph::NODE,
                &node_row(hub, "provisional-title", Kind::Lesson.id()),
            )?;
            for dst in &ids {
                tx.insert_dyn(Graph::EDGE, &edge_row(hub, *dst))?;
            }
            tx.insert_dyn(Graph::EDGE, &edge_row(ids[0], 9999))?;
            Ok(hub)
        })
        .expect_err("three outgoing edges and a dangling target");
    let Error::CommitRejected { violations } = err else {
        panic!("expected a rejection, got {err:?}");
    };

    let cited = violations.as_slice();
    assert!(
        matches!(
            cited,
            [
                Violation::Containment {
                    statement: EDGE_DST_CONTAINMENT,
                    ..
                },
                Violation::Cardinality {
                    statement: OUTDEGREE_WINDOW,
                    count: 3,
                    ..
                }
            ]
        ),
        "both statements cited, in citation order: {cited:?}"
    );
    // The decoded cited facts: the dangling edge, and the hub node whose
    // title only the rejected transaction ever interned.
    let edge = &violations.cited_facts(0)[0];
    assert_eq!(edge.relation, Graph::EDGE);
    assert_eq!(edge.values[1], Value::U64(9999));
    let hub = &violations.cited_facts(1)[0];
    assert_eq!(hub.relation, Graph::NODE);
    assert_eq!(
        hub.values[1],
        Value::String(Box::from("provisional-title".as_bytes())),
        "a provisional intern id decodes at rejection time"
    );

    let rendered = render_rejection(&Graph.descriptor(), &violations);
    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0].statement, EDGE_DST_CONTAINMENT);
    assert_eq!(rendered[0].kind, StatementKind::Containment);
    assert_eq!(rendered[0].spelling, "Edge(dst) <= Node(id)");
    assert_eq!(rendered[0].facts[0].relation.as_ref(), "Edge");
    assert_eq!(
        rendered[0].facts[0].fields[1],
        ("dst".into(), Value::U64(9999))
    );
    assert_eq!(rendered[1].statement, OUTDEGREE_WINDOW);
    assert_eq!(rendered[1].kind, StatementKind::Cardinality);
    assert_eq!(rendered[1].spelling, "Node(id) <={0..2} Edge(src)");
    assert_eq!(rendered[1].count, Some(3));
    assert_eq!(rendered[1].facts[0].relation.as_ref(), "Node");
    assert_eq!(
        rendered[1].facts[0].fields[1],
        (
            "title".into(),
            Value::String(Box::from("provisional-title".as_bytes()))
        )
    );
}

/// The FD form's rendering. Key violations preempt the statement phase
/// (the containment probes are defined over the keyed final state), so
/// an FD conviction is always its own rejection — one commit per phase,
/// all three forms covered between this test and the one above.
#[test]
fn an_fd_rejection_renders_the_key_form() {
    let dir = common::TempDir::new("dyn-rejection-fd");
    let (db, ids) = seeded(&dir, 1);
    let err = db
        .write(|tx| {
            tx.insert_dyn(Graph::NODE, &node_row(ids[0], "usurper", Kind::Lesson.id()))?;
            Ok(())
        })
        .expect_err("two live facts claim one key");
    let Error::CommitRejected { violations } = err else {
        panic!("expected a rejection, got {err:?}");
    };
    let cited = violations.as_slice();
    assert!(
        matches!(
            cited,
            [Violation::Functionality {
                statement: NODE_KEY,
                ..
            }]
        ),
        "one key citation: {cited:?}"
    );
    let fact = &violations.cited_facts(0)[0];
    assert_eq!(fact.relation, Graph::NODE);
    assert_eq!(fact.values[0], Value::U64(ids[0]));
    assert_eq!(
        fact.values[1],
        Value::String(Box::from("usurper".as_bytes()))
    );

    let rendered = render_rejection(&Graph.descriptor(), &violations);
    assert_eq!(rendered[0].kind, StatementKind::Functionality);
    assert_eq!(rendered[0].spelling, "Node(id) -> Node");
    assert_eq!(rendered[0].direction, None);
    assert_eq!(
        rendered[0].facts[0].fields[0],
        ("id".into(), Value::U64(ids[0]))
    );
}

/// The manifest carries every statement's id, kind, and canonical
/// spelling — the name→id tables a binding caches at open now cover
/// statements, so a foreign host cites any statement id without a Rust
/// renderer in reach.
#[test]
fn the_manifest_names_every_statement_in_canonical_spelling() {
    let manifest = Graph.manifest();
    let statements = &manifest.statements;
    assert_eq!(statements.len(), 6);
    let expect: [(StatementKind, &str); 6] = [
        (StatementKind::Functionality, "Node(id) -> Node"),
        (StatementKind::Functionality, "Kind(id) -> Kind"),
        (StatementKind::Containment, "Edge(src) <= Node(id)"),
        (StatementKind::Containment, "Edge(dst) <= Node(id)"),
        (StatementKind::Containment, "Node(kind) <= Kind(id)"),
        (StatementKind::Cardinality, "Node(id) <={0..2} Edge(src)"),
    ];
    for (idx, (kind, spelling)) in expect.into_iter().enumerate() {
        assert_eq!(
            statements[idx].id,
            StatementId(u16::try_from(idx).expect("fits"))
        );
        assert_eq!(statements[idx].kind, kind, "statement {idx}");
        assert_eq!(statements[idx].spelling, spelling, "statement {idx}");
    }
    // The extension table still rides per relation — handles as data.
    let rows = manifest.relations[0]
        .extension
        .as_ref()
        .expect("Kind is closed");
    assert_eq!(rows[1].handle.as_ref(), "Assessment");
}
