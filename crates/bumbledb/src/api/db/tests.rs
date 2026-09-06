//! Authored embedding-surface tests over the successor store (F1: written,
//! executed only in F3).
//!
//! Gate mapping (chapter 70 / audit 50):
//! - typed/dyn/accepted lane equality, reports, walls → E-DELTA, E-CODEC
//!   consumers, SDK substrate; the three lanes share one shape judgment.
//! - own-writes/committed fall-through point reads → E-VISIBILITY (api
//!   half), G06 candidate children.
//! - key-conflict rejection with both competing rows cited → ENG-005 /
//!   E-ADMIT through the full public path (the historical shared-key
//!   counterexample, preserved across the fresh-mechanism deletion).
//! - closed-relation refusals and sealed-extension reads → E-VALUE.
//! - witness lifecycle (clone/stale/foreign) → CONC substrate, SDK-009.
//! - deleted text unreachable after delete + reopen → ENG-006 (E-TEXT api
//!   remainder; no dictionary exists to leak).
//! - no `*_nosync` constructor exists (ENG-008) — structural: the surface
//!   has no such symbol; E-DURABILITY execution lives in the store tests.
//! - generation moves only on change → E-SNAPSHOT/G06 remainder.
//!
//! The fresh/reserve mechanism tests of the transitional surface are
//! deleted WITH the mechanism (ENG-004/E-NO-RESERVE): `reserve`,
//! `FreshRange`, `fresh_field` and `DynIdError::NotAFreshField` no longer
//! exist to test. Their safety intent — an aborted write leaks no issued
//! authority — is unrepresentable now: identities are application values.

use bumbledb_theory::schema::{
    FieldDescriptor, RelationId, Row, SchemaDescriptor, StatementDescriptor, StatementId, ValueType,
};

use crate::error::{Admission, Error, Result, Violation};
use crate::ir::Value;
use crate::schema::ValidateDescriptor as _;
use crate::storage::store::StoreError;
use crate::testutil::{TempDir, expect_rejected};
use crate::work::{ExecutionPolicy, Resource};
use crate::{ChangeSet, Db, InstanceBuilder, WorkContext};

use super::row_reader::RowReader;
use super::{Fact, Key};

// --- Test theory: one keyed relation, hand-built (the exact Fact/Key
// roster the schema! macro targets — P07's C-contract witness). ---

const ENTRY: RelationId = RelationId(0);
const ENTRY_NAME_KEY: StatementId = StatementId(0);

#[derive(Clone, Copy)]
struct Ledger;

impl crate::Theory for Ledger {
    fn descriptor(self) -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                name: "entry".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                    },
                ],
                extension: None,
            }],
            statements: vec![StatementDescriptor::Functionality {
                relation: ENTRY,
                projection: Box::new([bumbledb_theory::schema::FieldId(0)]),
            }],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Entry<'a> {
    name: &'a str,
    amount: i64,
}

impl<'a> Fact<'a> for Entry<'a> {
    type Schema = Ledger;

    const RELATION: RelationId = ENTRY;

    fn append_values(&self, out: &mut Vec<Value>) -> Result<()> {
        out.push(Value::String(self.name.into()));
        out.push(Value::I64(self.amount));
        Ok(())
    }

    fn decode(mut row: RowReader<'a>) -> Result<Self> {
        let name = row.next_str()?;
        let amount = row.next_i64()?;
        row.finish()?;
        Ok(Self { name, amount })
    }
}

struct EntryName<'a>(&'a str);

impl<'a> Key<'a> for EntryName<'a> {
    type Schema = Ledger;
    type Fact = Entry<'a>;

    const STATEMENT: StatementId = ENTRY_NAME_KEY;

    fn append_key_values(&self, out: &mut Vec<Value>) -> Result<()> {
        out.push(Value::String(self.0.into()));
        Ok(())
    }
}

fn entry_row(name: &str, amount: i64) -> Vec<Value> {
    vec![Value::String(name.into()), Value::I64(amount)]
}

fn operation() -> WorkContext {
    super::test_operation().expect("work")
}

fn create(dir: &TempDir) -> Db<Ledger> {
    Db::create(dir.path(), Ledger, operation())
        .expect("create")
        .expect("empty theory admits")
}

#[test]
fn pending_canonical_owners_refund_duplicates_and_cancellation_and_transfer_into_seal() {
    let dir = TempDir::new("pending-owner-seal");
    let db = create(&dir);
    let work = operation();
    let parent = db.store.snapshot(&work).unwrap();
    let mut tx: super::WriteTx<'_, Ledger> =
        super::WriteTx::new(&db.schema, &db.closed, &parent, &work);
    let baseline = work.used(Resource::WorkingBytes);
    let fact = Entry {
        name: "alpha",
        amount: 3,
    };
    tx.insert([&fact]).unwrap();
    let retained = work.used(Resource::WorkingBytes);
    assert!(retained > baseline);
    tx.insert_dyn(ENTRY, [entry_row("alpha", 3)]).unwrap();
    assert_eq!(
        work.used(Resource::WorkingBytes),
        retained,
        "duplicate's temporary owner drops"
    );
    assert!(tx.contains_dyn(ENTRY, &entry_row("alpha", 3)).unwrap());
    assert_eq!(
        work.used(Resource::WorkingBytes),
        retained,
        "borrowed lookup retains no copied keys"
    );
    let accepted = crate::AcceptedCollection::from_value_rows(
        ENTRY,
        db.schema.relation(ENTRY).fields(),
        [entry_row("alpha", 3)],
    )
    .unwrap();
    tx.delete_accepted(&accepted).unwrap();
    assert_eq!(
        work.used(Resource::WorkingBytes),
        baseline,
        "opposite mutation drops both rows and empty trees"
    );
    tx.delete_dyn(ENTRY, [entry_row("missing", 0)]).unwrap();
    assert_eq!(
        work.used(Resource::WorkingBytes),
        baseline,
        "parent no-op retains nothing"
    );
    tx.insert_dyn(ENTRY, [entry_row("alpha", 3), entry_row("beta", 4)])
        .unwrap();
    let retained = work.used(Resource::WorkingBytes);
    tx.delete_dyn(ENTRY, [entry_row("absent", 0)]).unwrap();
    assert_eq!(
        work.used(Resource::WorkingBytes),
        retained,
        "parent no-op refunds an incoming payload adopted by an existing relation"
    );
    let pending = tx.into_pending();
    assert_eq!(
        work.used(Resource::WorkingBytes),
        retained,
        "ownership survives the transaction"
    );
    let changes = pending.seal(&db.schema, &work).unwrap();
    assert_eq!(changes.len(), 2);
    assert!(work.used(Resource::WorkingBytes) > baseline);
    assert!(
        work.used(Resource::WorkingBytes) < retained,
        "only sealed payload remains, not staging trees"
    );
    let clone = changes.clone();
    drop(changes);
    assert!(work.used(Resource::WorkingBytes) > baseline);
    drop(clone);
    assert_eq!(work.used(Resource::WorkingBytes), baseline);
}

#[test]
fn pending_tree_occupancy_charge_grows_and_shrinks_at_five_key_thresholds() {
    let dir = TempDir::new("pending-occupancy");
    let db = create(&dir);
    let work = operation();
    let parent = db.store.snapshot(&work).unwrap();
    let mut tx: super::WriteTx<'_, Ledger> =
        super::WriteTx::new(&db.schema, &db.closed, &parent, &work);
    let sample = crate::canonical::CanonicalRow::encode(
        db.schema.relation(ENTRY).fields(),
        &entry_row("000", 0),
        &work,
    )
    .unwrap();
    let payload = sample.len() as u64;
    drop(sample);
    let baseline = work.used(Resource::WorkingBytes);
    let mut used = vec![baseline];
    for id in 0..16 {
        tx.insert_dyn(ENTRY, [entry_row(&format!("{id:03}"), id)])
            .unwrap();
        used.push(work.used(Resource::WorkingBytes));
    }
    let node_bytes = used[6] - used[5] - payload;
    assert!(node_bytes > 0);
    for n in 1usize..=16 {
        assert_eq!(
            used[n],
            used[1] + (n as u64 - 1) * payload + ((n as u64 - 1) / 5) * node_bytes
        );
    }
    for id in (0..16).rev() {
        tx.delete_dyn(ENTRY, [entry_row(&format!("{id:03}"), id)])
            .unwrap();
        assert_eq!(
            work.used(Resource::WorkingBytes),
            used[usize::try_from(id).unwrap()],
            "delete/rebalance refunds the matching occupancy bound"
        );
    }
}

