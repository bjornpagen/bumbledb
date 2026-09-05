//! Gate: real indexed point access (F3 finding B).
//!
//! The store maintains schema-derived determinant index entries atomically
//! with every row mutation, and the keyed access paths — `ReadInstance`
//! point gets, prepared key-probe queries, and candidate judgment
//! enumeration — resolve through fingerprint buckets plus exact
//! decoded-value confirmation instead of scanning the relation.
//!
//! Gate families: G04 (query denotation on the probe path), G05
//! (bucket-shaped access work — no relation-sized mandatory pass), G15
//! (the performance-first access contract, asserted STRUCTURALLY on the
//! deterministic work ledger, never on timing); audits PERF-001,
//! Q-COLLISION, E-ADMIT, HASH-02; chapters 10 §3–4, 12, 41.
//!
//! Every assertion here runs the ACTUAL public-to-native call path:
//! `Db`/`ReadInstance`/prepared queries, and — where forcing collisions
//! requires the store's probe constructors (`collision-probe` feature) —
//! the same `bumbledb::store` candidate protocol the log bridge drives.
//! The scan oracle is the public `scan_facts` walk; results must agree.

use bumbledb::work::Resource;
use bumbledb::{BindValue, Db};

mod common;

bumbledb::schema! {
    pub GateIdx;

    relation Acct {
        id: u64 as AcctId,
        note: str,
    }
    relation Task {
        kind: u64,
        subject: u64,
        note: str,
    }
    relation Doc {
        title: str,
        body: str,
    }

    Acct(id) -> Acct;
    Task(kind, subject) -> Task;
    Doc(title) -> Doc;
}

fn seeded_db(dir: &common::TempDir, accounts: u64) -> Db<GateIdx> {
    let db = Db::create(dir.path(), GateIdx, common::work())
        .expect("create")
        .expect("accepted");
    common::expect_admitted(db.write(common::work(), |tx| {
        for n in 0..accounts {
            tx.insert([&Acct {
                id: AcctId(n),
                note: "seeded",
            }])?;
            tx.insert([&Task {
                kind: n % 7,
                subject: n,
                note: "task",
            }])?;
        }
        Ok(())
    }));
    db
}

