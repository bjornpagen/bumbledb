use bumbledb::{
    AnswerValue, BindValue, Db,
    event::{
        ArithmeticLimits, DensityPiece, ExactArithmetic, ExactRational, FiniteFunction,
        FunctionLimits, FunctionPatch, FunctionPiece, LawLimits, Space, SpaceId,
    },
};
mod common;

bumbledb::schema! {
    pub CoveredPayoffs;
    relation Evidence { group: u64, given: event }
    relation Patch { group: u64, expression: u64, when: event }
    Evidence(group) -> Evidence;
    Evidence(group, given) -> Evidence;
    Patch(group, expression) -> Patch;
    Patch(group, when) <= Evidence(group, given);
}
fn work() -> ExactArithmetic<'static> {
    ExactArithmetic::new(ArithmeticLimits::default(), &())
}
fn ratio(n: i64, d: u64) -> ExactRational {
    ExactRational::fraction(&n.to_string(), &d.to_string(), &mut work()).unwrap()
}
fn source() -> Space {
    let raw = Space::new(SpaceId([225; 32]), 2, &()).unwrap();
    raw.with_density(
        &[DensityPiece {
            region: raw.full(),
            density: ratio(1, 4),
        }],
        LawLimits::default(),
        &mut work(),
    )
    .unwrap()
}
fn function(source: &Space, values: [(i64, u64); 4]) -> FiniteFunction {
    FiniteFunction::new(
        source,
        &values
            .into_iter()
            .enumerate()
            .map(|(i, (n, d))| FunctionPiece {
                region: source.table(3, &[1 << i], &()).unwrap(),
                value: ratio(n, d),
            })
            .collect::<Vec<_>>(),
        FunctionLimits::default(),
        &mut work(),
    )
    .unwrap()
}

#[test]
fn free_join_supplies_owned_function_patches_with_exact_local_agreement() {
    let path = common::TempDir::new("function-cover-queries");
    let db = Db::create(path.path(), CoveredPayoffs, common::work())
        .unwrap()
        .unwrap();
    let original = source();
    db.write(common::work(), |tx| {
        tx.insert([&Evidence {
            group: 1,
            given: original.full(),
        }])?;
        for (expression, region) in [(1, 0b0011), (2, 0b0101), (3, 0b1000)] {
            tx.insert([&Patch {
                group: 1,
                expression,
                when: original.table(3, &[region], &())?,
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop((db, original));
    let db = Db::open(path.path(), CoveredPayoffs, common::work()).unwrap();
    let query = bumbledb::query!(CoveredPayoffs {
        (expression, when: Event(region & given), given) |
            Patch(group, expression, when: region), Evidence(group: group, given);
    });
    let mut retained = Vec::new();
    for fallback in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(fallback);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, query));
    // These independently reconstructed functions agree locally, not globally.
    let source = source();
    let functions = [
        function(&source, [(1, 3), (2, 3), (8, 1), (9, 1)]),
        function(&source, [(1, 3), (7, 1), (-2, 1), (8, 1)]),
        function(&source, [(0, 1); 4]),
    ];
    assert!(!functions[0].equivalent(&functions[1], &()).unwrap());
    for rows in retained {
        assert_eq!(rows.len(), 3);
        let mut patches = Vec::new();
        let mut parent = None;
        for row in 0..rows.len() {
            let AnswerValue::U64(expression) = rows.get(row, 0) else {
                panic!("expression")
            };
            let AnswerValue::Event(region) = rows.get(row, 1) else {
                panic!("region")
            };
            let AnswerValue::Event(given) = rows.get(row, 2) else {
                panic!("evidence")
            };
            patches.push(FunctionPatch {
                region: region.clone(),
                function: functions[usize::try_from(expression).unwrap() - 1].clone(),
            });
            parent = Some(given.clone());
        }
        let cover = FiniteFunction::glue(
            parent.as_ref().unwrap(),
            &patches,
            FunctionLimits::default(),
            &mut work(),
        )
        .unwrap();
        let result = cover
            .function()
            .expectation(cover.parent(), &mut work())
            .unwrap();
        assert_eq!(result.value(&mut work()).unwrap(), Some(ratio(-1, 4)));
        assert_eq!(cover.patches().len(), 3);
        for (world, expected) in [ratio(1, 3), ratio(2, 3), ratio(-2, 1), ratio(0, 1)]
            .iter()
            .enumerate()
        {
            assert_eq!(&cover.function().at(world as u64, &()).unwrap(), expected);
        }
    }
}