#[test]
fn pending_budget_refusal_keeps_prefix_charged_and_poisoned_until_drop() {
    let dir = TempDir::new("pending-budget");
    let db = create(&dir);
    let work = operation();
    let parent = db.store.snapshot(&work).unwrap();
    let mut tx: super::WriteTx<'_, Ledger> =
        super::WriteTx::new(&db.schema, &db.closed, &parent, &work);
    tx.insert_dyn(ENTRY, (0..5).map(|id| entry_row(&format!("{id:03}"), id)))
        .unwrap();
    let prefix = work.used(Resource::WorkingBytes);
    // Measure the sixth row's actual ownership delta, rather than assuming
    // a particular key/guard layout or allocator-dependent node size.
    tx.insert_dyn(ENTRY, [entry_row("005", 5)]).unwrap();
    let sixth_delta = work.used(Resource::WorkingBytes) - prefix;
    tx.delete_dyn(ENTRY, [entry_row("005", 5)]).unwrap();
    assert_eq!(work.used(Resource::WorkingBytes), prefix);
    let sample = crate::canonical::CanonicalRow::encode(
        db.schema.relation(ENTRY).fields(),
        &entry_row("005", 5),
        &work,
    )
    .unwrap();
    let collection_scratch =
        sample.len() as u64 + 2 * std::mem::size_of::<crate::canonical::CanonicalRow>() as u64;
    drop(sample);
    let remaining = sixth_delta - 1;
    assert!(
        collection_scratch <= remaining,
        "encoding fits; the refusal must happen when the pending tree grows"
    );
    let held = work
        .reserve(
            crate::work::ByteKind::Working,
            work.limit(Resource::WorkingBytes) - prefix - remaining,
        )
        .unwrap();
    let before = work.used(Resource::WorkingBytes);
    assert!(
        tx.insert_dyn(ENTRY, [entry_row("005", 5)]).is_err(),
        "the next tree occupancy envelope does not fit"
    );
    assert!(tx.poisoned().is_some());
    assert_eq!(
        work.used(Resource::WorkingBytes),
        before,
        "failed row and collection scratch are released; prior owner remains"
    );
    drop(tx);
    assert_eq!(work.used(Resource::WorkingBytes), held.bytes());
    drop(held);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
}

#[test]
fn pending_collection_and_seal_refusals_release_all_owned_memory() {
    let dir = TempDir::new("pending-refusals");
    let db = create(&dir);
    for seal in [false, true] {
        let work = operation();
        let parent = db.store.snapshot(&work).unwrap();
        let mut tx: super::WriteTx<'_, Ledger> =
            super::WriteTx::new(&db.schema, &db.closed, &parent, &work);
        if seal {
            tx.insert_dyn(ENTRY, [entry_row("kept", 1)]).unwrap();
        }
        let held = work
            .reserve(
                crate::work::ByteKind::Working,
                work.limit(Resource::WorkingBytes)
                    - work.used(Resource::WorkingBytes)
                    - if seal { 0 } else { 512 },
            )
            .unwrap();
        if seal {
            assert!(tx.into_pending().seal(&db.schema, &work).is_err());
        } else {
            assert!(
                tx.insert_dyn(ENTRY, (0..64).map(|id| entry_row(&format!("{id:03}"), id)))
                    .is_err()
            );
            assert!(
                tx.poisoned().is_none(),
                "parse-all-first collection failure applies no prefix"
            );
            drop(tx);
        }
        assert_eq!(work.used(Resource::WorkingBytes), held.bytes());
        drop(held);
        assert_eq!(work.used(Resource::WorkingBytes), 0);
    }
}

// --- The three write lanes produce identical stores. ---

#[test]
fn the_three_write_lanes_produce_identical_stores() {
    let typed_dir = TempDir::new("db-lane-typed");
    let dyn_dir = TempDir::new("db-lane-dyn");
    let accepted_dir = TempDir::new("db-lane-accepted");
    let typed = create(&typed_dir);
    let dynamic = create(&dyn_dir);
    let accepted = create(&accepted_dir);
    let typed_work = operation();
    let dynamic_work = operation();
    let accepted_work = operation();

    let facts = [
        Entry {
            name: "alpha",
            amount: 3,
        },
        Entry {
            name: "beta",
            amount: -7,
        },
    ];
    typed
        .write(typed_work.clone(), |tx| {
            let report = tx.insert(facts.iter())?;
            assert_eq!(report.submitted(), 2);
            assert_eq!(report.changed(), 2);
            Ok(())
        })
        .expect("write")
        .unwrap();

    dynamic
        .write(dynamic_work.clone(), |tx| {
            tx.insert_dyn(ENTRY, [entry_row("alpha", 3), entry_row("beta", -7)])
                .map(|_| ())
        })
        .expect("write")
        .unwrap();

    let schema = accepted.schema();
    let collection = crate::AcceptedCollection::from_value_rows(
        ENTRY,
        schema.relation(ENTRY).fields(),
        [entry_row("alpha", 3), entry_row("beta", -7)],
    )
    .expect("shape proof");
    accepted
        .write(accepted_work.clone(), |tx| {
            tx.insert_accepted(&collection).map(|_| ())
        })
        .expect("write")
        .unwrap();

    for resource in [Resource::Rows, Resource::InputBytes, Resource::WorkUnits] {
        assert_eq!(typed_work.used(resource), dynamic_work.used(resource));
        assert_eq!(typed_work.used(resource), accepted_work.used(resource));
    }

    let digest = typed.catalog_digest(operation()).expect("digest");
    assert_eq!(digest, dynamic.catalog_digest(operation()).expect("digest"));
    assert_eq!(
        digest,
        accepted.catalog_digest(operation()).expect("digest")
    );
}

#[test]
fn accepted_collection_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<crate::AcceptedCollection>();
}

// --- Reports, no-ops, and the ENG-006 remainder. ---

#[test]
fn a_typo_delete_is_a_counted_noop_and_moves_nothing() {
    let dir = TempDir::new("db-typo-delete");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "kept",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    let before = db.generation(operation()).expect("generation");
    db.write(operation(), |tx| {
        let report = tx.delete([&Entry {
            name: "absent",
            amount: 9,
        }])?;
        assert_eq!(report.submitted(), 1);
        assert_eq!(report.changed(), 0);
        Ok(())
    })
    .expect("write")
    .unwrap();
    // A no-op command does not move the generation (G06 remainder).
    assert_eq!(db.generation(operation()).expect("generation"), before);
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        1,
        "the typo delete deleted nothing"
    );
}

#[test]
fn deleted_text_is_unreachable_after_delete_and_reopen() {
    let dir = TempDir::new("db-text-gone");
    {
        let db = create(&dir);
        db.write(operation(), |tx| {
            tx.insert([
                &Entry {
                    name: "resident",
                    amount: 1,
                },
                &Entry {
                    name: "ephemeral",
                    amount: 2,
                },
            ])
            .map(|_| ())
        })
        .expect("write")
        .unwrap();
        db.write(operation(), |tx| {
            tx.delete([&Entry {
                name: "ephemeral",
                amount: 2,
            }])
            .map(|_| ())
        })
        .expect("write")
        .unwrap();
        assert!(
            !db.read(operation(), |snap| snap.contains(&Entry {
                name: "ephemeral",
                amount: 2
            }))
            .expect("contains")
        );
    }
    // Reopen: canonical rows own their text inline; the deleted tuple left
    // no independently live text entry anywhere (ENG-006: no dictionary
    // namespace even exists in the store).
    let db = Db::open(dir.path(), Ledger, operation()).expect("open");
    let rows: Vec<crate::canonical::DecodedRow> = db
        .read(operation(), |snap| snap.scan(ENTRY)?.collect::<Result<_>>())
        .expect("scan");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].values(), entry_row("resident", 1));
}

// --- Point reads: own writes, committed fall-through, typed errors. ---

