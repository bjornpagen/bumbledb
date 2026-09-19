//! Query-local positive binders retain row inputs while iterating predicate values.
use bumbledb::{
    AnswerValue, BindValue, Db, Event, EventExpr as E, EventExprError, EventImport, EventScope,
    Fact, FixedPointKind as K, PredicateDepth, RelationExpr as R, RelationProductOp,
    RelationViewOp,
    event::{
        AdmittedDescriptor, BoolOp4, CoordinateMap, DescriptorLimits, FibreProduct, ModalOp,
        RelationalProduct, Space, SpaceId,
    },
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::FieldId,
};
mod common;

bumbledb::schema! {
    pub BinderQueries;
    relation Edge { id: u64, when: event }
    relation Goal { id: u64, when: event }
}

fn table(s: &Space, bits: u64) -> Event {
    s.table((1 << s.dimensions()) - 1, &[bits], &()).unwrap()
}
fn scope(s: &Space) -> EventScope {
    let scope = EventScope::capture(s, &()).unwrap();
    EventScope::from_bytes(scope.bytes(), &()).unwrap()
}
fn import(v: &AdmittedDescriptor) -> EventImport {
    EventImport::capture(v, DescriptorLimits::default(), &()).unwrap()
}
fn bound(depth: u16) -> E {
    E::Bound(PredicateDepth(depth))
}
fn fixed(kind: K, scope: &EventScope, body: E) -> E {
    E::FixedPoint {
        kind,
        scope: scope.clone(),
        body: Box::new(body),
    }
}
fn apply(op: BoolOp4, a: E, b: E) -> E {
    E::Apply {
        op,
        left: Box::new(a),
        right: Box::new(b),
    }
}
fn modal(op: ModalOp, r: &R, e: E) -> E {
    E::Modal {
        operation: op,
        relation: Box::new(r.clone()),
        input: Box::new(e),
    }
}
fn shape() -> (Space, FibreProduct, EventImport, EventImport) {
    let states = Space::new(SpaceId([111; 32]), 1, &()).unwrap();
    let env = Space::new(SpaceId([112; 32]), 0, &()).unwrap();
    let base = CoordinateMap::new(&states, &env, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = FibreProduct::new(SpaceId([113; 32]), &base, &base, &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([114; 32]), &pair, &pair, &pair, &()).unwrap();
    let faces = import(&AdmittedDescriptor::Fibre(pair.clone()));
    (
        states,
        pair,
        faces,
        import(&AdmittedDescriptor::Composition(plan)),
    )
}
fn query(finds: Vec<FindTerm>) -> Query {
    Query::single(Rule {
        finds,
        atoms: vec![
            Atom {
                source: AtomSource::Edb(Edge::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: AtomSource::Edb(Goal::RELATION),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(3))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}
fn mask(value: AnswerValue<'_>) -> u64 {
    let AnswerValue::Event(event) = value else {
        panic!("Event result")
    };
    (0..(1 << event.space().dimensions()))
        .filter(|&w| event.contains(w).unwrap())
        .fold(0, |m, w| m | (1 << w))
}
fn scalar(value: AnswerValue<'_>) -> u64 {
    let AnswerValue::U64(v) = value else {
        panic!("ordinary id")
    };
    v
}
fn modal_oracle(edges: u64, event: u64, all: bool, live: bool) -> u64 {
    (0..2)
        .filter(|&s| {
            let targets: Vec<_> = (0..2)
                .filter(|&t| edges & (1 << (s + 2 * t)) != 0)
                .collect();
            if all {
                (!live || !targets.is_empty()) && targets.iter().all(|t| event & (1 << t) != 0)
            } else {
                targets.iter().any(|t| event & (1 << t) != 0)
            }
        })
        .fold(0, |m, s| m | (1 << s))
}
fn iterate(top: bool, f: impl Fn(u64) -> u64) -> u64 {
    let mut current = if top { 3 } else { 0 };
    for _ in 0..3 {
        let next = f(current);
        if next == current {
            return next;
        }
        current = next;
    }
    panic!("finite monotone oracle did not stabilize")
}

fn game_expressions(
    states: &Space,
    pair_scope: &EventScope,
    faces: &EventImport,
    plan: &EventImport,
) -> [E; 5] {
    let r = R::Bind {
        faces: faces.clone(),
        region: Box::new(E::Var(VarId(1))),
    };
    let g = E::Var(VarId(3));
    let carrier = scope(states);
    let reach = fixed(
        K::Least,
        &carrier,
        apply(BoolOp4::OR, g.clone(), modal(ModalOp::May, &r, bound(0))),
    );
    let inevitable = fixed(
        K::Least,
        &carrier,
        apply(BoolOp4::OR, g.clone(), modal(ModalOp::Must, &r, bound(0))),
    );
    let safe = fixed(
        K::Greatest,
        &carrier,
        apply(BoolOp4::AND, g.clone(), modal(ModalOp::All, &r, bound(0))),
    );
    // Greatest X, least Y: (goal ∩ May(R,X)) ∪ May(R,Y).
    let nested = fixed(
        K::Greatest,
        &carrier,
        fixed(
            K::Least,
            &carrier,
            apply(
                BoolOp4::OR,
                apply(BoolOp4::AND, g.clone(), modal(ModalOp::May, &r, bound(1))),
                modal(ModalOp::May, &r, bound(0)),
            ),
        ),
    );
    let x = R::Bind {
        faces: faces.clone(),
        region: Box::new(bound(0)),
    };
    let composition = R::Product {
        operation: RelationProductOp::Compose,
        plan: plan.clone(),
        left: Box::new(r.clone()),
        right: Box::new(x),
    };
    let star = fixed(
        K::Least,
        pair_scope,
        E::Relation {
            operation: RelationViewOp::Region,
            relation: Box::new(R::Apply {
                op: BoolOp4::OR,
                left: Box::new(R::Identity {
                    faces: faces.clone(),
                }),
                right: Box::new(composition),
            }),
        },
    );
    [reach, inevitable, safe, nested, star]
}

#[test]
fn positive_nested_predicates_and_relation_binders_match_all_two_state_games() {
    let path = common::TempDir::new("event-query-binders");
    let db = Db::create(path.path(), BinderQueries, common::work())
        .unwrap()
        .unwrap();
    let (states, pair, faces, plan) = shape();
    db.write(common::work(), |tx| {
        for id in 0..16 {
            tx.insert([&Edge {
                id,
                when: table(pair.space(), id),
            }])?;
        }
        for id in 0..4 {
            tx.insert([&Goal {
                id,
                when: table(&states, id),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let expressions = game_expressions(&states, &scope(pair.space()), &faces, &plan);
    let mut finds = vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))];
    finds.extend(expressions.into_iter().map(FindTerm::Event));
    let query = query(finds);
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, query, pair, plan, states));
    for rows in retained {
        assert_eq!(rows.len(), 64);
        for row in 0..rows.len() {
            let r = scalar(rows.get(row, 0));
            let g = scalar(rows.get(row, 1));
            let reach = iterate(false, |x| g | modal_oracle(r, x, false, false));
            let inevitable = iterate(false, |x| g | modal_oracle(r, x, true, true));
            let safe = iterate(true, |x| g & modal_oracle(r, x, true, false));
            let nested = iterate(true, |x| {
                iterate(false, |y| {
                    (g & modal_oracle(r, x, false, false)) | modal_oracle(r, y, false, false)
                })
            });
            assert_eq!(
                [
                    mask(rows.get(row, 2)),
                    mask(rows.get(row, 3)),
                    mask(rows.get(row, 4)),
                    mask(rows.get(row, 5)),
                    mask(rows.get(row, 6))
                ],
                [reach, inevitable, safe, nested, r | 9]
            );
        }
    }
}

#[test]
fn illegal_binders_refuse_before_empty_relations_can_hide_them() {
    let path = common::TempDir::new("event-query-binder-admission");
    let db = Db::create(path.path(), BinderQueries, common::work())
        .unwrap()
        .unwrap();
    let (states, _, _, _) = shape();
    let carrier = scope(&states);
    for (expr, error) in [
        (
            bound(0),
            EventExprError::UnboundPredicate(PredicateDepth(0)),
        ),
        (
            fixed(K::Least, &carrier, bound(1)),
            EventExprError::UnboundPredicate(PredicateDepth(1)),
        ),
        (
            fixed(K::Least, &carrier, E::Not(Box::new(bound(0)))),
            EventExprError::NonMonotoneFixedPoint,
        ),
        (
            fixed(
                K::Least,
                &carrier,
                fixed(K::Greatest, &carrier, E::Not(Box::new(bound(1)))),
            ),
            EventExprError::NonMonotoneFixedPoint,
        ),
    ] {
        assert_eq!(expr.validate_shape(), Err(error));
        assert!(
            db.prepare(&query(vec![FindTerm::Event(expr)]), common::work())
                .is_err()
        );
    }
    // Lexical shadowing is distinct from a body-bound variable with ordinal 0.
    assert!(
        fixed(K::Least, &carrier, fixed(K::Greatest, &carrier, bound(0)))
            .validate_shape()
            .is_ok()
    );
    let other = Space::new(SpaceId([115; 32]), 1, &()).unwrap();
    assert_eq!(
        fixed(
            K::Least,
            &carrier,
            fixed(K::Greatest, &scope(&other), bound(0))
        )
        .validate_shape(),
        Err(EventExprError::IncompatibleContexts)
    );
    assert!(EventScope::from_event(&table(&states, 1), &()).is_err());
}

#[test]
fn constant_bodies_admit_every_external_occurrence_once_before_shortcuts() {
    let path = common::TempDir::new("event-query-binder-faults");
    let db = Db::create(path.path(), BinderQueries, common::work())
        .unwrap()
        .unwrap();
    let (states, pair, _, _) = shape();
    db.write(common::work(), |tx| {
        tx.insert([&Edge {
            id: 0,
            when: pair.space().empty(),
        }])?;
        tx.insert([&Goal {
            id: 0,
            when: states.full(),
        }])
    })
    .unwrap()
    .unwrap();
    // False discards every value mathematically. It must still diagnose both
    // written occurrences of the wrong-context edge, and no bound predicate.
    let expr = fixed(
        K::Least,
        &scope(&states),
        apply(
            BoolOp4::FALSE,
            apply(BoolOp4::OR, bound(0), E::Var(VarId(1))),
            apply(BoolOp4::OR, E::Var(VarId(3)), E::Var(VarId(1))),
        ),
    );
    assert_eq!(
        expr.variables().collect::<Vec<_>>(),
        vec![VarId(1), VarId(3), VarId(1)]
    );
    for cursor in [false, true] {
        let mut prepared = db
            .prepare(&query(vec![FindTerm::Event(expr.clone())]), common::work())
            .unwrap();
        prepared.force_cursor_fallback(cursor);
        let failure = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err();
        let bumbledb::Error::EventFaults(faults) = failure else {
            panic!("complete Event faults")
        };
        assert_eq!(
            faults
                .iter()
                .map(|f| (f.operand, f.source))
                .collect::<Vec<_>>(),
            vec![
                (0, bumbledb::EventOperandSource::Variable(VarId(1))),
                (2, bumbledb::EventOperandSource::Variable(VarId(1)))
            ]
        );
    }
}

#[test]
fn sealed_parameter_cells_and_zero_mass_worlds_survive_binders_and_probability() {
    let path = common::TempDir::new("event-query-binder-sources");
    let db = Db::create(path.path(), BinderQueries, common::work())
        .unwrap()
        .unwrap();
    let dummy = Space::new(SpaceId([119; 32]), 0, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Edge {
            id: 0,
            when: dummy.full(),
        }])
    })
    .unwrap()
    .unwrap();
    for fixture in [
        include_str!("fixtures/event-v2-source.hex"),
        include_str!("../../../ts/test/fixtures/event-v3-parameter-source.hex"),
    ] {
        let bytes: Vec<_> = fixture
            .trim()
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        let event = Event::from_bytes(&bytes, &()).unwrap();
        let scope = scope(&event.space());
        assert_eq!(
            scope.carrier().atoms(),
            event.space().full().atom_count(&()).unwrap()
        );
        let expr = fixed(K::Greatest, &scope, bound(0));
        let expected = event.space().full().to_bytes(&()).unwrap();
        // A captured scope suffices even with no row-bound Event operands.
        let mut q = query(vec![
            FindTerm::Event(expr.clone()),
            FindTerm::Probability {
                event: expr.clone(),
                given: expr,
            },
        ]);
        q.rules[0].atoms = vec![Atom {
            source: AtomSource::Edb(Edge::RELATION),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }];
        for cursor in [false, true] {
            let mut prepared = db.prepare(&q, common::work()).unwrap();
            prepared.force_cursor_fallback(cursor);
            let answer = db
                .read(common::work(), |snapshot| {
                    snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(answer.len(), 1);
            let AnswerValue::Event(result) = answer.get(0, 0) else {
                panic!("owned Event")
            };
            assert_eq!(result.to_bytes(&()).unwrap(), expected);
            assert!(matches!(answer.get(0, 1), AnswerValue::Probability(_)));
        }
    }
}

#[test]
fn variance_checks_cardinality_conditionals_maps_and_changing_relations() {
    use bumbledb::event::MapOp;
    let (states, pair, faces, plan) = shape();
    let carrier = scope(&states);
    let at = |minimum, maximum| E::Cardinality {
        minimum,
        maximum,
        events: vec![bound(0), E::Var(VarId(3))],
    };
    let identity_map = import(&AdmittedDescriptor::Map(
        CoordinateMap::coordinates(&states, &states, &[0], &()).unwrap(),
    ));
    for body in [
        E::Not(Box::new(E::Not(Box::new(bound(0))))),
        at(1, usize::MAX),
        at(3, 2),
        at(0, 2),
        E::Not(Box::new(at(0, 1))),
        E::Ite {
            condition: Box::new(E::Var(VarId(3))),
            high: Box::new(bound(0)),
            low: Box::new(E::Var(VarId(3))),
        },
        E::Map {
            operation: MapOp::Pullback,
            map: identity_map,
            input: Box::new(bound(0)),
        },
    ] {
        fixed(K::Least, &carrier, body).validate_shape().unwrap();
    }
    for body in [
        at(0, 1),
        at(1, 1),
        E::Ite {
            condition: Box::new(bound(0)),
            high: Box::new(E::Var(VarId(3))),
            low: Box::new(E::Var(VarId(3))),
        },
    ] {
        assert_eq!(
            fixed(K::Least, &carrier, body).validate_shape(),
            Err(EventExprError::NonMonotoneFixedPoint)
        );
    }
    // Domain(All(X,G)) is antitone in a relation X; Must also changes enabledness.
    let relation = R::Bind {
        faces: faces.clone(),
        region: Box::new(bound(0)),
    };
    for op in [ModalOp::All, ModalOp::Must] {
        let body = R::Test {
            faces: faces.clone(),
            predicate: Box::new(modal(op, &relation, E::Var(VarId(3)))),
        };
        let body = E::Relation {
            operation: RelationViewOp::Region,
            relation: Box::new(body),
        };
        assert_eq!(
            fixed(K::Least, &scope(pair.space()), body).validate_shape(),
            Err(EventExprError::NonMonotoneFixedPoint)
        );
    }
    for (op, varying_left, admitted) in [
        (RelationProductOp::Compose, true, true),
        (RelationProductOp::LeftResidual, true, false),
        (RelationProductOp::LeftResidual, false, true),
        (RelationProductOp::RightResidual, true, true),
        (RelationProductOp::RightResidual, false, false),
    ] {
        let constant = R::Identity {
            faces: faces.clone(),
        };
        let (left, right) = if varying_left {
            (relation.clone(), constant)
        } else {
            (constant, relation.clone())
        };
        let body = E::Relation {
            operation: RelationViewOp::Region,
            relation: Box::new(R::Product {
                operation: op,
                plan: plan.clone(),
                left: Box::new(left),
                right: Box::new(right),
            }),
        };
        assert_eq!(
            fixed(K::Least, &scope(pair.space()), body)
                .validate_shape()
                .is_ok(),
            admitted
        );
    }
}
