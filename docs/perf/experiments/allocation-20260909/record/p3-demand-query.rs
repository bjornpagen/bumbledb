use super::*;
use crate::interval::overlap::p3_demand;

crate::schema! {
    pub Temporal;
    relation Key { id: u64 as TpKeyId, }
    relation Span { id: u64 as TpSpanId, key: u64 as TpKeyId, span: interval<i64>, weight: i64, }
    Key(id) -> Key;
    Span(id) -> Span;
    Span(key) <= Key(id);
}

#[test]
#[ignore = "saved input demand census and correctness replay; no timing"]
fn saved_flat_filter_demand() {
    let path = std::env::var_os("BUMBLEDB_DEMAND_DB").unwrap();
    let output = std::env::var_os("BUMBLEDB_DEMAND_OUT").unwrap();
    let expected: u64 = std::env::var("BUMBLEDB_DEMAND_ANSWER")
        .unwrap()
        .parse()
        .unwrap();
    let work = crate::WorkContext::new();
    let db = crate::Db::open(std::path::Path::new(&path), Temporal, work.clone()).unwrap();
    let v = |id| Term::Var(VarId(id));
    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), v(0)), (FieldId(1), v(2)), (FieldId(2), v(3))],
            },
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), v(1)), (FieldId(1), v(2)), (FieldId(2), v(4))],
            },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: v(0),
                rhs: v(1),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Allen {
                    mask: crate::AllenMask::INTERSECTS,
                },
                lhs: v(3),
                rhs: v(4),
            }),
        ],
    });
    let mut prepared = db.prepare(&query, work.clone()).unwrap();
    let mut out = Answers::new();
    let mut prior: Option<Vec<p3_demand::Sample>> = None;
    for label in ["cold", "warm"] {
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
            panic!("saved plan");
        };
        let initial_capacity = rule.executor.overlap_hit_capacity();
        p3_demand::begin(150034);
        db.read(work.clone(), |snap| {
            snap.execute(&mut prepared, &[] as &[BindValue], &mut out)
        })
        .unwrap();
        let samples = p3_demand::finish();
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), AnswerValue::U64(expected));
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
            panic!("saved plan");
        };
        rule.executor.audit_overlap_demand(
            label,
            &samples,
            initial_capacity,
            (label == "warm").then_some(std::path::Path::new(&output)),
        );
        if let Some(prior) = prior {
            let without_capacity = |rows: &[p3_demand::Sample]| p3_demand::identities(rows);
            assert_eq!(without_capacity(&samples), without_capacity(&prior));
        }
        prior = Some(samples);
        println!("PASS {label} count={expected}");
    }
}