#[test]
fn get_dyn_reads_its_own_writes_exactly_as_a_later_transaction_does() {
    let dir = TempDir::new("db-own-writes");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert_dyn(ENTRY, [entry_row("alpha", 3)])?;
        // The write's own final-state view answers like a later reader.
        let row = tx
            .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("alpha".into())])?
            .expect("own write visible");
        assert_eq!(row.values(), entry_row("alpha", 3));
        // Replacement inside the same command: delete + insert, either order.
        tx.delete_dyn(ENTRY, [entry_row("alpha", 3)])?;
        tx.insert_dyn(ENTRY, [entry_row("alpha", 5)])?;
        let row = tx
            .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("alpha".into())])?
            .expect("replacement visible");
        assert_eq!(row.values(), entry_row("alpha", 5));
        assert!(tx.contains_dyn(ENTRY, &entry_row("alpha", 5))?);
        assert!(!tx.contains_dyn(ENTRY, &entry_row("alpha", 3))?);
        Ok(())
    })
    .expect("write")
    .unwrap();
    let row = db
        .read(operation(), |snap| {
            snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("alpha".into())])
        })
        .expect("read")
        .expect("committed");
    assert_eq!(row.values(), entry_row("alpha", 5));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one snapshot-borrow lifetime across collision lookup, replacement, and reader reuse"
)]
fn keyed_reads_retain_snapshot_bytes_across_collisions_repeated_reads_and_replacement() {
    use crate::Theory as _;
    use crate::schema::FieldId;
    use crate::storage::store::{MapPolicy, Store};

    for forced_collision in [false, true] {
        let dir = TempDir::new("db-get-borrowed-snapshot");
        let schema = Ledger.descriptor().validate().expect("schema");
        let store = if forced_collision {
            Store::create_forced_fingerprint(dir.path(), &schema, MapPolicy::default(), [0xCC; 16])
                .expect("forced store")
        } else {
            Store::create(dir.path(), &schema, MapPolicy::default())
                .expect("store")
                .0
        };
        let db = Db::<Ledger>::assemble(store, schema, operation()).expect("db");
        db.write(operation(), |tx| {
            tx.insert_dyn(
                ENTRY,
                [
                    entry_row("alpha", 1),
                    entry_row("beta", 2),
                    entry_row("gamma", 3),
                ],
            )?;
            Ok(())
        })
        .expect("write")
        .unwrap();
        let work = operation();
        let pin = db.owned_read().expect("old pin");
        let snapshot = pin.snapshot();
        let key = snapshot
            .determinants()
            .key_for(ENTRY, &[FieldId(0)])
            .expect("name key");
        let mut borrowed = Vec::new();
        {
            let projected = crate::storage::store::det_index::determinant_bytes(
                key,
                &[Value::String("beta".into())],
                &work,
            )
            .expect("projection");
            snapshot
                .visit_projection(key.id, &projected, &work, &mut |_id, bytes| {
                    borrowed.push(bytes);
                    Ok(true)
                })
                .expect("retain borrowed rows beyond the cursor and routing buffer");
        }
        assert_eq!(borrowed.len(), if forced_collision { 3 } else { 1 });
        let selected = super::get::find_snapshot_row(
            snapshot,
            db.schema(),
            ENTRY,
            &[FieldId(0)],
            &[Value::String("beta".into())],
            &work,
        )
        .expect("get borrowed bytes")
        .expect("beta");
        assert!(borrowed.iter().any(|bytes| std::ptr::eq(*bytes, selected)));
        let old = pin
            .get(EntryName("beta"), &work)
            .expect("typed get")
            .expect("beta");
        for name in ["gamma", "absent", "alpha", "beta"] {
            let found = pin
                .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String(name.into())], &work)
                .expect("repeat dynamic get");
            assert_eq!(found.is_some(), name != "absent");
        }
        db.write(operation(), |tx| {
            tx.delete_dyn(ENTRY, [entry_row("beta", 2)])?;
            tx.insert_dyn(ENTRY, [entry_row("beta", 22)])?;
            Ok(())
        })
        .expect("replace")
        .unwrap();
        assert_eq!((old.name, old.amount), ("beta", 2));
        assert_eq!(
            crate::canonical::decode(db.schema().relation(ENTRY).fields(), selected, &work)
                .expect("retained canonical bytes")
                .values(),
            entry_row("beta", 2)
        );
        assert_eq!(
            pin.get(EntryName("beta"), &work)
                .expect("old snapshot get")
                .map(|entry| entry.amount),
            Some(2)
        );
        drop(pin);
        for _ in 0..2 {
            let latest = db.owned_read().expect("fresh or reused reader");
            assert_eq!(
                latest
                    .get(EntryName("beta"), &work)
                    .expect("latest get")
                    .map(|entry| entry.amount),
                Some(22)
            );
        }
    }
}

#[test]
fn selected_free_join_images_confirm_composite_fingerprint_collisions() {
    use crate::ir::{Atom, AtomSource, FindTerm, ParamId, Query, Rule, Term, VarId};
    use crate::schema::{FieldId, RelationDescriptor, Side, StatementDescriptor};
    use crate::storage::store::{MapPolicy, Store};

    let fields = [FieldId(0), FieldId(1), FieldId(2)];
    let side = |relation| Side {
        relation,
        projection: fields.into(),
        selection: Box::new([]),
    };
    let schema = SchemaDescriptor {
        relations: [("R", 4), ("Key", 3)]
            .into_iter()
            .map(|(name, arity)| RelationDescriptor {
                name: name.into(),
                extension: None,
                fields: (0..arity)
                    .map(|i| FieldDescriptor {
                        name: format!("f{i}").into(),
                        value_type: ValueType::U64,
                    })
                    .collect(),
            })
            .collect(),
        // Containment installs a nonunique reverse projection. Three
        // u64 coordinates force fingerprint routing, not ExactBounded.
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: fields.into(),
            },
            StatementDescriptor::Containment {
                source: side(RelationId(0)),
                target: side(RelationId(1)),
            },
        ],
    }
    .validate()
    .unwrap();
    let dir = TempDir::new("selected-image-collision");
    let store =
        Store::create_forced_fingerprint(dir.path(), &schema, MapPolicy::default(), [0xA5; 16])
            .unwrap();
    let db = Db::<()>::assemble(store, schema, operation()).unwrap();
    db.write(operation(), |tx| {
        tx.insert_dyn(
            RelationId(1),
            (0..4).map(|i| vec![Value::U64(i), Value::U64(9), Value::U64(u64::MAX)]),
        )?;
        tx.insert_dyn(
            RelationId(0),
            (0..32).map(|i| {
                vec![
                    Value::U64(i % 4),
                    Value::U64(9),
                    Value::U64(u64::MAX),
                    Value::U64(i),
                ]
            }),
        )?;
        Ok(())
    })
    .unwrap()
    .unwrap();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Literal(Value::U64(9))),
                (FieldId(2), Term::Literal(Value::U64(u64::MAX))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = db.prepare(&query, operation()).unwrap();
    for key in [0, 3, 4, 1, 0] {
        let answers = db
            .read(operation(), |snap| {
                snap.execute_collect(&mut prepared, &[crate::BindValue::U64(key)])
            })
            .unwrap();
        let mut actual: Vec<_> = (0..answers.len())
            .map(|i| {
                let crate::AnswerValue::U64(value) = answers.get(i, 0) else {
                    panic!("u64")
                };
                value
            })
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, (0..32).filter(|i| i % 4 == key).collect::<Vec<_>>());
    }
}

#[test]
fn get_with_work_charges_the_supplied_operation_budget() {
    let dir = TempDir::new("db-get-with-work");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert_dyn(ENTRY, [entry_row("budget", 1)]).map(|_| ())
    })
    .expect("write")
    .unwrap();
    let work = ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 10_000,
        work_units: 10_000,
        timeout: std::time::Duration::from_secs(60),
    }
    .start()
    .expect("policy");
    let hit = db
        .read(work, |snap| {
            let before = snap.work().used(Resource::WorkUnits);
            let hit = snap.get_dyn_with_work(
                ENTRY,
                ENTRY_NAME_KEY,
                &[Value::String("budget".into())],
                snap.work(),
            )?;
            assert!(snap.work().used(Resource::WorkUnits) > before);
            Ok(hit)
        })
        .expect("read");
    assert!(hit.is_some());
}

