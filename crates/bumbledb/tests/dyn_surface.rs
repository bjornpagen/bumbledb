mod common;

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::schema::render_rejection;
use bumbledb::{
    DynIdError, Error, FactShapeError, RelationId, StatementId, StatementKind, Theory, Value,
    Violation,
};

fn graph_schema() -> bumbledb::Schema {
    Graph
        .descriptor()
        .validate()
        .expect("the test schema is valid")
}

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

const NODE_KEY: StatementId = StatementId(0);
const KIND_KEY: StatementId = StatementId(1);
const EDGE_DST_CONTAINMENT: StatementId = StatementId(3);
const OUTDEGREE_CAPACITY: StatementId = StatementId(5);

fn node_row(id: u64, title: &str, kind: KindId) -> Vec<Value> {
    vec![
        Value::U64(id),
        Value::String(title.into()),
        Value::U64(kind.0),
    ]
}

fn edge_row(src: u64, dst: u64) -> Vec<Value> {
    vec![Value::U64(src), Value::U64(dst)]
}

fn seeded(dir: &common::TempDir, nodes: usize) -> (bumbledb::Db<Graph>, Vec<u64>) {
    let db = bumbledb::Db::create(dir.path(), Graph)
        .expect("create")
        .expect("accepted");
    let fresh = db
        .fresh_field(Graph::NODE, Graph::NODE_ID)
        .expect("Node.id is fresh");
    let ids = db
        .write(|tx| {
            (0..nodes)
                .map(|n| {
                    let id = tx.reserve_at(fresh, 1)?.start().expect("nonempty");
                    tx.insert_dyn(
                        Graph::NODE,
                        [&node_row(id, &format!("node-{n}"), Kind::Lesson.id())],
                    )?;
                    Ok(id)
                })
                .collect::<bumbledb::Result<Vec<u64>>>()
        })
        .expect("seed commit")
        .unwrap()
        .value;
    (db, ids)
}

#[test]
fn dyn_fresh_minting_returns_ids_and_explicit_resupply_preserves_identity() {
    let dir = common::TempDir::new("dyn-fresh-mint");
    let (db, ids) = seeded(&dir, 2);
    assert_eq!(ids.len(), 2);

    db.write(|tx| {
        assert_eq!(
            tx.delete_dyn(
                Graph::NODE,
                [&node_row(ids[0], "node-0", Kind::Lesson.id())]
            )?
            .changed(),
            1
        );
        assert_eq!(
            tx.insert_dyn(
                Graph::NODE,
                [&node_row(ids[0], "renamed", Kind::Assessment.id())]
            )?
            .changed(),
            1
        );
        Ok(())
    })
    .expect("identity rewrite commits")
    .unwrap();
    let renamed = db
        .write(|tx| tx.get_dyn(Graph::NODE, NODE_KEY, &[Value::U64(ids[0])]))
        .expect("point read")
        .unwrap()
        .value
        .expect("the row survived under its identity");
    assert_eq!(renamed[1], Value::String(Box::from("renamed")));

    let next = db
        .write(|tx| {
            let fresh = tx
                .reserve_at(
                    db.fresh_field(Graph::NODE, Graph::NODE_ID).expect("fresh"),
                    1,
                )?
                .start()
                .expect("nonempty");
            tx.insert_dyn(Graph::NODE, [&node_row(fresh, "next", Kind::Lesson.id())])?;
            Ok(fresh)
        })
        .expect("mint past explicit ids")
        .unwrap()
        .value;
    assert!(!ids.contains(&next), "never re-issues an observable id");
}

#[test]
fn a_non_fresh_field_earns_no_witness() {
    let dir = common::TempDir::new("dyn-not-fresh");
    let (db, _) = seeded(&dir, 1);
    let err = db
        .fresh_field(Graph::NODE, Graph::NODE_TITLE)
        .expect_err("title is not fresh");
    assert!(matches!(
        err,
        FactShapeError::Id(DynIdError::NotAFreshField { .. })
    ));
}

