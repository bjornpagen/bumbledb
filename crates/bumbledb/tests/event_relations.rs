use bumbledb::{
    AnswerValue, BindValue, Db, Event, Fact,
    event::{
        AdmittedDescriptor, CoordinateMap, Descriptor, DescriptorLimits, FibreProduct,
        RelationalProduct, Space, SpaceId, WorldRelation,
    },
    ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId},
    schema::FieldId,
};

mod common;

bumbledb::schema! {
    pub RelationSchema;
    relation Permission { id: u64, when: event }
    relation Expected { id: u64, when: event }
    Permission(id) -> Permission;
    Expected(id) -> Expected;
}

#[test]
fn computed_permissions_reopen_join_and_recover_their_relation_view() {
    let directory = common::TempDir::new("event-relational-permissions");
    let db = Db::create(directory.path(), RelationSchema, common::work())
        .unwrap()
        .unwrap();
    let environment = Space::new(SpaceId([1; 32]), 0, &()).unwrap();
    let states = Space::new(SpaceId([2; 32]), 1, &()).unwrap();
    let base = CoordinateMap::new(&states, &environment, &[], &())
        .unwrap()
        .certify_surjective(&())
        .unwrap();
    let pair = FibreProduct::new(SpaceId([3; 32]), &base, &base, &()).unwrap();
    let plan = RelationalProduct::new(SpaceId([4; 32]), &pair, &pair, &pair, &()).unwrap();
    let relation = |mask| {
        WorldRelation::new(&pair, &pair.space().table(3, &[mask], &()).unwrap(), &()).unwrap()
    };
    // R permits both middle states; V permits only target 0. The maximal
    // continuation therefore permits target 0 from either middle state.
    let permission = plan
        .left_residual(&relation(15), &relation(3), &common::work())
        .unwrap();
    let expected = pair.space().table(3, &[3], &()).unwrap();
    let independently_owned = Event::from_bytes(&expected.to_bytes(&()).unwrap(), &()).unwrap();
    db.write(common::work(), |transaction| {
        transaction.insert([&Permission {
            id: 1,
            when: permission.region().clone(),
        }])?;
        transaction.insert([&Expected {
            id: 1,
            when: independently_owned,
        }])?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let descriptor_limits = DescriptorLimits::default();
    let pair_bytes = Descriptor::capture(
        &AdmittedDescriptor::Fibre(pair.clone()),
        descriptor_limits,
        &(),
    )
    .unwrap()
    .to_bytes(descriptor_limits, &())
    .unwrap();
    drop((
        plan,
        permission,
        expected,
        db,
        pair,
        states,
        base,
        environment,
    ));
    let AdmittedDescriptor::Fibre(pair) =
        Descriptor::import(&pair_bytes, descriptor_limits, &()).unwrap()
    else {
        panic!("restored product");
    };
    let db = Db::open(directory.path(), RelationSchema, common::work()).unwrap();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Permission::RELATION, Expected::RELATION]
            .into_iter()
            .map(|source| Atom {
                source: AtomSource::Edb(source),
                bindings: vec![(FieldId(1), Term::Var(VarId(0)))],
            })
            .collect(),
        negated: vec![],
        conditions: vec![],
    });
    let mut retained = Vec::new();
    for cursor in [false, true] {
        let mut prepared = db.prepare(&query, common::work()).unwrap();
        prepared.force_cursor_fallback(cursor);
        let answers = db
            .read(common::work(), |snapshot| {
                snapshot.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        assert_eq!(answers.len(), 1);
        retained.push(answers);
    }
    drop(db);
    for answers in retained {
        let AnswerValue::Event(region) = answers.get(0, 0) else {
            panic!("owned Event")
        };
        // Reattach the independently imported BEDC descriptor. Every original
        // map, product and database owner was dropped before reopening.
        check_restored_permission(&pair, region);
    }
}

fn check_restored_permission(pair: &FibreProduct, region: &Event) {
    let states = pair.left().map().target();
    let permission = WorldRelation::new(pair, region, &()).unwrap();
    assert!(permission.domain(&()).unwrap().is_full());
    assert_eq!(permission.range(&()).unwrap().count(&()).unwrap(), 1);
    let output = states.coordinate(0, &()).unwrap().complement();
    assert!(permission.must(&output, &()).unwrap().is_full());
    assert_eq!(permission.readout(&()).unwrap().map_world(1).unwrap(), 0);
    let observation = permission.readout(&()).unwrap();
    let information = observation
        .information(
            &states.coordinate(0, &()).unwrap(),
            &states.full(),
            &common::work(),
        )
        .unwrap();
    assert!(information.ambiguous().is_full());
    assert!(information.guaranteed().is_empty());
    // Inspection is also available to an external database consumer. The
    // snapshot retains support after both database and row owners go away.
    let diagram = region.diagram(&common::work()).unwrap();
    let rebuilt = diagram.rebuild(pair.space(), &common::work()).unwrap();
    assert_eq!(rebuilt, *permission.region());
    for code in 0..4 {
        assert_eq!(diagram.contains(code).unwrap(), code < 2);
    }
}