#[test]
fn get_dyn_falls_through_to_committed_state() {
    let dir = TempDir::new("db-fall-through");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert_dyn(ENTRY, [entry_row("base", 10)]).map(|_| ())
    })
    .expect("write")
    .unwrap();
    db.write(operation(), |tx| {
        // Nothing pending for "base": the read falls through to committed.
        let row = tx
            .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("base".into())])?
            .expect("committed row visible");
        assert_eq!(row.values(), entry_row("base", 10));
        // A pending delete hides the committed row from this view.
        tx.delete_dyn(ENTRY, [entry_row("base", 10)])?;
        assert!(
            tx.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("base".into())])?
                .is_none()
        );
        // Abort by error: nothing changed durably.
        Err::<(), _>(Error::ForeignWitness)
    })
    .expect_err("deliberate abort");
    assert!(
        db.read(operation(), |snap| snap
            .contains_dyn(ENTRY, &entry_row("base", 10)))
            .expect("read"),
        "the aborted delete never reached storage"
    );
}

#[test]
fn repeated_pending_removal_uses_known_parent_presence() {
    let dir = TempDir::new("db-pending-parent-presence");
    let db = create(&dir);
    let row = entry_row("existing", 10);
    db.write(operation(), |tx| tx.insert_dyn(ENTRY, [&row]))
        .unwrap()
        .unwrap();
    let ctx = operation();
    db.write(ctx.clone(), |tx| {
        let before = ctx.used(Resource::WorkUnits);
        assert_eq!(tx.delete_dyn(ENTRY, [&row])?.changed(), 1);
        let after_first = ctx.used(Resource::WorkUnits);
        assert_eq!(tx.delete_dyn(ENTRY, [&row])?.changed(), 0);
        let after_second = ctx.used(Resource::WorkUnits);
        assert!(
            after_second - after_first < after_first - before,
            "an occupied pending row must not re-probe its committed parent"
        );
        assert_eq!(tx.insert_dyn(ENTRY, [&row])?.changed(), 1);
        assert!(tx.contains_dyn(ENTRY, &row)?);
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert!(
        db.read(operation(), |snap| snap.contains_dyn(ENTRY, &row))
            .unwrap()
    );
}

#[test]
fn get_dyn_rejects_mis_shaped_requests_with_typed_errors() {
    let dir = TempDir::new("db-mis-shaped");
    let db = create(&dir);
    db.read(operation(), |snap| {
        // Unknown relation.
        let unknown = RelationId(77);
        assert!(matches!(
            snap.get_dyn(unknown, ENTRY_NAME_KEY, &[Value::U64(1)]),
            Err(Error::FactShape(crate::error::FactShapeError::Id(
                crate::error::DynIdError::UnknownRelation { relation }
            ))) if relation == unknown
        ));
        // A statement id that is not a key statement of this relation.
        assert!(matches!(
            snap.get_dyn(ENTRY, StatementId(9), &[Value::U64(1)]),
            Err(Error::FactShape(crate::error::FactShapeError::Id(
                crate::error::DynIdError::NotAKeyStatement { .. }
            )))
        ));
        // Arity mismatch of the key tuple.
        assert!(matches!(
            snap.get_dyn(
                ENTRY,
                ENTRY_NAME_KEY,
                &[Value::String("a".into()), Value::I64(1)]
            ),
            Err(Error::FactShape(
                crate::error::FactShapeError::ArityMismatch { .. }
            ))
        ));
        // Type mismatch of the key value.
        assert!(matches!(
            snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::U64(1)]),
            Err(Error::FactShape(
                crate::error::FactShapeError::TypeMismatch { .. }
            ))
        ));
        Ok(())
    })
    .expect("read");
}

#[test]
fn typed_get_borrows_decoded_text_from_the_lease() {
    let dir = TempDir::new("db-typed-get");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "gamma",
            amount: 4,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    db.read(operation(), |snap| {
        let fact = snap.get(EntryName("gamma"))?.expect("present");
        assert_eq!(
            fact,
            Entry {
                name: "gamma",
                amount: 4
            }
        );
        assert!(snap.get(EntryName("missing"))?.is_none());
        let scanned: Vec<Entry<'_>> = snap.scan_facts::<Entry<'_>>()?.collect::<Result<_>>()?;
        assert_eq!(
            scanned,
            vec![Entry {
                name: "gamma",
                amount: 4
            }]
        );
        Ok(())
    })
    .expect("read");
}

// --- The historical shared-key counterexample, full public path. ---

#[test]
fn a_key_conflict_is_rejected_with_both_competing_rows_cited() {
    let dir = TempDir::new("db-key-conflict");
    let db = create(&dir);
    let violations = expect_rejected(db.write(operation(), |tx| {
        tx.insert([
            &Entry {
                name: "shared",
                amount: 1,
            },
            &Entry {
                name: "shared",
                amount: 2,
            },
        ])
        .map(|_| ())
    }));
    assert_eq!(violations.len(), 1, "one violated statement");
    let violation = violations.get(0).expect("violation");
    assert!(matches!(violation, Violation::Functionality { .. }));
    assert_eq!(
        violation.statement_id(db.schema()),
        ENTRY_NAME_KEY,
        "the stable materialized statement id is cited"
    );
    // Both competing proposals are evidence (ENG-005: the judge saw the
    // whole final state, not an install failure).
    let cited = violations.cited_facts(0);
    assert!(
        cited.len() >= 2,
        "both conflicting rows cited, got {cited:?}"
    );
    // The losing candidate is invisible: nothing was committed.
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        0
    );
    assert_eq!(db.generation(operation()).expect("generation").value(), 0);
}

#[test]
fn a_conflict_with_a_committed_row_rejects_and_preserves_it() {
    let dir = TempDir::new("db-committed-conflict");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "holder",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    let violations = expect_rejected(db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "holder",
            amount: 2,
        }])
        .map(|_| ())
    }));
    assert_eq!(violations.len(), 1);
    let row = db
        .read(operation(), |snap| {
            snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("holder".into())])
        })
        .expect("read")
        .expect("incumbent survives");
    assert_eq!(row.values(), entry_row("holder", 1));
}

#[test]
fn replacement_in_one_command_is_judged_as_final_state() {
    let dir = TempDir::new("db-replacement");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "acct",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    // delete(old) + insert(new) with one key: the final state holds one
    // row, so the key law admits — no transient-order refusal exists.
    db.write(operation(), |tx| {
        tx.delete([&Entry {
            name: "acct",
            amount: 1,
        }])?;
        tx.insert([&Entry {
            name: "acct",
            amount: 2,
        }])?;
        Ok(())
    })
    .expect("write")
    .unwrap();
    let row = db
        .read(operation(), |snap| {
            snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("acct".into())])
        })
        .expect("read")
        .expect("replaced");
    assert_eq!(row.values(), entry_row("acct", 2));
}

#[test]
fn apply_and_owned_snapshot_are_the_public_path() {
    let dir = TempDir::new("db-apply-snap");
    let db = create(&dir);
    let work = operation();
    let mut builder = ChangeSet::builder(db.schema(), work.clone());
    builder
        .insert(ENTRY, &entry_row("snap", 1))
        .expect("insert");
    let changes = builder.finish().expect("seal");
    match db
        .apply(&changes, super::ApplyExpected::Any, &work)
        .expect("apply")
    {
        super::ApplyOutcome::Accepted { .. } => {}
        _ => panic!("expected Accepted"),
    }
    let pin = db.snapshot(&work).expect("pin");
    assert_eq!(pin.count(ENTRY).expect("count"), 1);
    assert_eq!(
        pin.generation().value(),
        db.generation(work.clone()).expect("gen").value()
    );
    let frame = pin.frame(&work);
    assert!(
        frame
            .contains_dyn(ENTRY, &entry_row("snap", 1))
            .expect("contains")
    );
    let witness = pin.witness();
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .expect("empty");
    match db
        .apply(&empty, super::ApplyExpected::Exact(witness), &work)
        .expect("no-change")
    {
        super::ApplyOutcome::NoChange { .. } => {}
        super::ApplyOutcome::InvariantRejected { .. } => {
            panic!("empty delta under a lawful parent is not invariant rejection")
        }
        _ => panic!("expected NoChange"),
    }
}

