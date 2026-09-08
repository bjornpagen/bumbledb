use super::*;

#[test]
fn changing_text_parameters_do_not_retain_their_entire_history() {
    let fix = posting_store("text-parameter-reclamation", &[(1, 7, "live", 10)]);
    let mut prepared = fix.prepare(&by_memo_query()).unwrap();
    assert_eq!(
        amounts_of(&fix.execute(&mut prepared, &memo_param("live")).unwrap()),
        [10]
    );
    let generation = prepared.cache.cache_generation();
    let payload = "x".repeat(1024);
    let mut first = String::new();
    for turn in 0..128 {
        let text = format!("obsolete-{turn}-{payload}");
        if turn == 0 {
            first.clone_from(&text);
        }
        assert!(
            fix.execute(&mut prepared, &memo_param(&text))
                .unwrap()
                .is_empty()
        );
    }
    let owner = prepared.cache.acquire();
    let retained = owner.lock_resolver().retained_bytes();
    eprintln!(
        "text parameter churn: 128 distinct ~1KiB parameters, retained interner bytes={retained}"
    );
    assert_eq!(
        prepared.cache.cache_generation(),
        generation,
        "churn must not globally rotate unrelated caches"
    );
    assert_eq!(
        owner.resolver().lookup(&first),
        None,
        "obsolete parameters are reclaimed automatically"
    );
    assert!(
        retained < 16 * 1024,
        "retention must not grow with all historical parameters"
    );
    #[cfg(feature = "alloc-counter")]
    let live_after_short = crate::alloc_counter::snapshot().absolute.live_bytes;
    for turn in 128..4224 {
        let text = format!("obsolete-{turn}-{payload}");
        assert!(
            fix.execute(&mut prepared, &memo_param(&text))
                .unwrap()
                .is_empty()
        );
    }
    #[cfg(feature = "alloc-counter")]
    {
        let live_after_long = crate::alloc_counter::snapshot().absolute.live_bytes;
        eprintln!(
            "text churn live heap: after 128={live_after_short}, after 4224={live_after_long}"
        );
        assert!(
            live_after_long.saturating_sub(live_after_short) < 8192,
            "retained heap must plateau, not accumulate another 4 MiB of historical text"
        );
    }
    assert!(owner.lock_resolver().retained_bytes() < 16 * 1024);
    assert_eq!(
        amounts_of(&fix.execute(&mut prepared, &memo_param("live")).unwrap()),
        [10]
    );
}

#[test]
fn scalar_parameter_memo_shares_the_canonical_string_and_release_drops_its_owner() {
    let fix = posting_store("shared-parameter-text-owner", &[(1, 7, "live", 10)]);
    let mut prepared = fix.prepare(&by_memo_query()).unwrap();
    let payload = "absent".repeat(1024);
    assert!(
        fix.execute(&mut prepared, &memo_param(&payload))
            .unwrap()
            .is_empty()
    );
    let generation = prepared.cache.acquire();
    let Const::Text(parameter) = &prepared.resolved_params[0] else {
        panic!("owned text parameter")
    };
    let canonical = generation.resolver().owned_text(parameter.word).unwrap();
    assert!(std::sync::Arc::ptr_eq(&canonical, &parameter.text));
    assert!(std::sync::Arc::ptr_eq(
        &canonical,
        prepared.param_word_memo[0].text.as_ref().unwrap()
    ));
    let weak = std::sync::Arc::downgrade(&canonical);
    drop(canonical);
    generation.lock_resolver().reclaim_unowned();
    assert!(
        weak.upgrade().is_some(),
        "retained parameter owns its token"
    );
    assert!(
        fix.execute(&mut prepared, &memo_param(&payload))
            .unwrap()
            .is_empty()
    );
    prepared.release_memory();
    generation.lock_resolver().reclaim_unowned();
    assert!(
        weak.upgrade().is_none(),
        "release drops all query-owned copies of the text owner"
    );
    assert_eq!(generation.resolver().lookup(&payload), None);
    assert!(
        generation.resolver().lookup("live").is_some(),
        "shared cached image remains live"
    );
    assert_eq!(
        amounts_of(&fix.execute(&mut prepared, &memo_param("live")).unwrap()),
        [10]
    );
}

fn uuid_param_fixture() -> (StoreFix, Query, [crate::Uuid; 3]) {
    let ids = [
        crate::Uuid::from_bytes([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        crate::Uuid::from_bytes([15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]),
        crate::Uuid::from_bytes([0x55; 16]),
    ];
    let relation = RelationId(0);
    let descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Record".into(),
            fields: [
                ("id", ValueType::Uuid),
                ("value", ValueType::U64),
                ("owner", ValueType::Uuid),
            ]
            .into_iter()
            .map(|(name, value_type)| FieldDescriptor {
                name: name.into(),
                value_type,
            })
            .collect(),
            extension: None,
        }],
        statements: vec![crate::schema::StatementDescriptor::Functionality {
            relation,
            projection: Box::new([FieldId(0)]),
        }],
    };
    let fix = StoreFix::store("uuid-param-slot-reuse", descriptor);
    fix.insert_dyn(
        relation,
        &[
            vec![Value::Uuid(ids[0]), Value::U64(11), Value::Uuid(ids[0])],
            vec![Value::Uuid(ids[1]), Value::U64(22), Value::Uuid(ids[0])],
        ],
    );
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(relation),
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Literal(Value::Uuid(ids[0]))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    (fix, query, ids)
}