/// Work-unit cost of one closure over a fresh read lease (each `Db::read`
/// lease starts a fresh embedded ledger, so the delta is the closure's own
/// deterministic charge — a structural count, never timing).
fn lease_work<R>(
    db: &Db<GateIdx>,
    f: impl FnOnce(&bumbledb::ReadFrame<'_, GateIdx>) -> bumbledb::Result<R>,
) -> (R, u64) {
    db.read(common::work(), |snap| {
        let before = snap.work().used(Resource::WorkUnits);
        let out = f(snap)?;
        Ok((out, snap.work().used(Resource::WorkUnits) - before))
    })
    .expect("read lease")
}

/// A generous flat ceiling for one indexed point access (bucket walk, one
/// fetch, one decode). A relation scan charges at least one work unit per
/// row, so the seeded sizes below sit far above this ceiling.
const POINT_ACCESS_CEILING: u64 = 256;

#[test]
fn indexed_point_reads_are_bucket_shaped_across_growing_relations() {
    let mut per_size = Vec::new();
    for (tag, size) in [("small", 64u64), ("large", 4096u64)] {
        let dir = common::TempDir::new(&format!("gate-idx-point-{tag}"));
        let db = seeded_db(&dir, size);

        let (hit, hit_work) = lease_work(&db, |snap| {
            Ok(snap
                .get(AcctById {
                    id: AcctId(size / 2),
                })?
                .map(|fact| fact.id))
        });
        assert_eq!(hit, Some(AcctId(size / 2)), "indexed hit at size {size}");
        let (miss, miss_work) = lease_work(&db, |snap| {
            Ok(snap
                .get(AcctById {
                    id: AcctId(size + 9),
                })?
                .map(|fact| fact.id))
        });
        assert!(miss.is_none(), "indexed miss at size {size}");

        // Oracle: the public scan agrees with the indexed answer.
        let scanned = db
            .read(common::work(), |snap| {
                Ok(snap
                    .scan_facts::<Acct>()?
                    .collect::<bumbledb::Result<Vec<_>>>()?
                    .iter()
                    .find(|acct| acct.id == AcctId(size / 2))
                    .map(|acct| acct.id))
            })
            .expect("oracle scan");
        assert_eq!(scanned, Some(AcctId(size / 2)));

        assert!(
            hit_work < POINT_ACCESS_CEILING && miss_work < POINT_ACCESS_CEILING,
            "point access at {size} rows must not scan: hit {hit_work}, miss {miss_work} work units"
        );
        per_size.push((hit_work, miss_work));
    }
    // STRUCTURAL: 64x more rows must not grow the access work (identical
    // bucket shapes; the ledger is deterministic, so equality-up-to-slack
    // is assertable). A scan would grow ~64x.
    let (small_hit, small_miss) = per_size[0];
    let (large_hit, large_miss) = per_size[1];
    assert!(
        large_hit <= small_hit.saturating_mul(2) && large_miss <= small_miss.saturating_mul(2),
        "keyed access work grew with relation size: {small_hit}/{small_miss} -> {large_hit}/{large_miss}"
    );
}

#[test]
fn composite_keys_and_mutation_maintenance() {
    let dir = common::TempDir::new("gate-idx-composite");
    let db = seeded_db(&dir, 128);

    // Composite (kind, subject) hit and miss.
    let (found, work) = lease_work(&db, |snap| {
        Ok(snap
            .get(TaskByKindSubject {
                kind: 5,
                subject: 5,
            })?
            .map(|task| task.subject))
    });
    assert_eq!(found, Some(5));
    assert!(work < POINT_ACCESS_CEILING, "composite hit scanned: {work}");
    let (absent, _) = lease_work(&db, |snap| {
        Ok(snap
            .get(TaskByKindSubject {
                kind: 6,
                subject: 5,
            })?
            .map(|task| task.subject))
    });
    assert!(absent.is_none(), "wrong composite prefix must miss");

    // Replacement (one command: delete old + insert new under the same
    // key): the index follows atomically.
    common::expect_admitted(db.write(common::work(), |tx| {
        tx.delete([&Task {
            kind: 5,
            subject: 5,
            note: "task",
        }])?;
        tx.insert([&Task {
            kind: 5,
            subject: 5,
            note: "replaced",
        }])?;
        Ok(())
    }));
    let (replaced, _) = lease_work(&db, |snap| {
        Ok(snap
            .get(TaskByKindSubject {
                kind: 5,
                subject: 5,
            })?
            .map(|task| task.note.to_string()))
    });
    assert_eq!(replaced, Some("replaced".into()));

    // Deletion: the keyed read misses, exactly like the scan oracle.
    common::expect_admitted(db.write(common::work(), |tx| {
        tx.delete([&Task {
            kind: 5,
            subject: 5,
            note: "replaced",
        }])?;
        Ok(())
    }));
    let (gone, _) = lease_work(&db, |snap| {
        Ok(snap
            .get(TaskByKindSubject {
                kind: 5,
                subject: 5,
            })?
            .map(|task| task.subject))
    });
    assert!(gone.is_none(), "a deleted fact's key must miss");
    let oracle = db
        .read(common::work(), |snap| {
            Ok(snap
                .scan_facts::<Task>()?
                .collect::<bumbledb::Result<Vec<_>>>()?
                .into_iter()
                .any(|task| task.subject == 5 && task.kind == 5))
        })
        .expect("oracle scan");
    assert!(!oracle, "the oracle agrees the fact is gone");
}

#[test]
fn long_text_determinants_resolve_exactly() {
    let dir = common::TempDir::new("gate-idx-long-text");
    let db = Db::create(dir.path(), GateIdx, common::work())
        .expect("create")
        .expect("accepted");
    // Titles far past LMDB's 511-byte key bound: the determinant is
    // fingerprinted, never keyed raw (chapter 10 §3).
    let title_a = "a".repeat(4096) + " — the long one";
    let title_b = "a".repeat(4096) + " — the long two";
    common::expect_admitted(db.write(common::work(), |tx| {
        tx.insert([&Doc {
            title: &title_a,
            body: "body a",
        }])?;
        tx.insert([&Doc {
            title: &title_b,
            body: "body b",
        }])?;
        Ok(())
    }));
    let (hit, work) = lease_work(&db, |snap| {
        Ok(snap
            .get(DocByTitle { title: &title_a })?
            .map(|doc| doc.body.to_string()))
    });
    assert_eq!(hit, Some("body a".into()));
    // The two long titles differ only near the tail; exact confirmation
    // (never a truncated key, never a fingerprint verdict) separates them.
    let (other, _) = lease_work(&db, |snap| {
        Ok(snap
            .get(DocByTitle { title: &title_b })?
            .map(|doc| doc.body.to_string()))
    });
    assert_eq!(other, Some("body b".into()));
    let near_miss = title_a.clone() + "x";
    let (miss, _) = lease_work(&db, |snap| {
        Ok(snap
            .get(DocByTitle { title: &near_miss })?
            .map(|doc| doc.body.to_string()))
    });
    assert!(miss.is_none(), "a near-variant long title must miss");
    // Long text charges input/decode bytes, not relation-sized row visits.
    assert!(
        work < POINT_ACCESS_CEILING * 64,
        "long-text keyed access ran away: {work} work units"
    );
}

#[test]
fn snapshot_point_reads_are_isolated_from_concurrent_writes() {
    let dir = common::TempDir::new("gate-idx-snapshot-isolation");
    let db = seeded_db(&dir, 32);
    db.read(common::work(), |snap| {
        let before = snap.get(AcctById { id: AcctId(7) })?.expect("seeded");
        assert_eq!(before.note, "seeded");
        // A later write commits WHILE this lease is pinned…
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    common::expect_admitted(db.write(common::work(), |tx| {
                        tx.delete([&Acct {
                            id: AcctId(7),
                            note: "seeded",
                        }])?;
                        tx.insert([&Acct {
                            id: AcctId(7),
                            note: "rewritten",
                        }])?;
                        Ok(())
                    }));
                })
                .join()
                .expect("writer thread");
        });
        // …and this snapshot still answers its own generation, through the
        // same indexed path.
        let pinned = snap
            .get(AcctById { id: AcctId(7) })?
            .expect("still visible");
        assert_eq!(pinned.note, "seeded", "a pinned snapshot never moves");
        Ok(())
    })
    .expect("pinned lease");
    // A fresh lease sees the committed replacement through the index.
    let (after, _) = lease_work(&db, |snap| {
        Ok(snap
            .get(AcctById { id: AcctId(7) })?
            .map(|fact| fact.note.to_string()))
    });
    assert_eq!(after, Some("rewritten".into()));
}

