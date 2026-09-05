//! D01: decoded-row charges cannot escape the payload. Reservations
//! transfer with the owner, refund exactly once, and every early error
//! path refunds — the ownership rule under C2 for the canonical codec.

use super::*;
use crate::schema::FieldDescriptor;
use crate::work::{ExecutionPolicy, Resource};
use std::time::Duration;

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 1 << 20,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 1000,
        work_units: 1 << 20,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("policy starts")
}

fn text_fields() -> Vec<FieldDescriptor> {
    vec![
        FieldDescriptor {
            name: "id".into(),
            value_type: crate::schema::ValueType::U64,
        },
        FieldDescriptor {
            name: "text".into(),
            value_type: crate::schema::ValueType::String,
        },
    ]
}

fn encoded_row(work: &WorkContext) -> Vec<u8> {
    CanonicalRow::encode(
        &text_fields(),
        &[Value::U64(7), Value::String("hello canonical".into())],
        work,
    )
    .expect("encodes")
    .as_bytes()
    .to_vec()
}

/// `into_owner` transfers the charge with the values: the working bytes
/// stay reserved while the owner lives past the decode scope, and refund
/// EXACTLY once — when the transferred owner drops.
#[test]
fn d01_decoded_owner_carries_charge_until_release() {
    let work = work();
    let bytes = encoded_row(&work);
    let baseline = work.used(Resource::WorkingBytes);
    let decoded = decode(&text_fields(), &bytes, &work).expect("decodes");
    let charged = work.used(Resource::WorkingBytes);
    assert!(charged > baseline, "decode reserves the decoded footprint");
    assert_eq!(decoded.charged_bytes(), charged - baseline);
    let owner = decoded.into_owner();
    assert_eq!(
        work.used(Resource::WorkingBytes),
        charged,
        "the transfer moves the charge, it does not refund it"
    );
    assert_eq!(owner.values()[0], Value::U64(7));
    drop(owner);
    assert_eq!(
        work.used(Resource::WorkingBytes),
        baseline,
        "exactly one refund, at owner drop"
    );
}

/// Sensitivity: transferring the owner must not charge a second time.
/// A defective double-charge implementation would leave `used == 2 * once`.
#[test]
fn d01_decode_transfer_charges_once_not_twice() {
    let work = work();
    let bytes = encoded_row(&work);
    let baseline = work.used(Resource::WorkingBytes);
    let decoded = decode(&text_fields(), &bytes, &work).expect("decodes");
    let once = work.used(Resource::WorkingBytes) - baseline;
    let moved = decoded.into_owner();
    assert_eq!(
        work.used(Resource::WorkingBytes) - baseline,
        once,
        "sensitivity: owner move charges once, not twice"
    );
    drop(moved);
    assert_eq!(work.used(Resource::WorkingBytes), baseline);
}

/// A zero working allowance refuses decode before owning values.
#[test]
fn d01_decode_refuses_before_owning_values() {
    let ctx = ExecutionPolicy {
        input_bytes: 1 << 20,
        working_bytes: 0,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 1000,
        work_units: 1 << 20,
        timeout: Duration::from_secs(60),
    }
    .start()
    .expect("start");
    let bytes = {
        let donor = work();
        encoded_row(&donor)
    };
    assert!(decode(&text_fields(), &bytes, &ctx).is_err());
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
}

/// Early decode errors refund before returning: a malformed row charges
/// nothing durable.
#[test]
fn decode_errors_refund_their_reservation() {
    let work = work();
    let mut bytes = encoded_row(&work);
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF; // corrupt the tail: trailing/UTF-8 refusal
    let baseline = work.used(Resource::WorkingBytes);
    let error = decode(&text_fields(), &bytes, &work).expect_err("malformed");
    assert!(
        !matches!(error, RowError::Work(_)),
        "a shape error, not work"
    );
    assert_eq!(
        work.used(Resource::WorkingBytes),
        baseline,
        "the failed decode holds no working bytes"
    );
}

/// Encode errors refund too — a refused row leaves the ledger untouched.
#[test]
fn encode_errors_leave_no_charge() {
    let work = work();
    let baseline = work.used(Resource::WorkingBytes);
    let error = CanonicalRow::encode(
        &text_fields(),
        &[Value::U64(7)], // arity mismatch
        &work,
    )
    .expect_err("wrong arity");
    assert!(matches!(error, RowError::Arity));
    assert_eq!(work.used(Resource::WorkingBytes), baseline);
}

/// A `CanonicalRow`'s reservation lives exactly as long as its bytes.
#[test]
fn canonical_row_refunds_on_drop() {
    let work = work();
    let baseline = work.used(Resource::WorkingBytes);
    let row = CanonicalRow::encode(
        &text_fields(),
        &[Value::U64(1), Value::String("x".repeat(4096).into())],
        &work,
    )
    .expect("encodes");
    assert!(work.used(Resource::WorkingBytes) >= baseline + 4096);
    drop(row);
    assert_eq!(work.used(Resource::WorkingBytes), baseline);
}

/// `fact_sort_key` returns the charged row owner, not detached bytes.
#[test]
fn d01_fact_sort_key_retains_charge() {
    let work = work();
    let baseline = work.used(Resource::WorkingBytes);
    let key = fact_sort_key(
        &text_fields(),
        &[Value::U64(1), Value::String("k".into())],
        &work,
    )
    .expect("key");
    assert!(work.used(Resource::WorkingBytes) > baseline);
    assert!(!key.as_bytes().is_empty());
    drop(key);
    assert_eq!(work.used(Resource::WorkingBytes), baseline);
}

#[test]
fn d01_canonical_row_as_ref_matches_as_bytes() {
    let work = work();
    let row = CanonicalRow::encode(
        &text_fields(),
        &[Value::U64(1), Value::String("as-ref".into())],
        &work,
    )
    .expect("encodes");
    assert_eq!(row.as_ref(), row.as_bytes());
}
