//! Apply: the refusal battery, the three proved chain causes, the
//! state-change instrument, and the idempotent crash-window
//! absorption.

mod lane_d_support;

use bumbledb::{Db, Value};
use bumbledb_log::apply::{apply, Applied, ApplyRefusal, ChainCause};
use bumbledb_log::codec::{BatchHeader, Op, OpKind};
use bumbledb_log::sidecar::{Chain, ChainEntry};
use lane_d_support::{
    codec, insert_note, insert_recipe, insert_step, kitchen_braid, note_braid, temp_dir, theory,
    NOTE, RECIPE,
};

fn fresh_db(tag: &str) -> Db<bumbledb::SchemaDescriptor> {
    let dir = temp_dir(tag).join("db");
    Db::create(&dir, theory())
        .expect("create")
        .expect("theory admits empty store")
}

fn header(braid: bumbledb_log::braids::BraidId, slot: u64, prev: [u8; 32], ts: u64) -> BatchHeader {
    BatchHeader {
        fingerprint: *codec().fingerprint(),
        braid,
        braid_gen: slot,
        prev,
        writer: 42,
        timestamp: ts,
    }
}

#[test]
fn first_apply_advances_and_moves_the_chain() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_advance");
    let mut chain = Chain::genesis(codec.braids());

    let bytes = codec
        .encode(&header(braid, 1, [0u8; 32], 500), &[insert_recipe(1)])
        .expect("encode");
    let applied = apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply");
    assert_eq!(applied, Applied::Advanced { generation: 1 });
    let position = chain.position(braid);
    assert_eq!(position.g, 1);
    assert_eq!(position.prev, *blake3::hash(&bytes).as_bytes());
    assert_eq!(position.ts, 500);
    assert!(db
        .read(|instance| instance.contains_dyn(RECIPE, &[Value::U64(1)]))
        .expect("read"));
}

#[test]
fn crash_window_reapply_is_absorbed_with_the_identity_exact() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_absorb");
    let mut chain = Chain::genesis(codec.braids());

    let bytes = codec
        .encode(&header(braid, 1, [0u8; 32], 500), &[insert_recipe(7)])
        .expect("encode");
    assert_eq!(
        apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply"),
        Applied::Advanced { generation: 1 }
    );

    // The crash window: the engine committed but the sidecar bump was
    // lost. Recovery re-applies the same slot from a rewound chain.
    let mut rewound = Chain::genesis(codec.braids());
    assert_eq!(
        apply(&db, &mut rewound, &codec, braid, 1, &bytes).expect("reapply"),
        Applied::Absorbed { generation: 1 }
    );
    assert_eq!(rewound.position(braid).g, 1);
    assert_eq!(db.generation().expect("generation").value(), 1);
}

#[test]
fn slot_cause_convicts_header_key_disagreement() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_slot");
    let mut chain = Chain::genesis(codec.braids());

    let bytes = codec
        .encode(&header(braid, 2, [0u8; 32], 500), &[insert_recipe(1)])
        .expect("encode");
    match apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply") {
        Applied::Refused(ApplyRefusal::ChainMismatch {
            cause: ChainCause::Slot { header_gen, .. },
            slot,
            writer,
            ..
        }) => {
            assert_eq!(header_gen, 2);
            assert_eq!(slot, 1);
            assert_eq!(writer, 42);
        }
        other => panic!("expected the slot cause, got {other:?}"),
    }
    assert_eq!(chain.position(braid).g, 0, "a refusal never advances");
}

#[test]
fn prev_cause_convicts_a_wrong_base() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_prev");
    let mut chain = Chain::genesis(codec.braids());

    let bytes = codec
        .encode(&header(braid, 1, [0x77; 32], 500), &[insert_recipe(1)])
        .expect("encode");
    match apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply") {
        Applied::Refused(ApplyRefusal::ChainMismatch {
            cause:
                ChainCause::Prev {
                    header_prev,
                    chain_prev,
                },
            ..
        }) => {
            assert_eq!(header_prev, [0x77; 32]);
            assert_eq!(chain_prev, [0u8; 32]);
        }
        other => panic!("expected the prev cause, got {other:?}"),
    }
}

#[test]
fn timestamp_cause_convicts_a_clock_that_ran_backward() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_ts");
    let mut chain = Chain::genesis(codec.braids());
    chain.entries_mut().insert(
        braid,
        ChainEntry {
            g: 0,
            prev: [0u8; 32],
            ts: 900,
        },
    );

    let bytes = codec
        .encode(&header(braid, 1, [0u8; 32], 500), &[insert_recipe(1)])
        .expect("encode");
    match apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply") {
        Applied::Refused(ApplyRefusal::ChainMismatch {
            cause:
                ChainCause::Timestamp {
                    header_ts,
                    chain_ts,
                },
            ..
        }) => {
            assert_eq!(header_ts, 500);
            assert_eq!(chain_ts, 900);
        }
        other => panic!("expected the timestamp cause, got {other:?}"),
    }
}