#[test]
fn key_probe_queries_are_bucket_shaped_and_agree_with_the_scan_oracle() {
    let template = bumbledb::query!(GateIdx {
        (note) | Task(kind, subject, note), kind == ?k, subject == ?s;
    });
    let mut per_size = Vec::new();
    for (tag, size) in [("small", 64u64), ("large", 4096u64)] {
        let dir = common::TempDir::new(&format!("gate-idx-probe-{tag}"));
        let db = seeded_db(&dir, size);
        let mut prepared = db.prepare(&template).expect("prepare");
        let target = size / 2;
        let (rows, probe_work) = lease_work(&db, |snap| {
            snap.execute_collect(
                &mut prepared,
                &[BindValue::U64(target % 7), BindValue::U64(target)],
            )
        });
        assert_eq!(rows.len(), 1, "the uniqueness probe finds its one row");
        let (missing, miss_work) = lease_work(&db, |snap| {
            snap.execute_collect(
                &mut prepared,
                &[BindValue::U64((target % 7) + 1), BindValue::U64(target)],
            )
        });
        assert_eq!(missing.len(), 0, "a wrong determinant probe answers empty");
        assert!(
            probe_work < POINT_ACCESS_CEILING && miss_work < POINT_ACCESS_CEILING,
            "key probe at {size} rows must not scan: hit {probe_work}, miss {miss_work}"
        );
        per_size.push((probe_work, miss_work));
    }
    let (small_hit, small_miss) = per_size[0];
    let (large_hit, large_miss) = per_size[1];
    assert!(
        large_hit <= small_hit.saturating_mul(2) && large_miss <= small_miss.saturating_mul(2),
        "key-probe work grew with relation size: {small_hit}/{small_miss} -> {large_hit}/{large_miss}"
    );
}

/// Forced fingerprint collisions through the same public candidate protocol
/// the log bridge drives: every determinant shares one bucket, and exact
/// decoded confirmation still separates every fact (Q-COLLISION, HASH-02).
/// The forcing constructor exists only under the `collision-probe` feature;
/// run with `--features collision-probe`.
#[cfg(feature = "collision-probe")]
mod forced_collisions {
    use bumbledb::schema::judge::JudgeBudget;
    use bumbledb::schema::{
        FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor,
        StatementDescriptor, StatementId, ValidateDescriptor as _, ValueType,
    };
    use bumbledb::store::{
        CandidateJudge, CandidateState, FP_LEN, HostChanges, Judgment, MapPolicy, Prepared,
        SchemaJudge, Store, StoreResult, UnindexedRows,
    };
    use bumbledb::work::Resource;
    use bumbledb::{ChangeSet, Value, WorkContext};

    const USER: RelationId = RelationId(0);
    const EMAIL_KEY: StatementId = StatementId(0);