// --- Witnesses. ---

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "the bare `witness` method path defeats the read closure's HRTB inference"
)]
fn write_from_borrows_a_cloneable_witness() {
    let dir = TempDir::new("db-witness");
    let db = create(&dir);
    let witness = db
        .read(operation(), |snap| snap.witness())
        .expect("witness");
    let again = witness.clone();
    let outcome = db
        .write_from(operation(), &witness, |tx| {
            tx.insert([&Entry {
                name: "w",
                amount: 1,
            }])
            .map(|_| ())
        })
        .expect("write");
    assert!(matches!(outcome, crate::ConditionalWrite::Accepted(_)));
    // The clone is now stale: the compare answers Moved, not an error.
    let moved = db
        .write_from(operation(), &again, |tx| {
            tx.insert([&Entry {
                name: "x",
                amount: 2,
            }])
            .map(|_| ())
        })
        .expect("write");
    assert!(matches!(moved, crate::ConditionalWrite::Moved { .. }));
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        1
    );
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "the bare `witness` method path defeats the read closure's HRTB inference"
)]
fn write_from_rejects_a_foreign_witness() {
    let a_dir = TempDir::new("db-foreign-witness-a");
    let b_dir = TempDir::new("db-foreign-witness-b");
    let a = create(&a_dir);
    let b = create(&b_dir);
    let foreign = b.read(operation(), |snap| snap.witness()).expect("witness");
    let err = a
        .write_from(operation(), &foreign, |tx| {
            tx.insert([&Entry {
                name: "n",
                amount: 1,
            }])
            .map(|_| ())
        })
        .expect_err("foreign witness refused");
    assert!(matches!(err, Error::ForeignWitness), "{err:?}");
    assert_eq!(a.generation(operation()).expect("generation").value(), 0);
}

// --- Poisoning and refusal boundaries. ---

#[test]
fn a_shape_failure_does_not_poison_a_clean_write() {
    let dir = TempDir::new("db-clean-shape-fail");
    let db = create(&dir);
    let work = operation();
    db.write(work.clone(), |tx| {
        for delete in [false, true] {
            for bad_row in [vec![Value::U64(1)], vec![Value::U64(1), Value::I64(2)]] {
                let arity = bad_row.len();
                let rows = [entry_row("not-staged", 2), bad_row];
                let attempted = work.used(Resource::Rows);
                let failure = if delete {
                    tx.delete_dyn(ENTRY, rows)
                } else {
                    tx.insert_dyn(ENTRY, rows)
                }
                .unwrap_err();
                assert_eq!(work.used(Resource::Rows), attempted + 2);
                match failure {
                    Error::FactShape(crate::error::FactShapeError::ArityMismatch {
                        relation,
                        mismatch,
                    }) => {
                        assert_eq!(relation, ENTRY);
                        assert_eq!((mismatch.witnessed, mismatch.required), (arity, 2));
                    }
                    Error::FactShape(crate::error::FactShapeError::TypeMismatch {
                        relation,
                        field,
                    }) => {
                        assert_eq!(relation, ENTRY);
                        assert_eq!(field, bumbledb_theory::schema::FieldId(0));
                        assert_eq!(arity, 2);
                    }
                    error => panic!("wrong shape error: {error:?}"),
                }
                assert!(tx.poisoned().is_none());
                assert!(!tx.contains_dyn(ENTRY, &entry_row("not-staged", 2))?);
            }
        }
        // The transaction is still usable: nothing had applied.
        tx.insert_dyn(ENTRY, [entry_row("fine", 1)]).map(|_| ())
    })
    .expect("write")
    .unwrap();
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        1
    );
}

#[test]
fn dynamic_ingestion_stops_at_work_refusal_before_later_bad_rows() {
    let dir = TempDir::new("dynamic-input-refusal");
    let db = create(&dir);
    for cancel in [false, true] {
        for prefix in [false, true] {
            let work = operation();
            let parent = db.store.snapshot(&work).unwrap();
            let mut tx: super::WriteTx<'_, Ledger> =
                super::WriteTx::new(&db.schema, &db.closed, &parent, &work);
            if prefix {
                tx.insert_dyn(ENTRY, [entry_row("prior", 0)]).unwrap();
            }
            let retained = work.used(Resource::WorkingBytes);
            if cancel {
                work.cancel();
            } else {
                work.rows(work.limit(Resource::Rows) - work.used(Resource::Rows) - 1)
                    .unwrap();
            }
            let mut seen = 0;
            let rows = [
                entry_row("never-staged", 1),
                entry_row("also-never-staged", 2),
                vec![Value::U64(9)],
            ]
            .into_iter()
            .inspect(|_| seen += 1);
            let error = tx.insert_dyn(ENTRY, rows).unwrap_err();
            let Error::Store(error) = error else {
                panic!("ingestion must stop at its work refusal");
            };
            assert!(
                matches!(
                    *error,
                    StoreError::Changes(crate::changes::ChangeError::Row(
                        crate::canonical::RowError::Work(crate::WorkError::Cancelled)
                    )) if cancel
                ) || matches!(
                    *error,
                    StoreError::Changes(crate::changes::ChangeError::Row(
                        crate::canonical::RowError::Work(crate::WorkError::Exhausted {
                            resource: Resource::Rows,
                            ..
                        })
                    )) if !cancel
                )
            );
            assert_eq!(seen, if cancel { 1 } else { 2 });
            assert_eq!(tx.poisoned().is_some(), prefix);
            assert_eq!(work.used(Resource::WorkingBytes), retained);
            let pending = tx.into_pending().seal(&db.schema, &operation()).unwrap();
            assert_eq!(pending.len(), u64::from(prefix));
        }
    }
}

#[test]
fn poison_preserves_the_original_error_after_an_applied_prefix() {
    let dir = TempDir::new("db-poison");
    let db = create(&dir);
    let err = db
        .write(operation(), |tx| {
            tx.insert_dyn(ENTRY, [entry_row("applied", 1)])?;
            // Now a later collection fails its shape check: the transaction
            // poisons (a prefix already entered the delta).
            let failure = tx.insert_dyn(ENTRY, [vec![Value::U64(9)]]).unwrap_err();
            for report in [
                tx.insert_dyn(RelationId(77), std::iter::empty::<Vec<Value>>())?,
                tx.delete_dyn(RelationId(77), std::iter::empty::<Vec<Value>>())?,
            ] {
                assert_eq!((report.submitted(), report.changed()), (0, 0));
            }
            // Every later operation reports the poisoned state.
            let poisoned = tx
                .contains_dyn(ENTRY, &entry_row("applied", 1))
                .unwrap_err();
            assert!(matches!(poisoned, Error::TransactionPoisoned { .. }));
            // Swallow the refusal: the engine still refuses to commit.
            drop(failure);
            Ok(())
        })
        .expect_err("poisoned write refuses commit");
    assert!(matches!(err, Error::TransactionPoisoned { .. }), "{err:?}");
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        0
    );
}

#[test]
fn an_empty_write_commits_without_moving_the_generation() {
    let dir = TempDir::new("db-empty-write");
    let db = create(&dir);
    let committed = db.write(operation(), |_tx| Ok(42)).expect("write").unwrap();
    assert_eq!(committed.value, 42);
    assert_eq!(committed.generation.value(), 0);
}

