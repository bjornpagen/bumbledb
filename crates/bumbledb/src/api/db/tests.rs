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
use crate::storage::store::StoreError;
use crate::testutil::{TempDir, expect_rejected};
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

fn create(dir: &TempDir) -> Db<Ledger> {
    Db::create(dir.path(), Ledger)
        .expect("create")
        .expect("empty theory admits")
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
        .write(|tx| {
            let report = tx.insert(facts.iter())?;
            assert_eq!(report.submitted(), 2);
            assert_eq!(report.changed(), 2);
            Ok(())
        })
        .expect("write")
        .unwrap();

    dynamic
        .write(|tx| {
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
        .write(|tx| tx.insert_accepted(&collection).map(|_| ()))
        .expect("write")
        .unwrap();

    let digest = typed.catalog_digest().expect("digest");
    assert_eq!(digest, dynamic.catalog_digest().expect("digest"));
    assert_eq!(digest, accepted.catalog_digest().expect("digest"));
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
    db.write(|tx| {
        tx.insert([&Entry {
            name: "kept",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    let before = db.generation().expect("generation");
    db.write(|tx| {
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
    assert_eq!(db.generation().expect("generation"), before);
    assert_eq!(
        db.read(|snap| snap.count(ENTRY)).expect("count"),
        1,
        "the typo delete deleted nothing"
    );
}

#[test]
fn deleted_text_is_unreachable_after_delete_and_reopen() {
    let dir = TempDir::new("db-text-gone");
    {
        let db = create(&dir);
        db.write(|tx| {
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
        db.write(|tx| {
            tx.delete([&Entry {
                name: "ephemeral",
                amount: 2,
            }])
            .map(|_| ())
        })
        .expect("write")
        .unwrap();
        assert!(
            !db.read(|snap| snap.contains(&Entry {
                name: "ephemeral",
                amount: 2
            }))
            .expect("contains")
        );
    }
    // Reopen: canonical rows own their text inline; the deleted tuple left
    // no independently live text entry anywhere (ENG-006: no dictionary
    // namespace even exists in the store).
    let db = Db::open(dir.path(), Ledger).expect("open");
    let rows: Vec<Vec<Value>> = db
        .read(|snap| snap.scan(ENTRY)?.collect::<Result<_>>())
        .expect("scan");
    assert_eq!(rows, vec![entry_row("resident", 1)]);
}

// --- Point reads: own writes, committed fall-through, typed errors. ---

#[test]
fn get_dyn_reads_its_own_writes_exactly_as_a_later_transaction_does() {
    let dir = TempDir::new("db-own-writes");
    let db = create(&dir);
    db.write(|tx| {
        tx.insert_dyn(ENTRY, [entry_row("alpha", 3)])?;
        // The write's own final-state view answers like a later reader.
        let row = tx
            .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("alpha".into())])?
            .expect("own write visible");
        assert_eq!(row, entry_row("alpha", 3));
        // Replacement inside the same command: delete + insert, either order.
        tx.delete_dyn(ENTRY, [entry_row("alpha", 3)])?;
        tx.insert_dyn(ENTRY, [entry_row("alpha", 5)])?;
        let row = tx
            .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("alpha".into())])?
            .expect("replacement visible");
        assert_eq!(row, entry_row("alpha", 5));
        assert!(tx.contains_dyn(ENTRY, &entry_row("alpha", 5))?);
        assert!(!tx.contains_dyn(ENTRY, &entry_row("alpha", 3))?);
        Ok(())
    })
    .expect("write")
    .unwrap();
    let row = db
        .read(|snap| snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("alpha".into())]))
        .expect("read")
        .expect("committed");
    assert_eq!(row, entry_row("alpha", 5));
}

#[test]
fn get_dyn_falls_through_to_committed_state() {
    let dir = TempDir::new("db-fall-through");
    let db = create(&dir);
    db.write(|tx| tx.insert_dyn(ENTRY, [entry_row("base", 10)]).map(|_| ()))
        .expect("write")
        .unwrap();
    db.write(|tx| {
        // Nothing pending for "base": the read falls through to committed.
        let row = tx
            .get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("base".into())])?
            .expect("committed row visible");
        assert_eq!(row, entry_row("base", 10));
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
        db.read(|snap| snap.contains_dyn(ENTRY, &entry_row("base", 10)))
            .expect("read"),
        "the aborted delete never reached storage"
    );
}

#[test]
fn get_dyn_rejects_mis_shaped_requests_with_typed_errors() {
    let dir = TempDir::new("db-mis-shaped");
    let db = create(&dir);
    db.read(|snap| {
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
    db.write(|tx| {
        tx.insert([&Entry {
            name: "gamma",
            amount: 4,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    db.read(|snap| {
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
    let violations = expect_rejected(db.write(|tx| {
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
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 0);
    assert_eq!(db.generation().expect("generation").value(), 0);
}

#[test]
fn a_conflict_with_a_committed_row_rejects_and_preserves_it() {
    let dir = TempDir::new("db-committed-conflict");
    let db = create(&dir);
    db.write(|tx| {
        tx.insert([&Entry {
            name: "holder",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    let violations = expect_rejected(db.write(|tx| {
        tx.insert([&Entry {
            name: "holder",
            amount: 2,
        }])
        .map(|_| ())
    }));
    assert_eq!(violations.len(), 1);
    let row = db
        .read(|snap| snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("holder".into())]))
        .expect("read")
        .expect("incumbent survives");
    assert_eq!(row, entry_row("holder", 1));
}

#[test]
fn replacement_in_one_command_is_judged_as_final_state() {
    let dir = TempDir::new("db-replacement");
    let db = create(&dir);
    db.write(|tx| {
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
    db.write(|tx| {
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
        .read(|snap| snap.get_dyn(ENTRY, ENTRY_NAME_KEY, &[Value::String("acct".into())]))
        .expect("read")
        .expect("replaced");
    assert_eq!(row, entry_row("acct", 2));
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
    let witness = db.read(|snap| snap.witness()).expect("witness");
    let again = witness.clone();
    let outcome = db
        .write_from(&witness, |tx| {
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
        .write_from(&again, |tx| {
            tx.insert([&Entry {
                name: "x",
                amount: 2,
            }])
            .map(|_| ())
        })
        .expect("write");
    assert!(matches!(moved, crate::ConditionalWrite::Moved { .. }));
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 1);
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
    let foreign = b.read(|snap| snap.witness()).expect("witness");
    let err = a
        .write_from(&foreign, |tx| {
            tx.insert([&Entry {
                name: "n",
                amount: 1,
            }])
            .map(|_| ())
        })
        .expect_err("foreign witness refused");
    assert!(matches!(err, Error::ForeignWitness), "{err:?}");
    assert_eq!(a.generation().expect("generation").value(), 0);
}

// --- Poisoning and refusal boundaries. ---

#[test]
fn a_shape_failure_does_not_poison_a_clean_write() {
    let dir = TempDir::new("db-clean-shape-fail");
    let db = create(&dir);
    db.write(|tx| {
        // A mis-shaped dyn insert refuses before anything is staged.
        assert!(tx.insert_dyn(ENTRY, [vec![Value::U64(1)]]).is_err());
        // The transaction is still usable: nothing had applied.
        tx.insert_dyn(ENTRY, [entry_row("fine", 1)]).map(|_| ())
    })
    .expect("write")
    .unwrap();
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 1);
}

#[test]
fn poison_preserves_the_original_error_after_an_applied_prefix() {
    let dir = TempDir::new("db-poison");
    let db = create(&dir);
    let err = db
        .write(|tx| {
            tx.insert_dyn(ENTRY, [entry_row("applied", 1)])?;
            // Now a later collection fails its shape check: the transaction
            // poisons (a prefix already entered the delta).
            let failure = tx.insert_dyn(ENTRY, [vec![Value::U64(9)]]).unwrap_err();
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
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 0);
}

#[test]
fn an_empty_write_commits_without_moving_the_generation() {
    let dir = TempDir::new("db-empty-write");
    let db = create(&dir);
    let committed = db.write(|_tx| Ok(42)).expect("write").unwrap();
    assert_eq!(committed.value, 42);
    assert_eq!(committed.generation.value(), 0);
}

#[test]
fn a_reentrant_write_is_refused_typed_not_deadlocked() {
    let dir = TempDir::new("db-reentrant");
    let db = create(&dir);
    let err = db
        .write(|_outer| {
            // The transitional surface panicked here; the successor refuses
            // with the store's typed reentrancy error (same safety intent,
            // now an answer instead of an abort).
            match db.write(|_inner| Ok(())) {
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
    let db = Db::create(dir.path(), Currencies)
        .expect("create")
        .expect("accepted");
    let insert = db.write(|tx| tx.insert_dyn(CURRENCY, [&[Value::U64(9)]]).map(|_| ()));
    assert!(matches!(
        insert,
        Err(Error::ClosedRelationWrite { relation }) if relation == CURRENCY
    ));
    // A closure that swallows the refusal commits empty: the generation
    // never moves and the store stays rowless.
    db.write(|tx| {
        let _ = tx.delete_dyn(CURRENCY, [&[Value::U64(0), Value::U64(2)]]);
        Ok(())
    })
    .expect("write")
    .unwrap();
    assert_eq!(db.generation().expect("generation").value(), 0);
}

#[test]
fn closed_point_reads_resolve_against_the_extension() {
    let dir = TempDir::new("db-closed-read");
    let db = Db::create(dir.path(), Currencies)
        .expect("create")
        .expect("accepted");
    // The auto-materialized handle key is the first statement.
    let key = StatementId(0);
    db.read(|snap| {
        assert_eq!(snap.count(CURRENCY)?, 2);
        let usd = snap
            .get_dyn(CURRENCY, key, &[Value::U64(0)])?
            .expect("sealed row");
        assert_eq!(usd, vec![Value::U64(0), Value::U64(2)]);
        assert!(snap.contains_dyn(CURRENCY, &[Value::U64(1), Value::U64(2)])?);
        assert!(!snap.contains_dyn(CURRENCY, &[Value::U64(1), Value::U64(3)])?);
        let rows: Vec<Vec<Value>> = snap.scan(CURRENCY)?.collect::<Result<_>>()?;
        assert_eq!(rows.len(), 2);
        Ok(())
    })
    .expect("read");
    // The same answers inside a write transaction's view.
    db.write(|tx| {
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
    let mut builder = InstanceBuilder::new(Ledger).expect("builder");
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
    let db = Db::from_instance(&path, &instance).expect("publish");
    assert_eq!(
        db.catalog_digest().expect("digest"),
        instance.catalog_digest().expect("digest"),
        "the replication oracle agrees across backends"
    );
}

#[test]
fn a_builder_rejection_is_the_same_complete_verdict() {
    let mut builder = InstanceBuilder::new(Ledger).expect("builder");
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
    let mut builder = InstanceBuilder::new(Ledger).expect("builder");
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
            .contains_dyn(ENTRY, &entry_row("b", 2))
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
        .write(|tx| tx.insert_accepted(&narrow).map(|_| ()))
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
        .write(|tx| tx.insert_accepted(&foreign).map(|_| ()))
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
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 0);
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
    db.write(|tx| {
        let report = tx.insert_accepted(&rows)?;
        assert_eq!((report.submitted(), report.changed()), (3, 2));
        let report = tx.delete_accepted(&rows)?;
        assert_eq!((report.submitted(), report.changed()), (3, 2));
        Ok(())
    })
    .expect("write")
    .unwrap();
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 0);
    assert_eq!(
        db.generation().expect("generation").value(),
        0,
        "insert and delete of the same rows is one no-op command"
    );
}

// --- Compaction and the integration adjunct. ---

#[test]
fn compact_copies_content_host_records_and_generation_coherently() {
    let dir = TempDir::new("db-compact");
    let db = create(&dir);
    db.write(|tx| {
        tx.insert([&Entry {
            name: "kept",
            amount: 1,
        }])
        .map(|_| ())
    })
    .expect("write")
    .unwrap();
    // Attach one host record + attachment through the integration seam.
    let work = super::embedded_work().expect("work");
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
    let generation = db.generation().expect("generation");
    let dest = dir.path().join("compacted");
    db.compact(&dest).expect("compact");
    let copy = Db::open(&dest, Ledger).expect("open copy");
    assert_eq!(
        copy.catalog_digest().expect("digest"),
        db.catalog_digest().expect("digest")
    );
    assert_eq!(copy.generation().expect("generation"), generation);
    copy.read(|snap| {
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
    let work: WorkContext = super::embedded_work().expect("work");
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
    assert_eq!(db.read(|snap| snap.count(ENTRY)).expect("count"), 0);
    db.read(|snap| {
        assert!(
            snap.integration_host_record(b"receipt/rejected")
                .expect("record")
                .is_some()
        );
        Ok(())
    })
    .expect("read");
}
