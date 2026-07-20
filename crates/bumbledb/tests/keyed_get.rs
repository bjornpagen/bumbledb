//! Keyed point reads through the GENERATED key structs
//! (`docs/architecture/70-api.md` § the `schema!` grammar): every
//! declared `R(x, ..) -> R` on an ordinary relation emits a
//! `{R}By{Fields}` struct implementing `Key`, its `STATEMENT` computed
//! at expansion by exactly `SchemaDescriptor::materialized_statements`'
//! rule — the schema below places a containment AND a bidirectional
//! `==` (two materialized slots) ahead of the declared keys, so the
//! const-id arithmetic is exercised, not assumed.

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
    // The `==`'s mirror containment targets Meta(grp), so grp must key
    // Meta — declared LAST, so the two asserted key ids above are
    // untouched by it.
    Meta(grp) -> Meta;
}

/// A committed Task answers through `TaskByKindSubject` on BOTH scopes:
/// the snapshot (`db.read(|snap| snap.get(..))`) and the write
/// transaction (`tx.get(..)`) return the same fact — the composite key's
/// columns ride the host-enum weld for the closed-relation cell
/// (`Kind::Alpha.id()`), and a determinant nobody wrote misses cleanly.
#[test]
fn keyed_get_reads_through_a_declared_key_on_both_scopes() {
    let dir = common::TempDir::new("keyed-get-both-scopes");
    let db = Db::create(dir.path(), KeyedGet).expect("create");
    let (grp, task) = db
        .write(|tx| {
            let grp = tx.alloc::<GrpId>()?;
            tx.insert(&Grp {
                id: grp,
                label: "home",
            })?;
            tx.insert(&Meta {
                grp,
                title: "the home group",
            })?;
            let task = tx.alloc::<TaskId>()?;
            tx.insert(&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "water",
            })?;
            Ok((grp, task))
        })
        .expect("seed");

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
        // The other vocabulary row shares the subject but keys nothing.
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

    // The same value through `tx.get` inside `db.write` agrees.
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
    .expect("write-scope keyed get");
}

/// The const-id arithmetic under mirror offsets: schema admission
/// succeeds with a bidirectional `==` occupying TWO materialized slots
/// before the declared keys, and a keyed get through BOTH generated
/// structs answers correctly — this test fails if the expansion's
/// statement-id computation is off by one.
#[test]
fn keyed_get_statement_ids_survive_mirror_offsets() {
    // The materialized order (`SchemaDescriptor::materialized_statements`):
    // [0] Grp(id) -> Grp (fresh), [1] Task(id) -> Task (fresh),
    // [2] Kind(id) -> Kind (closed auto-key),
    // [3] Task(subject) <= Grp(id),
    // [4][5] Meta(grp) == Grp(id) — the two adjacent containments,
    // [6] Task(kind, subject) -> Task, [7] Grp(label) -> Grp,
    // [8] Meta(grp) -> Meta.
    assert_eq!(
        <TaskByKindSubject as Key>::STATEMENT,
        bumbledb::schema::StatementId(6)
    );
    assert_eq!(
        <GrpByLabel as Key>::STATEMENT,
        bumbledb::schema::StatementId(7)
    );

    let dir = common::TempDir::new("keyed-get-mirror-offsets");
    let db = Db::create(dir.path(), KeyedGet).expect("schema admission succeeds");
    let grp = db
        .write(|tx| {
            let grp = tx.alloc::<GrpId>()?;
            tx.insert(&Grp {
                id: grp,
                label: "inbox",
            })?;
            tx.insert(&Meta {
                grp,
                title: "the inbox",
            })?;
            let task = tx.alloc::<TaskId>()?;
            tx.insert(&Task {
                id: task,
                kind: Kind::Beta.id(),
                subject: grp,
                note: "triage",
            })?;
            Ok(grp)
        })
        .expect("seed");
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

/// String determinants resolve, never mint: a never-interned label
/// proves absence on a snapshot; inside a write transaction a NOVEL
/// label (a provisional intern id in the pending delta) is found —
/// read-your-writes — and after the compensating delete the key answers
/// `None` again.
#[test]
fn keyed_get_string_keys_resolve_pending_first_and_never_mint() {
    let dir = common::TempDir::new("keyed-get-string-keys");
    let db = Db::create(dir.path(), KeyedGet).expect("create");

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
        let grp = tx.alloc::<GrpId>()?;
        tx.insert(&Grp {
            id: grp,
            label: "novel-label",
        })?;
        // The label exists only as a provisional intern id here, and
        // the keyed get resolves it pending-first.
        assert_eq!(
            tx.get(GrpByLabel {
                label: "novel-label",
            })?,
            Some(Grp {
                id: grp,
                label: "novel-label",
            })
        );
        tx.delete(&Grp {
            id: grp,
            label: "novel-label",
        })?;
        assert_eq!(
            tx.get(GrpByLabel {
                label: "novel-label",
            })?,
            None
        );
        Ok(())
    })
    .expect("pending-first resolution");
}

/// The final-state overlay through a declared key: insert then keyed-get
/// in the same transaction (the delta's `Present` arm), delete then
/// keyed-get `None` (the `Absent` arm), reinsert modified — and every
/// pre-commit answer equals the post-commit read.
#[test]
fn keyed_get_observes_the_final_state_overlay() {
    let dir = common::TempDir::new("keyed-get-final-state");
    let db = Db::create(dir.path(), KeyedGet).expect("create");
    let (grp, task) = db
        .write(|tx| {
            let grp = tx.alloc::<GrpId>()?;
            tx.insert(&Grp {
                id: grp,
                label: "garden",
            })?;
            tx.insert(&Meta {
                grp,
                title: "the garden",
            })?;
            let task = tx.alloc::<TaskId>()?;
            let key = TaskByKindSubject {
                kind: Kind::Alpha.id(),
                subject: grp,
            };
            // Present arm: the pending insert answers through the delta.
            tx.insert(&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "sow",
            })?;
            assert_eq!(
                tx.get(key)?,
                Some(Task {
                    id: task,
                    kind: Kind::Alpha.id(),
                    subject: grp,
                    note: "sow",
                })
            );
            // Absent arm: the pending delete answers None.
            tx.delete(&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "sow",
            })?;
            assert_eq!(tx.get(key)?, None);
            // Reinsert modified: the key re-establishes on the final state.
            tx.insert(&Task {
                id: task,
                kind: Kind::Alpha.id(),
                subject: grp,
                note: "harvest",
            })?;
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
        .expect("pre-commit overlay reads");

    // Every pre-commit answer equals the post-commit read.
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