#[test]
fn dyn_writes_refuse_malformed_input_typed_never_panicking() {
    let dir = common::TempDir::new("dyn-write-sweep");
    let (db, ids) = seeded(&dir, 1);
    db.write(|tx| {
        let unknown = RelationId(99);
        let wrong_arity = vec![Value::U64(ids[0])];
        let wrong_type = vec![Value::Bool(true), Value::U64(1), Value::U64(0)];
        for (values, expect) in [(&wrong_arity, "arity"), (&wrong_type, "type")] {
            let insert = tx.insert_dyn(Graph::NODE, [values]).expect_err(expect);
            assert!(matches!(insert, Error::FactShape(_)), "{insert:?}");
            let delete = tx.delete_dyn(Graph::NODE, [values]).expect_err(expect);
            assert!(matches!(delete, Error::FactShape(_)), "{delete:?}");
        }
        for outcome in [
            tx.insert_dyn(unknown, [&[]]).expect_err("unknown relation"),
            tx.delete_dyn(unknown, [&[]]).expect_err("unknown relation"),
        ] {
            assert!(matches!(
                outcome,
                Error::FactShape(FactShapeError::Id(DynIdError::UnknownRelation { .. }))
            ));
        }

        let closed = tx
            .insert_dyn(Graph::KIND, [&[Value::U64(0)]])
            .expect_err("ground axioms are never written");
        assert!(matches!(closed, Error::ClosedRelationWrite { .. }));
        Ok(())
    })
    .expect("the sweep commits nothing")
    .unwrap();
}

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
            Error::FactShape(FactShapeError::Id(DynIdError::UnknownRelation { .. }))
        ));

        for statement in [StatementId(40), EDGE_DST_CONTAINMENT, KIND_KEY] {
            let err = tx
                .get_dyn(Graph::NODE, statement, &[Value::U64(ids[0])])
                .expect_err("not a key of Node");
            assert!(matches!(
                err,
                Error::FactShape(FactShapeError::Id(DynIdError::NotAKeyStatement { .. }))
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
    .expect("reads commit nothing")
    .unwrap();

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
            Error::FactShape(FactShapeError::Id(DynIdError::UnknownRelation { .. }))
        ));

        let row = snap
            .get_dyn(Graph::NODE, NODE_KEY, &[Value::U64(ids[0])])?
            .expect("seeded row");
        assert_eq!(row[1], Value::String(Box::from("node-0")));
        assert_eq!(
            snap.get_dyn(Graph::NODE, NODE_KEY, &[Value::U64(555)])?,
            None
        );

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
                Error::FactShape(FactShapeError::Id(DynIdError::NotAKeyStatement { .. }))
            ));
        }
        Ok(())
    })
    .expect("snapshot sweep");
}

