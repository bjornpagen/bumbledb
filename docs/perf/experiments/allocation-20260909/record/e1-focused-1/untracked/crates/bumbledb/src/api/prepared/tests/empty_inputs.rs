use super::*;
use crate::ir::{FoldOp, ParamId};

const ROWS: RelationId = RelationId(0);
const GATE: RelationId = RelationId(1);

fn schema() -> SchemaDescriptor {
    let relation = |name: &str, fields: &[&str]| RelationDescriptor {
        extension: None,
        name: name.into(),
        fields: fields
            .iter()
            .map(|name| FieldDescriptor {
                name: (*name).into(),
                value_type: ValueType::U64,
            })
            .collect(),
    };
    SchemaDescriptor {
        relations: vec![
            relation("Rows", &["id", "entry", "account", "instrument"]),
            relation("Gate", &["account"]),
        ],
        statements: vec![crate::schema::StatementDescriptor::Functionality {
            relation: ROWS,
            projection: Box::new([FieldId(0)]),
        }],
    }
}

fn store(name: &'static str) -> StoreFix {
    let fix = StoreFix::store(name, schema());
    let rows: Vec<_> = (0..512)
        .map(|id| {
            [id, id / 2, id % 8, (id / 3) % 128]
                .map(Value::U64)
                .to_vec()
        })
        .collect();
    fix.insert_dyn(ROWS, &rows);
    fix
}

fn triangle() -> Rule {
    let v = |id| Term::Var(VarId(id));
    Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(ROWS),
                bindings: vec![(FieldId(2), v(0)), (FieldId(3), v(1))],
            },
            Atom {
                source: AtomSource::Edb(ROWS),
                bindings: vec![(FieldId(1), v(2)), (FieldId(3), v(1))],
            },
            Atom {
                source: AtomSource::Edb(ROWS),
                bindings: vec![(FieldId(1), v(2)), (FieldId(2), v(0))],
            },
        ],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: v(0),
                rhs: Term::Param(ParamId(0)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: v(0),
                rhs: Term::Param(ParamId(1)),
            }),
        ],
    }
}

fn gate(keyed: bool) -> Atom {
    Atom {
        source: AtomSource::Edb(GATE),
        bindings: if keyed {
            vec![(FieldId(0), Term::Var(VarId(0)))]
        } else {
            vec![]
        },
    }
}

fn accounts(out: &Answers) -> Vec<u64> {
    let mut rows: Vec<_> = (0..out.len())
        .map(|row| {
            let AnswerValue::U64(account) = out.get(row, 0) else {
                panic!("u64 account")
            };
            account
        })
        .collect();
    rows.sort_unstable();
    rows
}

fn free_join(prepared: &PreparedQuery<T>) -> &FreeJoinRule {
    let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
        panic!("triangle fixture uses one Free Join rule")
    };
    rule
}