#[test]
fn dynamic_empty_collections_bypass_guards_and_nullary_rows_remain_facts() {
    let dir = TempDir::new("dynamic-nullary");
    let descriptor = SchemaDescriptor {
        relations: vec![
            bumbledb_theory::schema::RelationDescriptor {
                name: "flag".into(),
                fields: vec![],
                extension: None,
            },
            bumbledb_theory::schema::RelationDescriptor {
                name: "closed".into(),
                fields: vec![],
                extension: Some(Box::new([Row {
                    handle: "present".into(),
                    values: Box::new([]),
                }])),
            },
        ],
        statements: vec![],
    };
    let db = Db::create(dir.path(), descriptor, operation())
        .unwrap()
        .unwrap();
    db.write(operation(), |tx| {
        for relation in [RelationId(1), RelationId(77)] {
            for report in [
                tx.insert_dyn(relation, std::iter::empty::<Vec<Value>>())?,
                tx.delete_dyn(relation, std::iter::empty::<Vec<Value>>())?,
            ] {
                assert_eq!((report.submitted(), report.changed()), (0, 0));
            }
        }
        let empty_rows = || std::iter::repeat_with(Vec::<Value>::new).take(3);
        let inserted = tx.insert_dyn(ENTRY, empty_rows())?;
        assert_eq!((inserted.submitted(), inserted.changed()), (3, 1));
        assert!(tx.contains_dyn(ENTRY, &[])?);
        let deleted = tx.delete_dyn(ENTRY, empty_rows())?;
        assert_eq!((deleted.submitted(), deleted.changed()), (3, 1));
        assert!(!tx.contains_dyn(ENTRY, &[])?);
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert_eq!(db.generation(operation()).unwrap().value(), 0);
}

#[test]
fn a_reentrant_write_is_refused_typed_not_deadlocked() {
    let dir = TempDir::new("db-reentrant");
    let db = create(&dir);
    let err = db
        .write(operation(), |_outer| {
            // The transitional surface panicked here; the successor refuses
            // with the store's typed reentrancy error (same safety intent,
            // now an answer instead of an abort).
            match db.write(operation(), |_inner| Ok(())) {
                Err(error) => Err::<(), _>(error),
                Ok(_) => panic!("nested write must not run"),
            }
        })
        .expect_err("nested write refused");
    assert!(
        matches!(&err, Error::Store(inner) if matches!(**inner, StoreError::ReentrantWriter)),
        "{err:?}"
    );
}

// --- Closed relations. ---

const CURRENCY: RelationId = RelationId(0);

#[derive(Clone, Copy)]
struct Currencies;

impl crate::Theory for Currencies {
    fn descriptor(self) -> SchemaDescriptor {
        SchemaDescriptor {
            relations: vec![bumbledb_theory::schema::RelationDescriptor {
                name: "Currency".into(),
                fields: vec![FieldDescriptor {
                    name: "minor_units".into(),
                    value_type: ValueType::U64,
                }],
                extension: Some(Box::new([
                    Row {
                        handle: "Usd".into(),
                        values: Box::new([Value::U64(2)]),
                    },
                    Row {
                        handle: "Eur".into(),
                        values: Box::new([Value::U64(2)]),
                    },
                ])),
            }],
            statements: vec![],
        }
    }
}

#[test]
fn writes_to_a_closed_relation_are_refused_before_the_delta() {
    let dir = TempDir::new("db-closed-write");
    let db = Db::create(dir.path(), Currencies, operation())
        .expect("create")
        .expect("accepted");
    let insert = db.write(operation(), |tx| {
        tx.insert_dyn(CURRENCY, [&[Value::U64(9)]]).map(|_| ())
    });
    assert!(matches!(
        insert,
        Err(Error::ClosedRelationWrite { relation }) if relation == CURRENCY
    ));
    // A closure that swallows the refusal commits empty: the generation
    // never moves and the store stays rowless.
    db.write(operation(), |tx| {
        let _ = tx.delete_dyn(CURRENCY, [&[Value::U64(0), Value::U64(2)]]);
        Ok(())
    })
    .expect("write")
    .unwrap();
    assert_eq!(db.generation(operation()).expect("generation").value(), 0);
}

#[test]
fn closed_point_reads_resolve_against_the_extension() {
    let dir = TempDir::new("db-closed-read");
    let db = Db::create(dir.path(), Currencies, operation())
        .expect("create")
        .expect("accepted");
    // The auto-materialized handle key is the first statement.
    let key = StatementId(0);
    db.read(operation(), |snap| {
        assert_eq!(snap.count(CURRENCY)?, 2);
        let usd = snap
            .get_dyn(CURRENCY, key, &[Value::U64(0)])?
            .expect("sealed row");
        assert_eq!(usd.values(), &[Value::U64(0), Value::U64(2)]);
        assert!(snap.contains_dyn(CURRENCY, &[Value::U64(1), Value::U64(2)])?);
        assert!(!snap.contains_dyn(CURRENCY, &[Value::U64(1), Value::U64(3)])?);
        let rows: Vec<crate::canonical::DecodedRow> =
            snap.scan(CURRENCY)?.collect::<Result<_>>()?;
        assert_eq!(rows.len(), 2);
        Ok(())
    })
    .expect("read");
    // The same answers inside a write transaction's view.
    db.write(operation(), |tx| {
        assert!(tx.contains_dyn(CURRENCY, &[Value::U64(0), Value::U64(2)])?);
        assert!(
            tx.get_dyn(CURRENCY, key, &[Value::U64(1)])?.is_some(),
            "closed reads are identical on both surfaces"
        );
        Ok(())
    })
    .expect("write")
    .unwrap();
}

// --- InstanceBuilder / OwnedInstance / publication. ---

#[test]
fn a_builder_admits_judged_content_and_publishes_it() {
    let mut builder = InstanceBuilder::new(Ledger, operation()).expect("builder");
    builder
        .load([&Entry {
            name: "alpha",
            amount: 3,
        }])
        .expect("load");
    builder
        .load_dyn(ENTRY, [entry_row("beta", -7)])
        .expect("load_dyn");
    // Overlay reads see the staged state.
    assert!(
        builder
            .contains(&Entry {
                name: "alpha",
                amount: 3
            })
            .expect("contains")
    );
    let staged = builder
        .get(EntryName("beta"))
        .expect("get")
        .expect("staged row");
    assert_eq!(
        staged,
        Entry {
            name: "beta",
            amount: -7
        }
    );
    let instance = builder.admit().expect("admit").expect("lawful content");
    assert_eq!(instance.count(ENTRY).expect("count"), 2);

    // Publication: the durable copy carries exactly the admitted content.
    let dir = TempDir::new("db-from-instance");
    let path = dir.path().join("published");
    let db = Db::from_instance(&path, &instance, operation()).expect("publish");
    assert_eq!(
        db.catalog_digest(operation()).expect("digest"),
        instance.catalog_digest().expect("digest"),
        "the replication oracle agrees across backends"
    );
}

#[test]
fn a_builder_rejection_is_the_same_complete_verdict() {
    let mut builder = InstanceBuilder::new(Ledger, operation()).expect("builder");
    builder
        .load([
            &Entry {
                name: "dup",
                amount: 1,
            },
            &Entry {
                name: "dup",
                amount: 2,
            },
        ])
        .expect("staging accepts; judgment decides");
    let violations = match builder.admit().expect("admit runs") {
        Admission::Rejected(violations) => violations,
        Admission::Accepted(_) => panic!("conflicting keys must reject"),
    };
    assert_eq!(violations.len(), 1);
    assert!(violations.cited_facts(0).len() >= 2);
}

#[test]
fn builder_deletes_are_set_arithmetic_from_empty() {
    let mut builder = InstanceBuilder::new(Ledger, operation()).expect("builder");
    let report = builder
        .load_dyn(ENTRY, [entry_row("a", 1), entry_row("b", 2)])
        .expect("load");
    assert_eq!((report.submitted(), report.changed()), (2, 2));
    let report = builder
        .delete_dyn(ENTRY, [entry_row("a", 1), entry_row("absent", 9)])
        .expect("delete");
    assert_eq!((report.submitted(), report.changed()), (2, 1));
    let instance = builder.admit().expect("admit").expect("lawful");
    assert_eq!(instance.count(ENTRY).expect("count"), 1);
    assert!(
        instance
            .contains_dyn(ENTRY, &entry_row("b", 2), &operation())
            .expect("contains")
    );
}

// --- AcceptedCollection walls (one shape judgment for every lane). ---

#[test]
fn accepted_collections_hit_the_same_walls_as_the_dyn_lane() {
    let dir = TempDir::new("db-accepted-walls");
    let db = create(&dir);
    let fields = db.schema().relation(ENTRY).fields().to_vec();
    // Foreign arity: a one-field roster against the two-field relation.
    let narrow = crate::AcceptedCollection::from_value_rows(
        ENTRY,
        &fields[..1],
        [vec![Value::String("a".into())]],
    )
    .expect("internally consistent");
    let err = db
        .write(operation(), |tx| tx.insert_accepted(&narrow).map(|_| ()))
        .expect_err("foreign arity refused at apply");
    assert!(matches!(
        err,
        Error::FactShape(crate::error::FactShapeError::ArityMismatch { .. })
    ));
    // Foreign type roster of the right arity.
    let foreign_fields = vec![
        FieldDescriptor {
            name: "name".into(),
            value_type: ValueType::U64,
        },
        FieldDescriptor {
            name: "amount".into(),
            value_type: ValueType::I64,
        },
    ];
    let foreign = crate::AcceptedCollection::from_value_rows(
        ENTRY,
        &foreign_fields,
        [vec![Value::U64(1), Value::I64(2)]],
    )
    .expect("internally consistent");
    let err = db
        .write(operation(), |tx| tx.insert_accepted(&foreign).map(|_| ()))
        .expect_err("foreign types refused at apply");
    assert!(matches!(
        err,
        Error::FactShape(crate::error::FactShapeError::TypeMismatch { .. })
    ));
    // The mis-typed cell refuses at construction on the true roster.
    assert!(
        crate::AcceptedCollection::from_value_rows(
            ENTRY,
            &fields,
            [vec![Value::U64(1), Value::I64(2)]],
        )
        .is_err()
    );
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        0
    );
}

#[test]
fn accepted_reports_are_exact_and_delete_never_mints() {
    let dir = TempDir::new("db-accepted-reports");
    let db = create(&dir);
    let fields = db.schema().relation(ENTRY).fields().to_vec();
    let rows = crate::AcceptedCollection::from_value_rows(
        ENTRY,
        &fields,
        [entry_row("a", 1), entry_row("a", 1), entry_row("b", 2)],
    )
    .expect("shape proof");
    db.write(operation(), |tx| {
        let report = tx.insert_accepted(&rows)?;
        assert_eq!((report.submitted(), report.changed()), (3, 2));
        let report = tx.delete_accepted(&rows)?;
        assert_eq!((report.submitted(), report.changed()), (3, 2));
        Ok(())
    })
    .expect("write")
    .unwrap();
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        0
    );
    assert_eq!(
        db.generation(operation()).expect("generation").value(),
        0,
        "insert and delete of the same rows is one no-op command"
    );
}

