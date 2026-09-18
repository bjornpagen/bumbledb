//! Query relation programs compared with explicit Boolean matrices.
use bumbledb::{
    AnswerValue, BindValue, Db, Event, EventExprError, EventImport, FindTerm,
    event::{
        AdmittedDescriptor, CoordinateMap, DescriptorLimits, FibreProduct, RelationalProduct,
        Space, SpaceId,
    },
    query,
};
mod common;

bumbledb::schema! {
    pub RelationQueries;
    relation Left { id: u64, region: event }
    relation Right { id: u64, region: event }
    relation Bound { id: u64, region: event }
    relation Goal { id: u64, region: event }
}

fn import(value: AdmittedDescriptor) -> EventImport {
    let original = EventImport::capture(&value, DescriptorLimits::default(), &()).unwrap();
    drop(value);
    // Execute with independently reconstructed owners, not the stored rows' arenas.
    EventImport::from_bytes(original.bytes(), DescriptorLimits::default(), &()).unwrap()
}

fn pair(id: u8, a: &Space, b: &Space, environment: &Space) -> FibreProduct {
    let map = |s| {
        CoordinateMap::coordinates(s, environment, &[], &())
            .unwrap()
            .certify_surjective(&())
            .unwrap()
    };
    FibreProduct::new(SpaceId([id; 32]), &map(a), &map(b), &()).unwrap()
}

fn region(space: &Space, bits: u64) -> Event {
    space
        .table((1 << space.dimensions()) - 1, &[bits], &())
        .unwrap()
}

fn mask(value: AnswerValue<'_>) -> u64 {
    let AnswerValue::Event(value) = value else {
        panic!("Event output")
    };
    (0..(1 << value.space().dimensions()))
        .filter(|&w| match value.contains(w) {
            Ok(member) => member,
            Err(bumbledb::event::Error::IllegalWorld(_)) => false,
            Err(error) => panic!("unexpected membership failure: {error}"),
        })
        .fold(0, |bits, w| bits | (1 << w))
}

fn scalar(value: AnswerValue<'_>) -> u64 {
    let AnswerValue::U64(value) = value else {
        panic!("matrix id")
    };
    value
}

fn cell(bits: u64, s: u64, t: u64) -> bool {
    bits & (1 << (s + 2 * t)) != 0
}
fn matrix(predicate: impl Fn(u64, u64) -> bool) -> u64 {
    (0..2)
        .flat_map(|s| (0..2).map(move |t| (s, t)))
        .filter(|&(s, t)| predicate(s, t))
        .fold(0, |bits, (s, t)| bits | (1 << (s + 2 * t)))
}

