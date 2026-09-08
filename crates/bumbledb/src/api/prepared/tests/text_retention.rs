use super::*;

fn fixture() -> Fix {
    Fix::heap(
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "Item".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "text".into(),
                        value_type: ValueType::String,
                    },
                ],
                extension: None,
            }],
            statements: vec![],
        },
        &[(
            RelationId(0),
            (0..256)
                .map(|id| {
                    vec![
                        Value::U64(id),
                        Value::String(format!("value-{id:04}").into()),
                    ]
                })
                .collect(),
        )],
    )
}

fn query(finds: Vec<FindTerm>) -> Query {
    Query::single(Rule {
        finds,
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn cursor_output_owns_text_beyond_row_reuse_and_release_preserves_answers() {
    let fix = fixture();
    let mut prepared = fix.prepare(&query(vec![FindTerm::Var(VarId(1))])).unwrap();
    prepared.force_cursor_fallback(true);
    let answers = fix.execute(&mut prepared, &[] as &[BindValue<'_>]).unwrap();
    assert_eq!(prepared.execution_texts.len(), 256);
    prepared.release_memory();
    prepared.cache.clear();
    let mut actual: Vec<_> = (0..answers.len())
        .map(|i| {
            let AnswerValue::String(text) = answers.get(i, 0) else {
                panic!("text answer")
            };
            text.to_owned()
        })
        .collect();
    actual.sort();
    assert_eq!(
        actual,
        (0..256)
            .map(|i| format!("value-{i:04}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(prepared.execution_texts.len(), 0);
    assert_eq!(
        fix.execute(&mut prepared, &[] as &[BindValue<'_>])
            .unwrap()
            .len(),
        256
    );
}

#[test]
fn cursor_numeric_projection_does_not_pin_scanned_text() {
    let fix = fixture();
    let mut prepared = fix.prepare(&query(vec![FindTerm::Var(VarId(0))])).unwrap();
    prepared.force_cursor_fallback(true);
    let answers = fix.execute(&mut prepared, &[] as &[BindValue<'_>]).unwrap();
    assert_eq!(answers.len(), 256);
    assert_eq!(prepared.execution_texts.len(), 0);
    assert!(prepared.cache.acquire().lock_resolver().retained_bytes() < 1024);
}

#[test]
fn cursor_aggregate_dedup_retains_non_output_text_across_repeated_arms() {
    use crate::api::prepared::fallback::{FallbackCtx, run_fallback};
    use crate::exec::sink::{AggSpec, AggregateSink, FindSpec};
    use crate::image::intern::InternerHandle;

    let fix = fixture();
    let mut prepared = fix.prepare(&query(vec![FindTerm::Count])).unwrap();
    let generation = prepared.cache.acquire();
    let work = crate::WorkContext::new();
    let source = super::super::source::QuerySource::heap(&fix.instance, 1, work.clone());
    let interner = InternerHandle::new(&generation, &work);
    let PreparedRule::FreeJoin(rule) = &mut prepared.pipeline.main_rules_mut()[0] else {
        panic!("count uses a join rule")
    };
    // Deliberately use exact full-binding dedup: the text is not an output
    // column, but reminting it between two arms would count each row twice.
    let mut sink = AggregateSink::new([FindSpec::Agg(AggSpec::Count)], rule.plan.slot_count());
    let mut bindings = Bindings::new(rule.plan.slot_count());
    let mut owners = crate::image::TextOwners::default();
    for turn in 0..2 {
        let mut ctx = FallbackCtx {
            source: &source,
            schema: &prepared.schema,
            interner: &interner,
            params: &[],
            missed: &[],
            retained_texts: &mut owners,
        };
        run_fallback(
            &mut rule.fallback,
            &mut ctx,
            &mut [],
            &mut bindings,
            &mut sink,
        )
        .unwrap();
        for i in 0..2048 {
            drop(interner.intern(&format!("churn-{turn}-{i}")).unwrap());
        }
    }
    assert_eq!(owners.len(), 256);
    assert_eq!(sink.into_answers().unwrap(), [vec![256]]);
    drop(owners);
    for i in 0..2048 {
        drop(interner.intern(&format!("released-{i}")).unwrap());
    }
    assert_eq!(generation.resolver().lookup("value-0000"), None);
}