#[test]
fn empty_positive_inputs_leave_all_join_maps_unforced() {
    use crate::image::{SourceImages, view::View};
    use crate::ir::normalize::{OccId, normalize_rules};
    use crate::plan::fj::{FjPlan, Node, Subatom};

    let fix = store("empty-positive-late-plan");
    let mut observed = Vec::new();
    for missing_relation in [false, true] {
        let mut rule = triangle();
        rule.conditions.clear();
        if missing_relation {
            rule.atoms.push(gate(true));
        } else {
            rule.conditions.push(ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(0)),
                rhs: Term::Literal(Value::U64(20)),
            }));
        }
        let query = Query::single(rule);
        fix.db
            .read(crate::api::db::test_operation(), |instance| {
                let schema = instance.schema();
                let witness = crate::ir::validate::validate(schema, &query)?;
                let normalized = normalize_rules(schema, &[], witness.rules());
                let node = |pairs: &[(u16, u16)]| Node {
                    subatoms: pairs
                        .iter()
                        .map(|&(occ, var)| Subatom {
                            occ: OccId(occ),
                            vars: vec![VarId(var)],
                        })
                        .collect(),
                    estimate: 512,
                };
                // An entry-first triangle is a legal physical plan. Its first
                // node builds entry tries before reaching the empty account view.
                // Seal this order explicitly: cost-estimator changes must not
                // silently turn the regression into an empty-first scan.
                let mut nodes = vec![
                    node(&[(1, 2), (2, 2)]),
                    node(&[(1, 1), (0, 1)]),
                    node(&[(0, 0), (2, 0)]),
                ];
                if missing_relation {
                    nodes[2].subatoms.push(Subatom {
                        occ: OccId(3),
                        vars: vec![VarId(0)],
                    });
                }
                let plan = crate::plan::fj::validate(
                    &FjPlan { nodes },
                    &normalized[0],
                    schema,
                    &witness.rule(0).sink_vars(),
                )
                .expect("valid late-empty plan");
                let mut memo = ViewMemo::new();
                for occurrence in plan.occurrences() {
                    assert!(occurrence.selections.is_empty());
                    let columns = occurrence
                        .trie_schema
                        .iter()
                        .map(|level| {
                            level
                                .iter()
                                .map(|var| {
                                    let (field, _) =
                                        occurrence.vars.iter().find(|(_, v)| v == var).unwrap();
                                    let span = occurrence.spans[usize::from(field.0)];
                                    assert_eq!(span.width.column_count(), 1);
                                    usize::from(span.first_column)
                                })
                                .collect()
                        })
                        .collect();
                    memo.push(Colt::new(View::Unbound, &[], columns), Binding::Unbound);
                }
                let filters: Vec<_> = plan
                    .occurrences()
                    .iter()
                    .map(|occ| occ.filters.clone())
                    .collect();
                let selections = vec![Vec::new(); plan.occurrences().len()];
                let source =
                    super::super::source::QuerySource::store(instance.snapshot(), instance.work());
                let images = SourceImages::bind(&source, instance.cache());
                let mut executor = Executor::new(&plan);
                let mut bindings = Bindings::new(plan.slot_count());
                let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(0))]);
                super::super::run_join::run_join(
                    &plan,
                    schema,
                    &images,
                    instance.work(),
                    &mut executor,
                    &mut bindings,
                    &filters,
                    &selections,
                    &mut memo,
                    &super::super::reach::OccImages::default(),
                    &mut Vec::new(),
                    &mut sink,
                    &mut crate::exec::run::NoopCounters,
                )?;
                assert_eq!(sink.len(), 0);
                observed.push((
                    missing_relation,
                    memo.colts
                        .iter()
                        .map(|colt| colt.forced_capacity(Colt::root()))
                        .collect::<Vec<_>>(),
                ));
                Ok(())
            })
            .unwrap();
    }
    assert!(
        observed
            .iter()
            .all(|(_, capacities)| capacities.iter().all(Option::is_none)),
        "known-empty positive input forced join maps: {observed:?}"
    );
}

#[test]
fn empty_first_plans_leave_all_join_maps_unforced() {
    let fix = store("empty-positive-no-force");
    for missing_relation in [false, true] {
        let mut rule = triangle();
        if missing_relation {
            rule.atoms.push(gate(true));
        }
        let mut prepared = fix.prepare(&Query::single(rule)).unwrap();
        let (lo, hi) = if missing_relation { (0, 8) } else { (20, 21) };
        let out = fix
            .execute(&mut prepared, &[BindValue::U64(lo), BindValue::U64(hi)])
            .unwrap();
        assert!(accounts(&out).is_empty());
        let capacities: Vec<_> = free_join(&prepared)
            .memo
            .colts
            .iter()
            .map(|colt| colt.forced_capacity(Colt::root()))
            .collect();
        assert!(
            capacities.iter().all(Option::is_none),
            "known-empty positive input forced join maps: {capacities:?}, missing_relation={missing_relation}"
        );
    }
}

