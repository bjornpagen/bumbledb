use bumbledb::{Db, Key};

mod common;

bumbledb::schema! {
    pub KeyedGet;

    closed relation Kind as KindId = { Alpha, Beta };

    relation Grp {
        id: u64 as GrpId, fresh,
        label: str,
    }
    relation Task {
        id: u64 as TaskId, fresh,
        kind: u64 as KindId,
        subject: u64 as GrpId,
        note: str,
    }
    relation Meta {
        grp: u64 as GrpId,
        title: str,
    }

    Task(subject) <= Grp(id);
    Meta(grp) == Grp(id);
    Task(kind, subject) -> Task;
    Grp(label) -> Grp;

    Meta(grp) -> Meta;
}

#[test]
fn keyed_get_reads_through_a_declared_key_on_both_scopes() {
    let dir = common::TempDir::new("keyed-get-both-scopes");
    let db = Db::create(dir.path(), KeyedGet)
        .expect("create")
        .expect("accepted");
    let (grp, task) = db
        .write(|tx| {
            let grp = tx.reserve::<GrpId>(1)?.start().expect("nonempty");
            tx.insert([&Grp {
                id: grp,
                label: "home",
            }])?;
            tx.insert([&Meta {
                grp,
                title: "the home group",
            }])?;
            let task = tx.reserve::<TaskId>(1)?.start().expect("nonempty");
            tx.insert([&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "water",
            }])?;
            Ok((grp, task))
        })
        .expect("seed")
        .unwrap()
        .value;

    db.read(|snap| {
        assert_eq!(
            snap.get(TaskByKindSubject {
                kind: Kind::Alpha.id(),
                subject: grp,
            })?,
            Some(Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "water",
            })
        );

        assert_eq!(
            snap.get(TaskByKindSubject {
                kind: Kind::Beta.id(),
                subject: grp,
            })?,
            None
        );
        Ok(())
    })
    .expect("snapshot keyed get");

    db.write(|tx| {
        assert_eq!(
            tx.get(TaskByKindSubject {
                kind: Kind::Alpha.id(),
                subject: grp,
            })?,
            Some(Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "water",
            })
        );
        Ok(())
    })
    .expect("write-scope keyed get")
    .unwrap();
}

/// The const-id arithmetic under mirror offsets: schema admission succeeds with
/// a bidirectional `==` occupying TWO materialized slots before the declared
/// keys, and a keyed get through BOTH generated structs answers correctly —
/// this test fails if the expansion's statement-id computation is off by one.
#[test]
fn keyed_get_statement_ids_survive_mirror_offsets() {
    assert_eq!(
        <TaskByKindSubject as Key>::STATEMENT,
        bumbledb::schema::StatementId(6)
    );
    assert_eq!(
        <GrpByLabel as Key>::STATEMENT,
        bumbledb::schema::StatementId(7)
    );

    let dir = common::TempDir::new("keyed-get-mirror-offsets");
    let db = Db::create(dir.path(), KeyedGet)
        .expect("schema admission succeeds")
        .expect("accepted");
    let grp = db
        .write(|tx| {
            let grp = tx.reserve::<GrpId>(1)?.start().expect("nonempty");
            tx.insert([&Grp {
                id: grp,
                label: "inbox",
            }])?;
            tx.insert([&Meta {
                grp,
                title: "the inbox",
            }])?;
            let task = tx.reserve::<TaskId>(1)?.start().expect("nonempty");
            tx.insert([&Task {
                id: task,
                kind: Kind::Beta.id(),
                subject: grp,
                note: "triage",
            }])?;
            Ok(grp)
        })
        .expect("seed")
        .unwrap()
        .value;
    db.read(|snap| {
        let by_key = snap
            .get(TaskByKindSubject {
                kind: Kind::Beta.id(),
                subject: grp,
            })?
            .expect("the composite key answers");
        assert_eq!(by_key.note, "triage");
        let by_label = snap
            .get(GrpByLabel { label: "inbox" })?
            .expect("the label key answers");
        assert_eq!(by_label.id, grp);
        Ok(())
    })
    .expect("keyed gets through both generated structs");
}

/// String determinants resolve, never mint: a never-interned label proves
/// absence on a snapshot; inside a write transaction a NOVEL label (a
/// provisional intern id in the pending delta) is found — read-your-writes —
/// and after the compensating delete the key answers `None` again.
#[test]
fn keyed_get_string_keys_resolve_pending_first_and_never_mint() {
    let dir = common::TempDir::new("keyed-get-string-keys");
    let db = Db::create(dir.path(), KeyedGet)
        .expect("create")
        .expect("accepted");

    db.read(|snap| {
        assert_eq!(
            snap.get(GrpByLabel {
                label: "never-interned",
            })?,
            None
        );
        Ok(())
    })
    .expect("a never-interned label proves absence");

    db.write(|tx| {
        let grp = tx.reserve::<GrpId>(1)?.start().expect("nonempty");
        tx.insert([&Grp {
            id: grp,
            label: "novel-label",
        }])?;

        assert_eq!(
            tx.get(GrpByLabel {
                label: "novel-label",
            })?,
            Some(Grp {
                id: grp,
                label: "novel-label",
            })
        );
        tx.delete([&Grp {
            id: grp,
            label: "novel-label",
        }])?;
        assert_eq!(
            tx.get(GrpByLabel {
                label: "novel-label",
            })?,
            None
        );
        Ok(())
    })
    .expect("pending-first resolution")
    .unwrap();
}

#[test]
fn keyed_get_observes_the_final_state_overlay() {
    let dir = common::TempDir::new("keyed-get-final-state");
    let db = Db::create(dir.path(), KeyedGet)
        .expect("create")
        .expect("accepted");
    let (grp, task) = db
        .write(|tx| {
            let grp = tx.reserve::<GrpId>(1)?.start().expect("nonempty");
            tx.insert([&Grp {
                id: grp,
                label: "garden",
            }])?;
            tx.insert([&Meta {
                grp,
                title: "the garden",
            }])?;
            let task = tx.reserve::<TaskId>(1)?.start().expect("nonempty");
            let key = TaskByKindSubject {
                kind: Kind::Alpha.id(),
                subject: grp,
            };

            tx.insert([&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "sow",
            }])?;
            assert_eq!(
                tx.get(key)?,
                Some(Task {
                    id: task,
                    kind: Kind::Alpha.id(),
                    subject: grp,
                    note: "sow",
                })
            );

            tx.delete([&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "sow",
            }])?;
            assert_eq!(tx.get(key)?, None);

            tx.insert([&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "harvest",
            }])?;
            assert_eq!(
                tx.get(key)?,
                Some(Task {
                    id: task,
                    kind: Kind::Alpha.id(),
                    subject: grp,
                    note: "harvest",
                })
            );
            Ok((grp, task))
        })
        .expect("pre-commit overlay reads")
        .unwrap()
        .value;

    db.read(|snap| {
        assert_eq!(
            snap.get(TaskByKindSubject {
                kind: Kind::Alpha.id(),
                subject: grp,
            })?,
            Some(Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "harvest",
            })
        );
        Ok(())
    })
    .expect("post-commit keyed get");
}