#[test]
fn a_rejection_renders_statement_spelling_kind_and_decoded_facts() {
    let dir = common::TempDir::new("dyn-rejection-render");
    let (db, ids) = seeded(&dir, 3);
    let fresh = db.fresh_field(Graph::NODE, Graph::NODE_ID).expect("fresh");
    let violations = common::expect_rejected(db.write(|tx| {
        let hub = tx.reserve_at(fresh, 1)?.start().expect("nonempty");
        tx.insert_dyn(
            Graph::NODE,
            [&node_row(hub, "provisional-title", Kind::Lesson.id())],
        )?;
        for dst in &ids {
            tx.insert_dyn(Graph::EDGE, [&edge_row(hub, *dst)])?;
        }
        tx.insert_dyn(Graph::EDGE, [&edge_row(ids[0], 9999)])?;
        Ok(hub)
    }));

    let cited = violations.as_slice();
    assert!(
        matches!(
            cited,
            [
                (Violation::Containment { .. }, _),
                (Violation::Capacity { measure: 3, .. }, _),
            ]
        ) && violations.get(0).unwrap().statement_id(&graph_schema()) == EDGE_DST_CONTAINMENT
            && violations.get(1).unwrap().statement_id(&graph_schema()) == OUTDEGREE_CAPACITY,
        "both statements cited, in citation order: {cited:?}"
    );

    let edge = &violations.cited_facts(0)[0];
    assert_eq!(edge.relation(), Graph::EDGE);
    assert_eq!(edge.values()[1], Value::U64(9999));
    let hub = &violations.cited_facts(1)[0];
    assert_eq!(hub.relation(), Graph::NODE);
    assert_eq!(
        hub.values()[1],
        Value::String(Box::from("provisional-title")),
        "a provisional intern id decodes at rejection time"
    );

    let rendered = render_rejection(&Graph.descriptor(), &violations);
    assert_eq!(rendered.len(), 2);
    assert_eq!(rendered[0].statement(), EDGE_DST_CONTAINMENT);
    assert_eq!(rendered[0].kind(), StatementKind::Containment);
    assert_eq!(rendered[0].spelling(), "Edge(dst) <= Node(id)");
    assert_eq!(rendered[0].facts()[0].relation.as_ref(), "Edge");
    assert_eq!(
        rendered[0].facts()[0].fields[1],
        ("dst".into(), Value::U64(9999))
    );
    assert_eq!(rendered[1].statement(), OUTDEGREE_CAPACITY);
    assert_eq!(rendered[1].kind(), StatementKind::Capacity);
    assert_eq!(rendered[1].spelling(), "Node(id) <={0..2} Edge(src)");
    assert_eq!(rendered[1].measure(), Some(3));
    assert_eq!(rendered[1].facts()[0].relation.as_ref(), "Node");
    assert_eq!(
        rendered[1].facts()[0].fields[1],
        (
            "title".into(),
            Value::String(Box::from("provisional-title"))
        )
    );
}

#[test]
fn an_fd_rejection_renders_the_key_form() {
    let dir = common::TempDir::new("dyn-rejection-fd");
    let (db, ids) = seeded(&dir, 1);
    let violations = common::expect_rejected(db.write(|tx| {
        tx.insert_dyn(
            Graph::NODE,
            [&node_row(ids[0], "usurper", Kind::Lesson.id())],
        )?;
        Ok(())
    }));
    let cited = violations.as_slice();
    assert!(
        matches!(
            cited,
            [(Violation::Functionality { .. }, _)]
                if violations.get(0).unwrap().statement_id(&graph_schema()) == NODE_KEY
        ),
        "one key citation: {cited:?}"
    );
    let fact = &violations.cited_facts(0)[0];
    assert_eq!(fact.relation(), Graph::NODE);
    assert_eq!(fact.values()[0], Value::U64(ids[0]));
    assert_eq!(fact.values()[1], Value::String(Box::from("usurper")));

    let rendered = render_rejection(&Graph.descriptor(), &violations);
    assert_eq!(rendered[0].kind(), StatementKind::Functionality);
    assert_eq!(rendered[0].spelling(), "Node(id) -> Node");
    assert_eq!(rendered[0].direction(), None);
    assert_eq!(
        rendered[0].facts()[0].fields[0],
        ("id".into(), Value::U64(ids[0]))
    );
}

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
        (StatementKind::Capacity, "Node(id) <={0..2} Edge(src)"),
    ];
    for (idx, (kind, spelling)) in expect.into_iter().enumerate() {
        assert_eq!(
            statements[idx].id,
            StatementId(u16::try_from(idx).expect("fits"))
        );
        assert_eq!(statements[idx].kind, kind, "statement {idx}");
        assert_eq!(statements[idx].spelling, spelling, "statement {idx}");
    }

    let rows = manifest.relations[0]
        .extension
        .as_ref()
        .expect("Kind is closed");
    assert_eq!(rows[1].handle.as_ref(), "Assessment");
}
