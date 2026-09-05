//! F3 finding C regressions for the byte ledger itself: a reservation is
//! linear — it charges once, travels with its owner (across clones of the
//! context and past the operation), and refunds exactly once, at drop.

use super::*;

fn policy(working: u64) -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 0,
        working_bytes: working,
        scratch_bytes: 1024,
        result_bytes: 0,
        rows: 0,
        work_units: 1024,
        timeout: Duration::from_secs(60),
    }
}

#[test]
fn a_reservation_charges_once_and_refunds_once() {
    let work = policy(1000).start().expect("start");
    let reservation = work.reserve(ByteKind::Working, 600).expect("reserve");
    assert_eq!(work.used(Resource::WorkingBytes), 600);
    assert_eq!(reservation.bytes(), 600);
    // The remaining allowance refuses what no longer fits…
    assert!(work.reserve(ByteKind::Working, 500).is_err());
    // …and the refund happens exactly at drop, exactly once.
    drop(reservation);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
    let again = work.reserve(ByteKind::Working, 1000).expect("full again");
    drop(again);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
}

/// Clones share the ledger; a reservation taken through one clone stays
/// charged when other clones (and even the original) are gone — the charge
/// belongs to the allocation's owner, not to any context handle.
#[test]
fn reservations_outlive_every_context_handle() {
    let work = policy(1000).start().expect("start");
    let clone = work.clone();
    let reservation = clone.reserve(ByteKind::Working, 400).expect("reserve");
    drop(clone);
    assert_eq!(work.used(Resource::WorkingBytes), 400);
    drop(reservation);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
}

/// A refused reservation charges nothing: the failed attempt leaves the
/// counter exactly where it was (early-error accounting).
#[test]
fn a_refused_reservation_charges_nothing() {
    let work = policy(100).start().expect("start");
    let held = work.reserve(ByteKind::Working, 90).expect("reserve");
    let error = work.reserve(ByteKind::Working, 20).expect_err("refused");
    assert!(matches!(
        error,
        WorkError::Exhausted {
            resource: Resource::WorkingBytes,
            used: 90,
            requested: 20,
            limit: 100,
        }
    ));
    assert_eq!(work.used(Resource::WorkingBytes), 90);
    drop(held);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
}

/// Distinct byte kinds are distinct dimensions: a scratch reservation never
/// touches the working counter and vice versa.
#[test]
fn byte_kinds_charge_their_own_dimension() {
    let work = policy(1000).start().expect("start");
    let scratch = work.reserve(ByteKind::Scratch, 512).expect("scratch");
    assert_eq!(work.used(Resource::ScratchBytes), 512);
    assert_eq!(work.used(Resource::WorkingBytes), 0);
    drop(scratch);
    assert_eq!(work.used(Resource::ScratchBytes), 0);
}

/// Charged owners keep payload and charge inseparable: dropping the owner
/// refunds exactly once; there is no charge-shedding extraction.
#[test]
fn charged_bytes_keep_charge_with_payload() {
    use super::owners::ChargedBytes;
    let work = policy(4096).start().expect("start");
    let baseline = work.used(Resource::WorkingBytes);
    let payload = Box::from(*b"owned payload");
    let charged = ChargedBytes::adopt(&work, ByteKind::Working, payload).expect("adopt");
    assert_eq!(
        work.used(Resource::WorkingBytes),
        baseline + charged.charged_bytes()
    );
    drop(charged);
    assert_eq!(work.used(Resource::WorkingBytes), baseline);
}