#[test]
fn authored_products_preserve_one_middle_witness_for_composition_and_both_residuals() {
    let directory = common::TempDir::new("event-query-products");
    let db = Db::create(directory.path(), RelationQueries, common::work())
        .unwrap()
        .unwrap();
    let env = Space::new(SpaceId([20; 32]), 0, &()).unwrap();
    let source = Space::new(SpaceId([21; 32]), 1, &()).unwrap();
    let middle = Space::new(SpaceId([22; 32]), 1, &()).unwrap();
    let target = Space::new(SpaceId([23; 32]), 1, &()).unwrap();
    let st = pair(24, &source, &middle, &env);
    let tu = pair(25, &middle, &target, &env);
    let su = pair(26, &source, &target, &env);
    let plan = RelationalProduct::new(SpaceId([27; 32]), &st, &tu, &su, &()).unwrap();
    db.write(common::work(), |tx| {
        for bits in 0..16 {
            tx.insert([&Left {
                id: bits,
                region: region(st.space(), bits),
            }])?;
            tx.insert([&Right {
                id: bits,
                region: region(tu.space(), bits),
            }])?;
            tx.insert([&Bound {
                id: bits,
                region: region(su.space(), bits),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let first = import(AdmittedDescriptor::Fibre(st));
    let second = import(AdmittedDescriptor::Fibre(tu));
    let outer = import(AdmittedDescriptor::Fibre(su));
    let path = import(AdmittedDescriptor::Composition(plan));
    let template = query!(RelationQueries {
        use faces first = &first;
        use faces second = &second;
        use faces outer = &outer;
        use product path = &path;
        (i, j, k,
         composed: Event(Region(Compose(Relation(a, first), Relation(b, second), path))),
         left: Event(Region(LeftResidual(Relation(a, first), Relation(c, outer), path))),
         right: Event(Region(RightResidual(Relation(c, outer), Relation(b, second), path)))) |
            Left(id: i, region: a), Right(id: j, region: b), Bound(id: k, region: c);
    });
    drop((first, second, outer, path, source, middle, target, env));
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        retained.push(
            db.read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap(),
        );
    }
    drop((db, template));
    for rows in retained {
        assert_eq!(rows.len(), 4096);
        for n in 0..rows.len() {
            let a = scalar(rows.get(n, 0));
            let b = scalar(rows.get(n, 1));
            let c = scalar(rows.get(n, 2));
            let composed = matrix(|s, u| (0..2).any(|t| cell(a, s, t) && cell(b, t, u)));
            let left = matrix(|t, u| (0..2).all(|s| !cell(a, s, t) || cell(c, s, u)));
            let right = matrix(|s, t| (0..2).all(|u| !cell(b, t, u) || cell(c, s, u)));
            assert_eq!(mask(rows.get(n, 3)), composed);
            assert_eq!(mask(rows.get(n, 4)), left);
            assert_eq!(mask(rows.get(n, 5)), right);
        }
    }
}

#[test]
fn modal_queries_closure_and_constant_identity_match_finite_paths() {
    let directory = common::TempDir::new("event-query-modal");
    let db = Db::create(directory.path(), RelationQueries, common::work())
        .unwrap()
        .unwrap();
    let env = Space::new(SpaceId([30; 32]), 0, &()).unwrap();
    let states = Space::new(SpaceId([31; 32]), 1, &()).unwrap();
    let pair = pair(32, &states, &states, &env);
    let plan = RelationalProduct::new(SpaceId([33; 32]), &pair, &pair, &pair, &()).unwrap();
    db.write(common::work(), |tx| {
        for bits in 0..16 {
            tx.insert([&Left {
                id: bits,
                region: region(pair.space(), bits),
            }])?;
        }
        for bits in 0..4 {
            tx.insert([&Goal {
                id: bits,
                region: region(&states, bits),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let faces = import(AdmittedDescriptor::Fibre(pair));
    let plan = import(AdmittedDescriptor::Composition(plan));
    let template = query!(RelationQueries {
        use faces transition = &faces;
        use product paths = &plan;
        (i, j,
         domain: Event(Domain(Relation(r, transition))),
         range: Event(Range(Relation(r, transition))),
         may: Event(May(Relation(r, transition), g)),
         all: Event(All(Relation(r, transition), g)),
         must: Event(Must(Relation(r, transition), g)),
         post: Event(Post(Relation(r, transition), g)),
         identity: Event(Region(Id(transition))),
         test: Event(Region(TestRelation(g, transition))),
         star: Event(Region(Star(Relation(r, transition), paths))),
         reachable: Event(May(Star(Relation(r, transition), paths), g)),
         converse_domain: Event(Domain(Converse(Relation(r, transition)))),
         empty: Event(Region(Relation(r, transition) & !Relation(r, transition))),
         law: Test(Subset(g, May(Star(Relation(r, transition), paths), g)))) |
            Left(id: i, region: r), Goal(id: j, region: g);
    });
    drop((faces, plan, states, env));
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let rows = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    drop((db, template, prepared));
    assert_eq!(rows.len(), 64);
    for n in 0..rows.len() {
        let r = scalar(rows.get(n, 0));
        let g = scalar(rows.get(n, 1));
        let unary =
            |f: &dyn Fn(u64) -> bool| (0..2).filter(|&s| f(s)).fold(0, |bits, s| bits | (1 << s));
        let domain = unary(&|s| (0..2).any(|t| cell(r, s, t)));
        let range = unary(&|t| (0..2).any(|s| cell(r, s, t)));
        let may = unary(&|s| (0..2).any(|t| cell(r, s, t) && g & (1 << t) != 0));
        let all = unary(&|s| (0..2).all(|t| !cell(r, s, t) || g & (1 << t) != 0));
        let post = unary(&|t| (0..2).any(|s| cell(r, s, t) && g & (1 << s) != 0));
        // Two states: every reachable pair has a simple path of at most one edge.
        let star = r | 9;
        let reach = unary(&|s| (0..2).any(|t| cell(star, s, t) && g & (1 << t) != 0));
        let test = matrix(|s, t| s == t && g & (1 << s) != 0);
        for (column, expected) in [
            domain,
            range,
            may,
            all,
            domain & all,
            post,
            9,
            test,
            star,
            reach,
            range,
            0,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                mask(rows.get(n, column + 2)),
                expected,
                "relation={r}, goal={g}, column={column}"
            );
        }
        assert_eq!(rows.get(n, 14), AnswerValue::Bool(true));
    }
}

#[test]
fn role_errors_refuse_even_with_empty_bodies() {
    let directory = common::TempDir::new("event-query-roles");
    let db = Db::create(directory.path(), RelationQueries, common::work())
        .unwrap()
        .unwrap();
    let env = Space::new(SpaceId([40; 32]), 0, &()).unwrap();
    let a = Space::new(SpaceId([41; 32]), 1, &()).unwrap();
    let b = Space::new(SpaceId([42; 32]), 1, &()).unwrap();
    let forward = import(AdmittedDescriptor::Fibre(pair(43, &a, &b, &env)));
    let template = query!(RelationQueries {
        use faces forward = &forward;
        (out: Event(Region(Relation(r, forward) | Converse(Relation(r, forward))))) | Left(region: r);
    });
    let FindTerm::Event(expr) = &template.rules()[0].finds[0] else {
        unreachable!()
    };
    assert_eq!(
        expr.validate_shape(),
        Err(EventExprError::IncompatibleRoles)
    );
    assert!(db.prepare(&template, common::work()).is_err());
    let identity = query!(RelationQueries {
        use faces forward = &forward;
        (out: Event(Region(Id(forward)))) | Left(region: r);
    });
    assert!(db.prepare(&identity, common::work()).is_err());
}

#[test]
fn independently_named_products_reindex_converse_without_changing_role_meaning() {
    let directory = common::TempDir::new("event-query-reindex");
    let db = Db::create(directory.path(), RelationQueries, common::work())
        .unwrap()
        .unwrap();
    let env = Space::new(SpaceId([50; 32]), 0, &()).unwrap();
    let states = Space::new(SpaceId([51; 32]), 1, &()).unwrap();
    let first = pair(52, &states, &states, &env);
    let second = pair(53, &states, &states, &env);
    db.write(common::work(), |tx| {
        for bits in 0..16 {
            tx.insert([&Left {
                id: bits,
                region: region(first.space(), bits),
            }])?;
            tx.insert([&Right {
                id: bits,
                region: region(second.space(), bits),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let first = import(AdmittedDescriptor::Fibre(first));
    let second = import(AdmittedDescriptor::Fibre(second));
    let template = query!(RelationQueries {
        use faces first = &first;
        use faces second = &second;
        (id,
         same: Event(Region(Relation(a, first) ^ Relation(b, second))),
         asymmetric: Event(Region(Relation(a, first) ^ Converse(Relation(b, second))))) |
            Left(id: id, region: a), Right(id: id, region: b);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let rows = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    for n in 0..rows.len() {
        let bits = scalar(rows.get(n, 0));
        assert_eq!(mask(rows.get(n, 1)), 0);
        assert_eq!(mask(rows.get(n, 2)), bits ^ matrix(|s, t| cell(bits, t, s)));
    }
    assert_eq!(rows.len(), 16);
}

#[test]
fn modal_faults_retain_occurrences_and_cannot_hide_behind_dead_ends() {
    let directory = common::TempDir::new("event-query-modal-faults");
    let db = Db::create(directory.path(), RelationQueries, common::work())
        .unwrap()
        .unwrap();
    let env = Space::new(SpaceId([60; 32]), 0, &()).unwrap();
    let states = Space::new(SpaceId([61; 32]), 1, &()).unwrap();
    let faces = pair(62, &states, &states, &env);
    db.write(common::work(), |tx| {
        tx.insert([
            &Left {
                id: 0,
                region: faces.space().empty(),
            },
            &Left {
                id: 1,
                region: states.empty(),
            },
        ])
    })
    .unwrap()
    .unwrap();
    let faces = import(AdmittedDescriptor::Fibre(faces));
    let template = query!(RelationQueries {
        use faces step = &faces;
        (out: Event(All(Relation(r, step), Full(r)))) | Left(region: r);
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let bumbledb::Error::EventFaults(faults) = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err()
        else {
            panic!("context faults")
        };
        assert_eq!(faults.len(), 2);
        assert_eq!(faults[0].operand, 0);
        assert_eq!(faults[1].operand, 1);
        assert_eq!(
            &*faults[0].offending_value,
            states.empty().to_bytes(&()).unwrap()
        );
        assert_eq!(
            &*faults[1].expected_space,
            states.full().to_bytes(&()).unwrap()
        );
        retained.push(faults);
    }
    assert_eq!(retained[0], retained[1]);
}

#[test]
fn roles_compare_actual_environment_readouts_not_just_their_ranges() {
    let env = Space::new(SpaceId([70; 32]), 1, &()).unwrap();
    let states = Space::new(SpaceId([71; 32]), 1, &()).unwrap();
    let positive = CoordinateMap::new(&states, &env, &[states.coordinate(0, &()).unwrap()], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let negative = CoordinateMap::new(
        &states,
        &env,
        &[states.coordinate(0, &()).unwrap().complement()],
        &(),
    )
    .unwrap()
    .certify_surjective(&())
    .unwrap();
    let same = import(AdmittedDescriptor::Fibre(
        FibreProduct::new(SpaceId([72; 32]), &positive, &positive, &()).unwrap(),
    ));
    let changed = import(AdmittedDescriptor::Fibre(
        FibreProduct::new(SpaceId([73; 32]), &negative, &positive, &()).unwrap(),
    ));
    let template = query!(RelationQueries {
        use faces first = &same;
        use faces second = &changed;
        (out: Event(Region(Relation(a, first) | Relation(b, second)))) | Left(region: a), Right(region: b);
    });
    let FindTerm::Event(expr) = &template.rules()[0].finds[0] else {
        unreachable!()
    };
    assert_eq!(
        expr.validate_shape(),
        Err(EventExprError::IncompatibleRoles)
    );
}

#[test]
fn asymmetric_legal_supports_and_shared_environments_survive_query_products() {
    let directory = common::TempDir::new("event-query-shared-environment");
    let db = Db::create(directory.path(), RelationQueries, common::work())
        .unwrap()
        .unwrap();
    let env = Space::new(SpaceId([80; 32]), 1, &()).unwrap();
    let domain = |id, support| {
        let raw = Space::new(SpaceId([id; 32]), 2, &()).unwrap();
        raw.restrict(&region(&raw, support), &()).unwrap()
    };
    let source = domain(81, 7);
    let middle = domain(82, 15);
    let target = domain(83, 11);
    let base = |s| {
        CoordinateMap::coordinates(s, &env, &[0], &())
            .unwrap()
            .certify_surjective(&())
            .unwrap()
    };
    let first = FibreProduct::new(SpaceId([84; 32]), &base(&source), &base(&middle), &()).unwrap();
    let second = FibreProduct::new(SpaceId([85; 32]), &base(&middle), &base(&target), &()).unwrap();
    let outer = FibreProduct::new(SpaceId([86; 32]), &base(&source), &base(&target), &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([87; 32]), &first, &second, &outer, &()).unwrap();
    let patterns = [0, 65535, 43690, 21845, 52428, 13107, 42240, 23205];
    db.write(common::work(), |tx| {
        for bits in patterns {
            tx.insert([&Left {
                id: bits,
                region: region(first.space(), bits),
            }])?;
            tx.insert([&Right {
                id: bits,
                region: region(second.space(), bits),
            }])?;
            tx.insert([&Bound {
                id: bits,
                region: region(outer.space(), bits),
            }])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let first = import(AdmittedDescriptor::Fibre(first));
    let second = import(AdmittedDescriptor::Fibre(second));
    let outer = import(AdmittedDescriptor::Fibre(outer));
    let path = import(AdmittedDescriptor::Composition(plan));
    let template = query!(RelationQueries {
        use faces first = &first;
        use faces second = &second;
        use faces outer = &outer;
        use product path = &path;
        (i, j, k,
         composed: Event(Region(Compose(Relation(a, first), Relation(b, second), path))),
         left: Event(Region(LeftResidual(Relation(a, first), Relation(c, outer), path))),
         right: Event(Region(RightResidual(Relation(c, outer), Relation(b, second), path)))) |
            Left(id: i, region: a), Right(id: j, region: b), Bound(id: k, region: c);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let rows = db
        .read(common::work(), |snapshot| {
            snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(rows.len(), 512);
    for n in 0..rows.len() {
        let expected = coupled_oracle(
            scalar(rows.get(n, 0)),
            scalar(rows.get(n, 1)),
            scalar(rows.get(n, 2)),
        );
        for (column, value) in expected.into_iter().enumerate() {
            assert_eq!(mask(rows.get(n, column + 3)), value);
        }
    }
}

fn coupled_oracle(left: u64, right: u64, bound: u64) -> [u64; 3] {
    let legal = |support: u64, s: u64| support & (1 << s) != 0;
    let pair = |a: u64, b: u64| a & 1 == b & 1;
    let cell = |bits: u64, a: u64, b: u64| bits & (1 << (a + 4 * b)) != 0;
    let mut expected = [0; 3];
    for a in 0..4 {
        for b in 0..4 {
            if legal(7, a)
                && legal(11, b)
                && pair(a, b)
                && (0..4).any(|m| pair(a, m) && cell(left, a, m) && cell(right, m, b))
            {
                expected[0] |= 1 << (a + 4 * b);
            }
            if legal(11, b)
                && pair(a, b)
                && (0..4)
                    .all(|s| !legal(7, s) || !pair(s, a) || !cell(left, s, a) || cell(bound, s, b))
            {
                expected[1] |= 1 << (a + 4 * b);
            }
            if legal(7, a)
                && pair(a, b)
                && (0..4).all(|u| {
                    !legal(11, u) || !pair(b, u) || !cell(right, b, u) || cell(bound, a, u)
                })
            {
                expected[2] |= 1 << (a + 4 * b);
            }
        }
    }
    expected
}