#[test]
fn decode_refusals_surface_as_the_battery() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_decode");
    let mut chain = Chain::genesis(codec.braids());

    let mut bytes = codec
        .encode(&header(braid, 1, [0u8; 32], 500), &[insert_recipe(1)])
        .expect("encode");
    bytes[4] = 9;
    match apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply") {
        Applied::Refused(ApplyRefusal::Decode(error)) => {
            assert_eq!(error.identity(), "Version");
        }
        other => panic!("expected a decode refusal, got {other:?}"),
    }
}

#[test]
fn a_first_applied_net_noop_is_a_publish_law_violation() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_publish_law");
    let mut chain = Chain::genesis(codec.braids());

    let first = codec
        .encode(&header(braid, 1, [0u8; 32], 500), &[insert_recipe(3)])
        .expect("encode");
    assert_eq!(
        apply(&db, &mut chain, &codec, braid, 1, &first).expect("apply"),
        Applied::Advanced { generation: 1 }
    );

    // A dishonest slot 2 carrying effects slot 1 already delivered.
    let second = codec
        .encode(
            &header(braid, 2, *blake3::hash(&first).as_bytes(), 600),
            &[insert_recipe(3)],
        )
        .expect("encode");
    match apply(&db, &mut chain, &codec, braid, 2, &second).expect("apply") {
        Applied::Refused(ApplyRefusal::PublishLawViolation {
            writer,
            generation,
            identity,
            slot,
            ..
        }) => {
            assert_eq!(writer, 42);
            assert_eq!(slot, 2);
            assert_eq!(generation, 1);
            assert_eq!(identity, 2);
        }
        other => panic!("expected the publish-law instrument, got {other:?}"),
    }
    assert_eq!(chain.position(braid).g, 1, "the violation never advances");
}

#[test]
fn an_engine_rejection_is_data_for_the_caller() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_rejected");
    let mut chain = Chain::genesis(codec.braids());

    let bytes = codec
        .encode(
            &header(braid, 1, [0u8; 32], 500),
            &[insert_step(99, "stir")],
        )
        .expect("encode");
    match apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply") {
        Applied::Rejected(_) => {}
        other => panic!("expected the engine rejection, got {other:?}"),
    }
    assert_eq!(chain.position(braid).g, 0, "a rejection never advances");
    assert_eq!(db.generation().expect("generation").value(), 0);
}

#[test]
fn ops_apply_in_listed_order_within_one_write() {
    let codec = codec();
    let braid = kitchen_braid(&codec);
    let db = fresh_db("apply_order");
    let mut chain = Chain::genesis(codec.braids());

    // Insert then delete of the same row nets to nothing only if both
    // land inside one transaction in listed order; the batch also
    // carries a surviving row so the commit still changes state.
    let ops = [
        insert_recipe(1),
        insert_recipe(2),
        Op {
            kind: OpKind::Delete,
            relation: RECIPE,
            rows: vec![Box::from([Value::U64(2)])],
        },
    ];
    let bytes = codec
        .encode(&header(braid, 1, [0u8; 32], 500), &ops)
        .expect("encode");
    assert_eq!(
        apply(&db, &mut chain, &codec, braid, 1, &bytes).expect("apply"),
        Applied::Advanced { generation: 1 }
    );
    db.read(|instance| {
        assert!(instance.contains_dyn(RECIPE, &[Value::U64(1)])?);
        assert!(!instance.contains_dyn(RECIPE, &[Value::U64(2)])?);
        Ok(())
    })
    .expect("read");
}

#[test]
fn braids_apply_independently() {
    let codec = codec();
    let db = fresh_db("apply_braids");
    let mut chain = Chain::genesis(codec.braids());

    let note = codec
        .encode(
            &header(note_braid(&codec), 1, [0u8; 32], 300),
            &[insert_note(1, "remember the salt")],
        )
        .expect("encode");
    let kitchen = codec
        .encode(
            &header(kitchen_braid(&codec), 1, [0u8; 32], 400),
            &[insert_recipe(1)],
        )
        .expect("encode");
    assert_eq!(
        apply(&db, &mut chain, &codec, note_braid(&codec), 1, &note).expect("apply"),
        Applied::Advanced { generation: 1 }
    );
    assert_eq!(
        apply(&db, &mut chain, &codec, kitchen_braid(&codec), 1, &kitchen).expect("apply"),
        Applied::Advanced { generation: 2 }
    );
    assert_eq!(chain.sum(), 2);
    assert!(db
        .read(|instance| instance.contains_dyn(
            NOTE,
            &[Value::U64(1), Value::String("remember the salt".into())]
        ))
        .expect("read"));
}
