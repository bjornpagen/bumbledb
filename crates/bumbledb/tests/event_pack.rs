//! Grouped union and fault participation, independently checked as finite sets.
use bumbledb::{
    AnswerValue, BindValue, Db, Error, Event,
    event::{Space, SpaceId},
    query,
};
mod common;

bumbledb::schema! {
    pub EventPacking;
    relation Claim { id: u64, group: u64, region: event }
    relation Pair { id: u64, left: event, right: event, claim: event }
    relation Gate { id: u64 }
    Claim(id) -> Claim;
    Pair(id) -> Pair;
    Gate(id) -> Gate;
}

fn mask(value: &Event) -> u64 {
    (0..4)
        .filter(|&w| value.contains(w).unwrap_or(false))
        .fold(0, |bits, w| bits | (1 << w))
}

#[test]
fn grouped_unions_match_world_sets_through_interiors_and_independent_owners() {
    for support in [15, 7, 5] {
        let directory = common::TempDir::new(&format!("event-pack-{support}"));
        let db = Db::create(directory.path(), EventPacking, common::work())
            .unwrap()
            .unwrap();
        let initial = Space::new(SpaceId([41; 32]), 2, &()).unwrap();
        let space = initial
            .restrict(&initial.table(3, &[support], &()).unwrap(), &())
            .unwrap();
        db.write(common::work(), |tx| {
            for a in 0..16 {
                for b in 0..16 {
                    if (a | b) & !support != 0 {
                        continue;
                    }
                    let group = 16 * a + b;
                    for (i, bits) in [a, b, a].into_iter().enumerate() {
                        let independently_owned =
                            Event::from_bytes(&space.table(3, &[bits], &())?.to_bytes(&())?, &())?;
                        tx.insert([&Claim {
                            id: 3 * group + i as u64,
                            group,
                            region: independently_owned,
                        }])?;
                    }
                }
            }
            Ok(())
        })
        .unwrap()
        .unwrap();
        let template = query!(EventPacking {
            interior packed(group, covered: Pack(a)) | Claim(group, region: a);
            interior packed(group, covered: Pack(a)) | Claim(region: a, group);
            (group, covered, missing: Event(!covered)) | packed(group, covered);
        });
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        for fallback in [false, true] {
            prepared.force_cursor_fallback(fallback);
            let answers = db
                .read(common::work(), |s| {
                    s.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap();
            assert_eq!(answers.len(), 1 << (2 * support.count_ones()));
            for row in 0..answers.len() {
                let AnswerValue::U64(group) = answers.get(row, 0) else {
                    panic!("group");
                };
                let AnswerValue::Event(covered) = answers.get(row, 1) else {
                    panic!("union");
                };
                let AnswerValue::Event(missing) = answers.get(row, 2) else {
                    panic!("complement");
                };
                let union = (group / 16) | (group % 16);
                assert_eq!(mask(covered), union);
                assert_eq!(mask(missing), !union & support);
            }
        }
        let template = query!(EventPacking { (covered: Pack(a)) | Claim(region: a); });
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        let answers = db
            .read(common::work(), |s| {
                s.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap();
        drop(prepared);
        drop(db);
        assert_eq!(answers.len(), 1);
        let AnswerValue::Event(value) = answers.get(0, 0) else {
            panic!("owned union");
        };
        assert_eq!(mask(value), support);
    }
}

#[test]
fn absent_groups_stay_absent_and_scoped_empty_seeds_stay_values() {
    let directory = common::TempDir::new("event-pack-empty");
    let db = Db::create(directory.path(), EventPacking, common::work())
        .unwrap()
        .unwrap();
    let template = query!(EventPacking { (covered: Pack(a)) | Claim(region: a); });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    let empty = db
        .read(common::work(), |s| {
            s.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert!(empty.is_empty());
    let source = Space::new(SpaceId([42; 32]), 2, &()).unwrap();
    db.write(common::work(), |tx| {
        tx.insert([&Claim {
            id: 1,
            group: 0,
            region: source.empty(),
        }])
    })
    .unwrap()
    .unwrap();
    let seeded = db
        .read(common::work(), |s| {
            s.execute_collect(&mut prepared, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(seeded.len(), 1);
    let AnswerValue::Event(value) = seeded.get(0, 0) else {
        panic!("empty value");
    };
    assert!(value.is_empty());
}

#[test]
fn pack_faults_use_canonical_group_context_and_retain_written_provenance() {
    let mut sources: Vec<_> = [43, 44, 45]
        .map(|id| Space::new(SpaceId([id; 32]), 2, &()).unwrap())
        .into();
    sources.sort_by_key(|space| space.full().to_bytes(&()).unwrap());
    let [a, b, c] = sources.as_slice() else {
        unreachable!();
    };
    let mut expected = None;
    for reverse in [false, true] {
        let directory = common::TempDir::new(&format!("event-pack-faults-{reverse}"));
        let db = Db::create(directory.path(), EventPacking, common::work())
            .unwrap()
            .unwrap();
        let mut rows = vec![
            Claim {
                id: 1,
                group: 0,
                region: b.full(),
            },
            Claim {
                id: 2,
                group: 0,
                region: c.empty(),
            },
            Claim {
                id: 3,
                group: 0,
                region: a.full(),
            },
            Claim {
                id: 4,
                group: 1,
                region: c.full(),
            },
            Claim {
                id: 5,
                group: 0,
                region: Space::new(SpaceId([40; 32]), 2, &()).unwrap().empty(),
            },
        ];
        if reverse {
            rows.reverse();
        }
        db.write(common::work(), |tx| {
            for row in &rows {
                tx.insert([row])?;
            }
            for id in 1..=4 {
                tx.insert([&Gate { id }])?;
            }
            Ok(())
        })
        .unwrap()
        .unwrap();
        let template = query!(EventPacking {
            (group, covered: Pack(a)) | Claim(id, group, region: a), Gate(id: id);
            (group, covered: Pack(a)) | Claim(id, group, region: a), Gate(id: id), or(id == 1, id == 1);
            (group, covered: Pack(a)) | Claim(id, group, region: a), Gate(id: id);
        });
        let mut prepared = db.prepare(&template, common::work()).unwrap();
        for fallback in [false, true] {
            prepared.force_cursor_fallback(fallback);
            let error = db
                .read(common::work(), |s| {
                    s.execute_collect(&mut prepared, &[] as &[BindValue])
                })
                .unwrap_err();
            let Error::EventFaults(faults) = error else {
                panic!("complete faults: {error:?}");
            };
            assert_eq!(
                faults.len(),
                5,
                "two foreign values in rules 0/2; one in rule 1"
            );
            assert!(
                faults
                    .iter()
                    .all(|f| f.expected_space.as_ref() == a.full().to_bytes(&()).unwrap())
            );
            assert!(faults.windows(2).all(|p| p[0] < p[1]));
            if let Some(expected) = &expected {
                assert_eq!(&faults, expected);
            } else {
                expected = Some(faults);
            }
        }
    }
}

#[test]
fn bad_computed_keys_do_not_hide_later_pack_faults_or_invent_groups() {
    let directory = common::TempDir::new("event-pack-computed-faults");
    let db = Db::create(directory.path(), EventPacking, common::work())
        .unwrap()
        .unwrap();
    let a = Space::new(SpaceId([46; 32]), 2, &()).unwrap();
    let b = Space::new(SpaceId([47; 32]), 2, &()).unwrap();
    let c = Space::new(SpaceId([48; 32]), 2, &()).unwrap();
    db.write(common::work(), |tx| {
        for row in [
            Pair {
                id: 0,
                left: a.full(),
                right: b.full(),
                claim: c.full(),
            },
            Pair {
                id: 1,
                left: a.full(),
                right: a.empty(),
                claim: a.full(),
            },
            Pair {
                id: 2,
                left: a.full(),
                right: a.empty(),
                claim: b.empty(),
            },
            Pair {
                id: 3,
                left: a.empty(),
                right: a.empty(),
                claim: c.full(),
            },
        ] {
            tx.insert([&row])?;
        }
        Ok(())
    })
    .unwrap()
    .unwrap();
    let template = query!(EventPacking {
        interior produced(key: Event(a | b), covered: Pack(c)) | Pair(left: a, right: b, claim: c);
        (covered) | produced(key, covered), Gate(id == 999);
    });
    let mut prepared = db.prepare(&template, common::work()).unwrap();
    for fallback in [false, true] {
        prepared.force_cursor_fallback(fallback);
        let error = db
            .read(common::work(), |s| {
                s.execute_collect(&mut prepared, &[] as &[BindValue])
            })
            .unwrap_err();
        let Error::EventFaults(faults) = error else {
            panic!("mixed faults: {error:?}");
        };
        assert_eq!(faults.len(), 2);
        assert!(faults.iter().all(|f| f.stage == Some(0)));
        assert_eq!(
            faults.iter().map(|f| f.find).collect::<Vec<_>>(),
            vec![0, 1]
        );
    }
}