#[test]
fn uuid_param_slot_reuses_words_across_hits_misses_and_type_errors() {
    let (fix, query, ids) = uuid_param_fixture();
    let mut prepared = fix.prepare(&query).unwrap();
    let PreparedPipeline::PointProbe { rule, .. } = &prepared.pipeline else {
        panic!("UUID key should select the point pipeline");
    };
    let template = rule.plan.clone();
    let mut out = Answers::new();
    fix.execute_into(&mut prepared, &[BindValue::Uuid(ids[0])], &mut out)
        .unwrap();
    assert_eq!(out.get(0, 0), AnswerValue::U64(11));
    let Const::Words(initial) = &prepared.resolved_params[0] else {
        panic!("UUID bind uses two words");
    };
    assert_eq!(
        initial.as_ref(),
        [0x0001_0203_0405_0607, 0x0809_0a0b_0c0d_0e0f]
    );
    let address = initial.as_ptr();

    for (id, expected) in [(ids[1], Some(22)), (ids[2], None), (ids[0], Some(11))] {
        fix.execute_into(&mut prepared, &[BindValue::Uuid(id)], &mut out)
            .unwrap();
        match expected {
            Some(value) => {
                assert_eq!(out.len(), 1);
                assert_eq!(out.get(0, 0), AnswerValue::U64(value));
            }
            None => assert!(out.is_empty()),
        }
        let Const::Words(words) = &prepared.resolved_params[0] else {
            panic!("UUID slot retains its word representation");
        };
        assert_eq!(
            words.as_ptr(),
            address,
            "rebinding must reuse the live allocation"
        );
        assert_eq!(words.len(), 2);
        assert_eq!(
            [words[0].to_be_bytes(), words[1].to_be_bytes()].concat(),
            id.into_bytes()
        );
    }
    let err = fix
        .execute_into(&mut prepared, &[BindValue::FixedBytes(&[0; 16])], &mut out)
        .unwrap_err();
    assert!(
        matches!(err, Error::ParamTypeMismatch { param, expected: ValueType::Uuid } if param.0 == 0)
    );
    assert!(out.is_empty(), "a refused bind clears the preceding answer");
    let Const::Words(words) = &prepared.resolved_params[0] else {
        panic!("refusal cannot replace the retained slot");
    };
    assert_eq!(words.as_ptr(), address);
    fix.execute_into(&mut prepared, &[BindValue::Uuid(ids[1])], &mut out)
        .unwrap();
    assert_eq!(out.get(0, 0), AnswerValue::U64(22));
    let no_params: &[BindValue<'_>] = &[];
    let err = fix
        .execute_into(&mut prepared, no_params, &mut out)
        .unwrap_err();
    assert!(matches!(err, Error::ParamCountMismatch { .. }));
    assert!(out.is_empty());
    let PreparedPipeline::PointProbe { rule, .. } = &prepared.pipeline else {
        unreachable!("same point pipeline");
    };
    assert_eq!(
        rule.plan, template,
        "parameter storage never mutates the UUID literal template"
    );
}

#[test]
fn prepare_once_execute_many_with_varying_params() {
    let fix = postings(&[
        (1, 7, "rent", -1200),
        (2, 7, "salary", 5000),
        (3, 8, "coffee", -4),
    ]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();

    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(7), BindValue::I64(0)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(answers_of(&out), vec![("salary".to_owned(), 5000)]);

    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(7), BindValue::I64(i64::MIN)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(
        answers_of(&out),
        vec![("rent".to_owned(), -1200), ("salary".to_owned(), 5000)]
    );

    fix.execute_into(
        &mut prepared,
        &[BindValue::U64(8), BindValue::I64(i64::MIN)],
        &mut out,
    )
    .expect("execute");
    assert_eq!(answers_of(&out), vec![("coffee".to_owned(), -4)]);
}

#[test]
fn bind_time_checks_reject_bad_params() {
    let fix = postings(&[]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let mut out = Answers::new();

    let err = fix
        .execute_into(&mut prepared, &[BindValue::U64(7)], &mut out)
        .unwrap_err();
    assert!(
        matches!(
            err,
            Error::ParamCountMismatch {
                mismatch: crate::error::Mismatch {
                    witnessed: 1,
                    required: 2,
                },
            }
        ),
        "{err:?}"
    );

    let err = fix
        .execute_into(
            &mut prepared,
            &[BindValue::I64(7), BindValue::I64(0)],
            &mut out,
        )
        .unwrap_err();
    assert!(
        matches!(err, Error::ParamTypeMismatch { param, .. } if param.0 == 0),
        "{err:?}"
    );
}

#[test]
fn string_params_resolve_per_execution() {
    let fix = posting_store("prepared-string-params", &[(1, 7, "rent", -1200)]);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let mut out = Answers::new();

    fix.execute_into(&mut prepared, &[BindValue::Str("groceries")], &mut out)
        .expect("execute");
    assert!(out.is_empty());

    fix.insert_dyn(POSTING, &posting_rows(&[(2, 9, "groceries", -55)]));
    fix.execute_into(&mut prepared, &[BindValue::Str("groceries")], &mut out)
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(-55));
}

#[test]
fn resident_string_param_rebinds_after_shared_cache_generation_rotates() {
    let fix = posting_store(
        "prepared-param-generation",
        &[(1, 7, "alpha", 10), (2, 7, "beta", 20)],
    );
    let mut alpha = fix.prepare(&by_memo_query()).expect("prepare alpha");
    let mut beta = fix.prepare(&by_memo_query()).expect("prepare beta");
    assert_eq!(
        amounts_of(
            &fix.execute(&mut alpha, &memo_param("alpha"))
                .expect("alpha")
        ),
        vec![10],
    );

    // Trim drops alpha's views and retires its resolver. A sibling query
    // then deliberately reuses token zero for beta in the shared cache.
    alpha.release_memory();
    fix.db.clear_cache();
    assert_eq!(
        amounts_of(&fix.execute(&mut beta, &memo_param("beta")).expect("beta")),
        vec![20],
    );
    assert_eq!(
        amounts_of(
            &fix.execute(&mut alpha, &memo_param("alpha"))
                .expect("alpha rebound")
        ),
        vec![10],
        "a memoized token cannot change meaning after the shared cache rotates",
    );
}

/// Force the schedule where a sibling trim happens after parameters
/// bind but before image binding. The executing query must retain the
/// resolver that minted its parameters through finalization.
struct RotateAfterBind<P>(P);

impl<'a, P: BindArgs<'a>> BindArgs<'a> for RotateAfterBind<P> {
    fn bind<S>(
        self,
        prepared: &mut PreparedQuery<S>,
        work: &crate::work::WorkContext,
    ) -> crate::error::Result<()> {
        self.0.bind(prepared, work)?;
        prepared.cache.clear();
        let generation = prepared.cache.acquire();
        let interner = crate::image::intern::InternerHandle::new(&generation, work);
        assert_eq!(interner.intern("beta")?.word, 0);
        Ok(())
    }
}

#[test]
fn scalar_set_and_point_params_keep_one_owner_across_bind_time_rotation() {
    let fix = posting_store(
        "prepared-bind-generation",
        &[(1, 7, "alpha", 10), (2, 7, "beta", 20)],
    );
    for (point, set) in [(false, false), (false, true), (true, false)] {
        let mut bindings = vec![
            (
                FieldId(2),
                if set {
                    Term::ParamSet(crate::ir::ParamId(0))
                } else {
                    Term::Param(crate::ir::ParamId(0))
                },
            ),
            (FieldId(3), Term::Var(VarId(0))),
        ];
        if point {
            bindings.push((FieldId(0), Term::Literal(Value::U64(1))));
        }
        let query = Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![Atom {
                source: AtomSource::Edb(POSTING),
                bindings,
            }],
            negated: vec![],
            conditions: vec![],
        });
        let mut prepared = fix.prepare(&query).expect("prepare");
        let elements = [Value::String("alpha".into())];
        let args = [if set {
            ParamArg::Set(&elements)
        } else {
            ParamArg::Scalar(BindValue::Str("alpha"))
        }];
        let out = fix
            .execute(&mut prepared, RotateAfterBind(&args))
            .expect("execute across rotation");
        assert_eq!(amounts_of(&out), vec![10], "point={point}, set={set}");
        let out = fix.execute(&mut prepared, &args).expect("new owner");
        assert_eq!(amounts_of(&out), vec![10], "subsequent owner rebind");
    }
}

