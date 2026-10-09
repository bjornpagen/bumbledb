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
fn answers_own_their_text_beyond_release_and_cache_clear() {
    let fix = fixture();
    let mut prepared = fix.prepare(&query(vec![FindTerm::Var(VarId(1))])).unwrap();
    let answers = fix.execute(&mut prepared, &[] as &[BindValue<'_>]).unwrap();
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
    assert_eq!(
        fix.execute(&mut prepared, &[] as &[BindValue<'_>])
            .unwrap()
            .len(),
        256
    );
}