#[test]
fn empty_bindings_survive_rotation_new_epochs_and_memory_release() {
    let fix = store("empty-positive-rotation");
    let mut prepared = fix.prepare(&Query::single(triangle())).unwrap();
    let mut out = Answers::new();
    for _ in 0..2 {
        for (lo, hi) in [(20, 21), (0, 1), (4, 5), (30, 30), (1, 3), (20, 21)] {
            fix.execute_into(
                &mut prepared,
                &[BindValue::U64(lo), BindValue::U64(hi)],
                &mut out,
            )
            .unwrap();
            assert_eq!(
                accounts(&out),
                (0..8)
                    .filter(|id| lo <= *id && *id < hi)
                    .collect::<Vec<_>>()
            );
        }
    }
    fix.insert_dyn(ROWS, &[[1000, 1000, 20, 0].map(Value::U64).to_vec()]);
    for _ in 0..2 {
        for (lo, hi, expected) in [(20, 21, vec![20]), (30, 31, vec![]), (0, 1, vec![0])] {
            fix.execute_into(
                &mut prepared,
                &[BindValue::U64(lo), BindValue::U64(hi)],
                &mut out,
            )
            .unwrap();
            assert_eq!(accounts(&out), expected);
        }
        prepared.release_memory();
    }
}

#[test]
fn empty_negated_inputs_and_zero_arity_gates_keep_their_meanings() {
    let fix = store("empty-input-polarity");
    for keyed in [false, true] {
        for negated in [false, true] {
            let mut rule = triangle();
            if negated {
                rule.negated.push(gate(keyed));
            } else {
                rule.atoms.push(gate(keyed));
            }
            let mut prepared = fix.prepare(&Query::single(rule)).unwrap();
            let params = [BindValue::U64(0), BindValue::U64(8)];
            let before = fix.execute(&mut prepared, &params).unwrap();
            let expected: Vec<_> = if negated { (0..8).collect() } else { vec![] };
            assert_eq!(
                accounts(&before),
                expected,
                "keyed={keyed}, negated={negated}"
            );
        }
    }
    fix.insert_dyn(GATE, &[vec![Value::U64(3)]]);
    for keyed in [false, true] {
        for negated in [false, true] {
            let mut rule = triangle();
            if negated {
                rule.negated.push(gate(keyed));
            } else {
                rule.atoms.push(gate(keyed));
            }
            let mut prepared = fix.prepare(&Query::single(rule)).unwrap();
            let out = fix
                .execute(&mut prepared, &[BindValue::U64(0), BindValue::U64(8)])
                .unwrap();
            let expected: Vec<_> = (0..8)
                .filter(|id| (!keyed || *id == 3) != negated)
                .collect();
            assert_eq!(accounts(&out), expected, "keyed={keyed}, negated={negated}");
        }
    }
}

#[test]
fn an_empty_rule_does_not_erase_its_live_alternative() {
    let fix = store("empty-input-alternative");
    let mut live = triangle();
    live.conditions.clear();
    let query = Query {
        interiors: vec![],
        head: vec![HeadTerm::Var],
        rules: vec![triangle(), live],
        rec: None,
    };
    let mut prepared = fix.prepare(&query).unwrap();
    assert_eq!(prepared.pipeline.main_rules().len(), 2);
    for (lo, hi) in [(20, 21), (0, 1), (30, 31)] {
        let out = fix
            .execute(&mut prepared, &[BindValue::U64(lo), BindValue::U64(hi)])
            .unwrap();
        assert_eq!(accounts(&out), (0..8).collect::<Vec<_>>());
    }
}

#[test]
fn empty_fold_inputs_still_finalize_and_reuse_their_sinks() {
    let fix = store("empty-input-folds");
    for grouped in [false, true] {
        let mut rule = triangle();
        rule.finds = vec![
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(0),
            },
            FindTerm::Count,
        ];
        if grouped {
            rule.finds.insert(0, FindTerm::Var(VarId(1)));
        }
        let mut prepared = fix.prepare(&Query::single(rule)).unwrap();
        let mut out = Answers::new();
        for (lo, hi) in [(20, 21), (0, 8), (20, 21)] {
            fix.execute_into(
                &mut prepared,
                &[BindValue::U64(lo), BindValue::U64(hi)],
                &mut out,
            )
            .unwrap();
            if lo == 20 {
                assert_eq!(out.len(), usize::from(!grouped));
                if !grouped {
                    assert_eq!(out.get(0, 0), AnswerValue::U64(0));
                    assert_eq!(out.get(0, 1), AnswerValue::U64(0));
                }
            } else {
                assert!(!out.is_empty());
            }
        }
    }
}