// --- Compaction and the integration adjunct. ---

#[test]
fn compact_copies_content_host_records_and_generation_coherently() {
    let dir = TempDir::new("db-compact");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "kept",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    // Attach one host record + attachment through the integration seam.
    let work = super::test_operation().expect("work");
    {
        let mut session = db.integration_writer(&work).expect("session");
        let empty = ChangeSet::builder(db.schema(), work.clone())
            .finish()
            .expect("empty delta");
        let prepared = match session.prepare(&empty).expect("prepare") {
            Admission::Accepted(prepared) => prepared,
            Admission::Rejected(_) => panic!("the empty delta admits"),
        };
        let records = [crate::storage::store::HostRecordChange::Put {
            key: b"receipt/1",
            value: b"decided",
        }];
        let sealed = prepared
            .seal(crate::storage::store::HostChanges {
                records: &records,
                attachment: crate::storage::store::AttachmentChange::Put(b"control"),
            })
            .expect("seal");
        let commit = sealed.commit().expect("commit");
        assert!(commit.changed, "host mutation advances the generation once");
    }
    let generation = db.generation(operation()).expect("generation");
    let dest = dir.path().join("compacted");
    db.compact(&dest, operation()).expect("compact");
    let copy = Db::open(&dest, Ledger, operation()).expect("open copy");
    assert_eq!(
        copy.catalog_digest(operation()).expect("digest"),
        db.catalog_digest(operation()).expect("digest")
    );
    assert_eq!(
        copy.generation(operation()).expect("generation"),
        generation
    );
    copy.read(operation(), |snap| {
        assert_eq!(
            snap.integration_host_record(b"receipt/1").expect("record"),
            Some(b"decided".as_slice())
        );
        assert_eq!(
            snap.integration_host_attachment()?,
            Some(b"control".as_slice())
        );
        Ok(())
    })
    .expect("read copy");
}

#[test]
fn a_rejected_integration_candidate_retains_the_session() {
    let dir = TempDir::new("db-session-retained");
    let db = create(&dir);
    let work: WorkContext = super::test_operation().expect("work");
    let conflicting = {
        let mut builder = ChangeSet::builder(db.schema(), work.clone());
        builder
            .insert(ENTRY, &entry_row("same", 1))
            .expect("draft row");
        builder
            .insert(ENTRY, &entry_row("same", 2))
            .expect("draft row");
        builder.finish().expect("sealed delta")
    };
    let mut session = db.integration_writer(&work).expect("session");
    match session.prepare(&conflicting).expect("prepare") {
        Admission::Rejected(violations) => assert_eq!(violations.len(), 1),
        Admission::Accepted(_) => panic!("conflicting keys must reject"),
    }
    // The same exclusive session prepares the receipt-only follow-up: no
    // gap for another writer, no application fact changed.
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .expect("empty delta");
    let prepared = match session.prepare(&empty).expect("prepare") {
        Admission::Accepted(prepared) => prepared,
        Admission::Rejected(_) => panic!("the empty delta admits"),
    };
    let records = [crate::storage::store::HostRecordChange::Put {
        key: b"receipt/rejected",
        value: b"rejection recorded",
    }];
    let sealed = prepared
        .seal(crate::storage::store::HostChanges {
            records: &records,
            attachment: crate::storage::store::AttachmentChange::Keep,
        })
        .expect("seal");
    sealed.commit().expect("commit");
    drop(session);
    assert_eq!(
        db.read(operation(), |snap| snap.count(ENTRY))
            .expect("count"),
        0
    );
    db.read(operation(), |snap| {
        assert!(
            snap.integration_host_record(b"receipt/rejected")
                .expect("record")
                .is_some()
        );
        Ok(())
    })
    .expect("read");
}

/// Incremental apply on an admitted store: a lawful insert commits, a
/// key conflict is `InvariantRejected`, and the snapshot still sees only
/// the admitted row. Verification `NotRun`.
#[test]
fn apply_conflict_is_invariant_rejected_after_accepted_write() {
    let dir = TempDir::new("db-apply-invariant");
    let db = create(&dir);
    let work = operation();
    let accepted = {
        let mut builder = ChangeSet::builder(db.schema(), work.clone());
        builder
            .insert(ENTRY, &entry_row("acct", 1))
            .expect("insert");
        builder.finish().expect("seal")
    };
    match db
        .apply(&accepted, super::ApplyExpected::Any, &work)
        .expect("apply")
    {
        super::ApplyOutcome::Accepted { .. } => {}
        super::ApplyOutcome::NoChange { .. } => panic!("a new row must change"),
        super::ApplyOutcome::InvariantRejected { .. } => {
            panic!("a lone keyed row is lawful")
        }
        super::ApplyOutcome::Moved { .. } => panic!("Any cannot Move"),
    }
    let conflict = {
        let mut builder = ChangeSet::builder(db.schema(), work.clone());
        builder
            .insert(ENTRY, &entry_row("acct", 2))
            .expect("insert");
        builder.finish().expect("seal")
    };
    match db
        .apply(&conflict, super::ApplyExpected::Any, &work)
        .expect("conflict")
    {
        super::ApplyOutcome::InvariantRejected { violations } => {
            assert!(!violations.is_empty(), "the key statement is cited");
        }
        super::ApplyOutcome::Accepted { .. } | super::ApplyOutcome::NoChange { .. } => {
            panic!("a second row on the same key must reject")
        }
        super::ApplyOutcome::Moved { .. } => panic!("Any cannot Move"),
    }
    let pin = db.snapshot(&work).expect("pin");
    assert_eq!(pin.count(ENTRY).expect("count"), 1);
    assert!(
        pin.frame(&work)
            .contains_dyn(ENTRY, &entry_row("acct", 1))
            .expect("winner remains")
    );
}