    fn schema() -> bumbledb::Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "User".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "email".into(),
                        value_type: ValueType::String,
                    },
                ],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: USER,
                projection: Box::from([FieldId(1)]),
            }],
        }
        .validate()
        .expect("schema validates")
    }

    fn work() -> WorkContext {
        bumbledb::ExecutionPolicy {
            input_bytes: 1 << 30,
            working_bytes: 1 << 30,
            scratch_bytes: 1 << 30,
            result_bytes: 1 << 30,
            rows: 1 << 24,
            work_units: 1 << 40,
            timeout: std::time::Duration::from_secs(120),
        }
        .start()
        .expect("work context")
    }

    fn changes(schema: &bumbledb::Schema, adds: &[(u64, &str)]) -> ChangeSet {
        let mut builder = ChangeSet::builder(schema, work());
        for (id, email) in adds {
            builder
                .insert(USER, &[Value::U64(*id), Value::String((*email).into())])
                .expect("stage insert");
        }
        builder.finish().expect("sealed changes")
    }

    const NO_HOST: HostChanges<'static> = HostChanges {
        records: &[],
        attachment: bumbledb::store::AttachmentChange::Keep,
    };

    struct CaptureCompetitors {
        email: &'static str,
        seen: std::cell::RefCell<Vec<u64>>,
        enumeration_work: std::cell::Cell<u64>,
    }

    impl CandidateJudge for CaptureCompetitors {
        type Rejection = std::convert::Infallible;

        fn judge(
            &self,
            candidate: &CandidateState<'_, '_>,
            work: &WorkContext,
        ) -> StoreResult<Judgment<Self::Rejection>> {
            let before = work.used(Resource::WorkUnits);
            let mut seen = Vec::new();
            candidate
                .visit_determinant_competitors(
                    EMAIL_KEY,
                    &[Value::String(self.email.into())],
                    work,
                    &mut |_, values| {
                        match values[0] {
                            Value::U64(id) => seen.push(id),
                            ref other => panic!("user rows lead with u64 ids, saw {other:?}"),
                        }
                        Ok(true)
                    },
                )?
                .expect("the email key is sealed in this schema");
            self.enumeration_work
                .set(work.used(Resource::WorkUnits) - before);
            *self.seen.borrow_mut() = seen;
            Ok(Judgment::Admitted)
        }
    }

    #[test]
    fn forced_collisions_never_merge_facts_and_judgment_still_cites_true_competitors() {
        let dir = super::common::TempDir::new("gate-idx-forced-collision");
        let path = dir.path().join("store");
        std::fs::create_dir_all(dir.path()).expect("parent dir");
        let schema = schema();
        let store =
            Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0xEE; FP_LEN])
                .expect("forced-collision store")
                .0;

        // Distinct emails admit under total collisions: the fingerprint
        // never merges facts.
        let judge = SchemaJudge {
            schema: &schema,
            budget: JudgeBudget::default(),
        };
        let context = work();
        {
            let mut owner = store.writer(&context).expect("writer");
            match owner
                .prepare(
                    &changes(&schema, &[(1, "a@x"), (2, "b@x"), (3, "c@x")]),
                    &UnindexedRows,
                    &judge,
                )
                .expect("prepare")
            {
                Prepared::Admitted(prepared) => {
                    prepared
                        .seal(NO_HOST)
                        .expect("seal")
                        .commit()
                        .expect("commit");
                }
                Prepared::Rejected(violations) => {
                    panic!("distinct emails rejected: {violations:?}")
                }
            }
        }

        // A competing duplicate rejects, citing BOTH true competitors —
        // never a collision cohabitant.
        {
            let mut owner = store.writer(&context).expect("writer");
            match owner
                .prepare(&changes(&schema, &[(9, "b@x")]), &UnindexedRows, &judge)
                .expect("prepare")
            {
                Prepared::Admitted(_) => panic!("a duplicate email must reject"),
                Prepared::Rejected(violations) => {
                    assert_eq!(violations.len(), 1);
                    assert_eq!(violations[0].statement, EMAIL_KEY);
                    assert_eq!(
                        violations[0].examples.len(),
                        2,
                        "both competing rows are cited"
                    );
                }
            }
        }

        // Indexed competitor enumeration under total collisions: the
        // bucket holds everything, exact confirmation returns exactly the
        // one true row.
        let capture = CaptureCompetitors {
            email: "b@x",
            seen: std::cell::RefCell::new(Vec::new()),
            enumeration_work: std::cell::Cell::new(0),
        };
        {
            let mut owner = store.writer(&context).expect("writer");
            match owner
                .prepare(
                    &changes(&schema, &[(10, "unrelated@x")]),
                    &UnindexedRows,
                    &capture,
                )
                .expect("prepare")
            {
                Prepared::Admitted(prepared) => prepared.abort(),
                Prepared::Rejected(never) => match never {},
            }
        }
        assert_eq!(
            *capture.seen.borrow(),
            vec![2],
            "collision cohabitants are excluded by exact decoded equality"
        );
    }
}