#[test]
fn param_word_memo_preserves_tokens_across_rebinds_and_new_rows() {
    let fix = posting_store(
        "prepared-param-word-memo",
        &[(1, 7, "alpha", 10), (2, 7, "beta", 20)],
    );
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    let run = |prepared: &mut PreparedQuery<T>, text: &str| {
        let out = fix.execute(prepared, &memo_param(text)).expect("execute");
        let memo = &prepared.param_word_memo[0];
        assert_eq!(memo.text.as_deref(), Some(text));
        (amounts_of(&out), memo.word.expect("bound token"))
    };
    let alpha = run(&mut prepared, "alpha");
    assert_eq!(alpha.0, vec![10]);
    assert_eq!(run(&mut prepared, "alpha"), alpha);
    let beta = run(&mut prepared, "beta");
    assert_eq!(beta.0, vec![20]);
    assert_ne!(alpha.1, beta.1);
    assert_eq!(run(&mut prepared, "beta"), beta);
    let gamma = run(&mut prepared, "gamma");
    assert!(gamma.0.is_empty());
    assert_eq!(run(&mut prepared, "gamma"), gamma);
    fix.insert_dyn(POSTING, &posting_rows(&[(3, 7, "gamma", 30)]));
    assert_eq!(run(&mut prepared, "gamma"), (vec![30], gamma.1));
}