#[test]
fn scoped_read_borrows_metadata_while_owned_read_retains_it() {
    let dir = TempDir::new("db-read-metadata-ownership");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "alpha",
            amount: 7,
        }])
        .map(|_| ())
    })
    .expect("seed")
    .expect("lawful seed");
    let metadata_owners = || {
        [
            std::sync::Arc::strong_count(&db.schema),
            std::sync::Arc::strong_count(&db.closed),
            std::sync::Arc::strong_count(&db.cache),
        ]
    };
    let before = metadata_owners();
    let witness = db
        .read(operation(), |frame| {
            assert_eq!(
                metadata_owners(),
                before,
                "a scoped frame borrows metadata instead of retaining three temporary owners"
            );
            assert_eq!(
                frame.get(EntryName("alpha"))?,
                Some(Entry {
                    name: "alpha",
                    amount: 7,
                })
            );
            frame.witness()
        })
        .expect("scoped read");
    assert_eq!(metadata_owners(), before);
    let work = operation();
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .expect("empty delta");
    assert!(matches!(
        db.apply(&empty, super::ApplyExpected::Exact(witness), &work)
            .expect("scoped witness remains usable"),
        super::ApplyOutcome::NoChange { .. }
    ));
    let snapshot = db.snapshot(&operation()).expect("owned snapshot");
    assert_eq!(metadata_owners(), before.map(|count| count + 1));
    let fact = {
        let temporary_work = operation();
        snapshot
            .get(EntryName("alpha"), &temporary_work)
            .expect("owned read")
            .expect("stored fact")
    };
    assert_eq!(
        fact,
        Entry {
            name: "alpha",
            amount: 7,
        },
        "borrowed row bytes depend on the snapshot, not its operation's work lifetime"
    );
    drop(snapshot);
    assert_eq!(metadata_owners(), before);
}

#[test]
fn owned_read_keeps_rows_but_does_not_retain_retired_resolvers() {
    let dir = TempDir::new("db-owned-read-resolver-retirement");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "alpha",
            amount: 7,
        }])?;
        Ok(())
    })
    .expect("seed")
    .expect("lawful seed");
    let retired = db.cache.weak_current();
    let snapshot = db.owned_read().expect("snapshot");
    let generation = snapshot.generation();
    db.cache.trim();
    assert!(
        retired.upgrade().is_none(),
        "a canonical-row snapshot must not retain an unused retired text resolver"
    );
    assert_eq!(snapshot.generation(), generation);
    assert_eq!(
        snapshot.get(EntryName("alpha"), &operation()).expect("get"),
        Some(Entry {
            name: "alpha",
            amount: 7,
        })
    );
}

fn entry_scan_query() -> crate::ir::Query {
    use crate::ir::{Atom, AtomSource, FindTerm, Query, Rule, Term, VarId};
    use bumbledb_theory::schema::FieldId;

    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(ENTRY),
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
fn owned_read_text_queries_refresh_resolvers_without_refreshing_rows_or_work() {
    let dir = TempDir::new("db-owned-text-snapshot");
    let db = create(&dir);
    db.write(operation(), |tx| {
        tx.insert([&Entry {
            name: "alpha",
            amount: 7,
        }])
        .map(|_| ())
    })
    .expect("seed")
    .expect("lawful seed");
    let admission = operation();
    let snapshot = db.snapshot(&admission).expect("snapshot");
    let durable_generation = snapshot.generation();
    admission.cancel();
    let mut prepared = snapshot
        .prepare(&entry_scan_query(), &operation())
        .expect("prepare");
    let params: &[crate::ParamArg] = &[];
    let assert_row = |answers: &crate::Answers, name: &str, amount: i64| {
        assert_eq!(answers.len(), 1);
        assert_eq!(answers.get(0, 0), crate::AnswerValue::String(name));
        assert_eq!(answers.get(0, 1), crate::AnswerValue::I64(amount));
    };
    let original = prepared
        .execute_collect_owned(&snapshot, &operation(), params)
        .expect("original query");
    assert_row(&original, "alpha", 7);

    db.write(operation(), |tx| {
        tx.delete([&Entry {
            name: "alpha",
            amount: 7,
        }])?;
        tx.insert([&Entry {
            name: "beta",
            amount: 9,
        }])
        .map(|_| ())
    })
    .expect("replace")
    .expect("lawful replacement");
    assert_ne!(
        db.generation(operation()).expect("generation"),
        durable_generation
    );
    for _ in 0..2 {
        db.cache.trim();
        let current = db.cache.acquire();
        assert_eq!(current.resolver().lookup("alpha"), None);
        let answers = prepared
            .execute_collect_owned(&snapshot, &operation(), params)
            .expect("same snapshot under fresh resolver and work");
        assert_row(&answers, "alpha", 7);
        assert!(
            current.resolver().lookup("alpha").is_some(),
            "an old durable snapshot must use this operation's current resolver"
        );
        assert_eq!(snapshot.generation(), durable_generation);
        let latest = db
            .read(operation(), |frame| {
                frame.execute_collect(&mut prepared, params)
            })
            .expect("latest snapshot");
        assert_row(&latest, "beta", 9);
        // A newer cached image cannot replace the older snapshot's rows.
        let old_again = prepared
            .execute_collect_owned(&snapshot, &operation(), params)
            .expect("old snapshot after newer image");
        assert_row(&old_again, "alpha", 7);
    }
    let cancelled = operation();
    cancelled.cancel();
    let error = prepared
        .execute_collect_owned(&snapshot, &cancelled, params)
        .expect_err("execution obeys its fresh work, not admission's work");
    assert!(matches!(
        error,
        Error::Store(error) if matches!(error.as_ref(), StoreError::Work(crate::work::WorkError::Cancelled))
    ));
    let recovered = prepared
        .execute_collect_owned(&snapshot, &operation(), params)
        .expect("fresh work after cancellation");
    assert_row(&recovered, "alpha", 7);
    drop(snapshot);
    prepared.trim();
    assert_row(&original, "alpha", 7);
    assert_row(&recovered, "alpha", 7);
}

/// After apply, the owned pin's frame reads the row, collects the scan,
/// and close stays Incomplete until the pin drops. Verification `NotRun`.
#[test]
fn owned_read_frame_reads_applied_row_and_close_waits() {
    let dir = TempDir::new("db-owned-frame");
    let db = create(&dir);
    let work = operation();
    let changes = {
        let mut builder = ChangeSet::builder(db.schema(), work.clone());
        builder.insert(ENTRY, &entry_row("pin", 4)).expect("insert");
        builder.finish().expect("seal")
    };
    match db
        .apply(&changes, super::ApplyExpected::Any, &work)
        .expect("apply")
    {
        super::ApplyOutcome::Accepted { .. } => {}
        super::ApplyOutcome::NoChange { .. }
        | super::ApplyOutcome::InvariantRejected { .. }
        | super::ApplyOutcome::Moved { .. } => panic!("expected Accepted"),
    }
    let pin = db.owned_read().expect("pin");
    assert_eq!(pin.count(ENTRY).expect("count"), 1);
    assert_eq!(
        pin.generation().value(),
        db.generation(work.clone()).expect("gen").value()
    );
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .expect("empty");
    match db
        .apply(&empty, super::ApplyExpected::Exact(pin.witness()), &work)
        .expect("no-change")
    {
        super::ApplyOutcome::NoChange { .. } => {}
        super::ApplyOutcome::Accepted { .. }
        | super::ApplyOutcome::InvariantRejected { .. }
        | super::ApplyOutcome::Moved { .. } => {
            panic!("empty apply under the pin's witness is NoChange")
        }
    }
    let frame = pin.frame(&work);
    assert!(
        frame
            .contains_dyn(ENTRY, &entry_row("pin", 4))
            .expect("contains")
    );
    let out = frame
        .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("pin".into())])
        .expect("get")
        .expect("pin row");
    assert_eq!(out.values(), entry_row("pin", 4));
    let scanned: Vec<crate::canonical::DecodedRow> = frame
        .scan(ENTRY)
        .expect("scan")
        .collect::<Result<Vec<_>>>()
        .expect("rows");
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].values(), entry_row("pin", 4));
    let query = entry_scan_query();
    let mut prepared = frame.prepare(&query).expect("prepare");
    let answers = prepared
        .execute_collect_owned(&pin, &work, &[] as &[crate::ParamArg])
        .expect("collect");
    assert_eq!(answers.len(), 1);
    let close_work = operation();
    close_work.cancel();
    match db.close(&close_work) {
        crate::CloseReport::Incomplete {
            live_transactions, ..
        } => assert!(live_transactions >= 1),
        crate::CloseReport::Closed => panic!("close cannot complete under a live pin"),
    }
    drop(pin);
    assert_eq!(db.close(&work), crate::CloseReport::Closed);
}
