//! Authenticated decision-chain traversal (C6).
//!
//! One streaming walker visits parent locators only — no epoch-probing
//! fallback and no whole-tail `Vec` return. A missing required interior
//! link before the verified base is corruption-class refusal. Walk stops at
//! the captured base without fetching another object. A budget of `n`
//! admits at most `n` fetches, including `n = 1`.

use bumbledb::WorkContext;

use crate::history::command::Limits;
use crate::history::decision;
use crate::history::{DecisionStamp, FrameError};
use crate::store::{
    fetch_decision_ref, BackendError, ObjectError, ObjectKind, ObjectRef, ObservedError,
    ReceiveLimits, ReceivingStore, TransportContext,
};

/// Intersect the caller's envelope with the locator's declared length.
/// Never `u64::MAX`.
#[must_use]
pub fn receive_limits_for_object(reference: &ObjectRef, envelope_bytes: usize) -> ReceiveLimits {
    ReceiveLimits::capped((envelope_bytes as u64).min(reference.length))
}

fn work_object_error(error: bumbledb::WorkError) -> ObjectError {
    ObjectError::Backend(Box::new(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("{error:?}"),
    )))
}

/// ObjectRef wire width: 8 epoch + 1 kind + 32 digest + 8 length (C6).
pub const OBJECT_REF_WIRE_BYTES: usize = 49;
/// Option tag only: absent ObjectRef (C6).
pub const OBJECT_REF_OPTION_ABSENT_BYTES: usize = 1;
/// One tag + one ObjectRef (C6). Never 51 (extra tag) or 45.
pub const OBJECT_REF_OPTION_PRESENT_BYTES: usize =
    OBJECT_REF_OPTION_ABSENT_BYTES + OBJECT_REF_WIRE_BYTES;

/// Encoded size of an optional ObjectRef field: absent 1, present 50.
#[must_use]
pub const fn object_ref_option_bytes(present: bool) -> usize {
    if present {
        OBJECT_REF_OPTION_PRESENT_BYTES
    } else {
        OBJECT_REF_OPTION_ABSENT_BYTES
    }
}

/// Parent-bearing decision field width. Derives from the option helper once.
#[must_use]
pub const fn parent_locator_field_bytes(present: bool) -> usize {
    object_ref_option_bytes(present)
}

/// Streaming visitor for one authenticated walk (C6). A whole-tail `Vec`
/// return is not this contract.
pub trait ChainVisitor {
    type Error;

    /// One bounded decision record. Return `false` to stop early.
    fn visit(
        &mut self,
        stamp: DecisionStamp,
        bytes: &[u8],
        reference: ObjectRef,
    ) -> Result<bool, Self::Error>;
}

/// Walk decision objects backward from `cursor` to `base` using authenticated
/// parent locators. Each fetch is [`fetch_decision_ref`] → [`get_verified`]
/// under the caller's [`WorkContext`] and intersected locator/envelope
/// [`ReceiveLimits`]. Decode borrows [`bumbledb::work::ChargedBytes::as_bytes`];
/// the owner is dropped via [`bumbledb::work::ChargedBytes::into_owner`]
/// only after visit.
///
/// Stops at `base` without fetching another object. The caller's tip
/// locator is taken by value and is not overwritten.
///
/// # Errors
/// Missing locators before the base, stamp mismatch, budget exhaustion,
/// or work refusal.
pub fn walk_decision_chain<B, V>(
    backend: &B,
    prefix: &str,
    mut cursor: DecisionStamp,
    base: DecisionStamp,
    mut locator: Option<ObjectRef>,
    limits: Limits,
    budget: &mut u64,
    work: &WorkContext,
    visitor: &mut V,
) -> Result<(), V::Error>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
    V: ChainVisitor,
    V::Error: From<ObjectError>,
{
    while cursor != base {
        if cursor.seq < base.seq || (cursor.seq == base.seq && cursor != base) {
            return Err(ObjectError::Frame(FrameError::InvalidTerminalStamp).into());
        }
        if cursor.seq == 0 {
            return Err(ObjectError::Frame(FrameError::InvalidSequence).into());
        }
        if *budget == 0 {
            return Err(ObjectError::Backend(Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "decision walk budget exhausted",
            )))
            .into());
        }
        *budget -= 1;
        let reference = locator.take().ok_or_else(|| ObjectError::Missing {
            key: format!("parent locator missing before base at seq {}", cursor.seq),
        })?;
        validate_parent_locator(&reference, &cursor)?;
        work.checkpoint().map_err(work_object_error)?;
        let charged = fetch_decision_ref(
            backend,
            prefix,
            &reference,
            TransportContext::new(
                work,
                receive_limits_for_object(&reference, limits.envelope_bytes),
            ),
        )?;
        let envelope =
            decision::decode_decision(charged.as_bytes(), limits).map_err(ObjectError::Frame)?;
        if envelope.stamp() != cursor {
            return Err(ObjectError::WrongDigest {
                key: "decision stamp mismatch".into(),
            }
            .into());
        }
        if let Some(parent_ref) = envelope.parent_object {
            validate_parent_locator(&parent_ref, &envelope.parent)?;
        }
        let continue_walk = visitor.visit(envelope.stamp(), charged.as_bytes(), reference)?;
        locator = envelope.parent_object;
        cursor = envelope.parent;
        drop(charged.into_owner());
        if !continue_walk {
            return Ok(());
        }
    }
    Ok(())
}

/// Compatibility collector over [`walk_decision_chain`]. L08/L10/L14 must
/// migrate to a streaming [`ChainVisitor`]; this helper exists only so those
/// lanes can rename in one step. L09 callers do not materialize the tail.
///
/// # Errors
/// Same refusals as [`walk_decision_chain`].
pub fn walk_decision_chain_collect<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    cursor: DecisionStamp,
    base: DecisionStamp,
    locator: Option<ObjectRef>,
    limits: Limits,
    budget: &mut u64,
    work: &WorkContext,
) -> Result<Vec<(DecisionStamp, Vec<u8>, ObjectRef)>, ObjectError>
where
    B::Error: BackendError + ObservedError,
{
    struct Collect(Vec<(DecisionStamp, Vec<u8>, ObjectRef)>);
    impl ChainVisitor for Collect {
        type Error = ObjectError;
        fn visit(
            &mut self,
            stamp: DecisionStamp,
            bytes: &[u8],
            reference: ObjectRef,
        ) -> Result<bool, ObjectError> {
            self.0.push((stamp, bytes.to_vec(), reference));
            Ok(true)
        }
    }
    let mut collect = Collect(Vec::new());
    walk_decision_chain(
        backend,
        prefix,
        cursor,
        base,
        locator,
        limits,
        budget,
        work,
        &mut collect,
    )?;
    Ok(collect.0)
}

/// A present tip locator must name the tip stamp. `None` is the
/// checkpoint-only case at any sequence, including nonzero (C6). Pairing
/// with `base` is [`validate_recovery_locators`].
pub fn validate_tip_locator(
    tip: DecisionStamp,
    tip_object: Option<ObjectRef>,
) -> Result<(), FrameError> {
    match tip_object {
        None => Ok(()),
        Some(reference) => validate_decision_locator(&reference, &tip),
    }
}

/// Checked recovery-root locators: checkpoint-only `base == tip` with no
/// tip object, or suffix `base != tip` with a DecisionRef bound to the tip
/// stamp. Comparison is the complete stamp, not sequence alone.
pub fn validate_recovery_locators(
    base: DecisionStamp,
    tip: DecisionStamp,
    tip_object: Option<ObjectRef>,
) -> Result<(), FrameError> {
    if tip.seq < base.seq {
        return Err(FrameError::InvalidSequence);
    }
    if base == tip {
        return match tip_object {
            None => Ok(()),
            Some(_) => Err(FrameError::InvalidTerminalStamp),
        };
    }
    if tip.seq == base.seq {
        return Err(FrameError::InvalidTerminalStamp);
    }
    match tip_object {
        None => Err(FrameError::InvalidTerminalStamp),
        Some(reference) => validate_decision_locator(&reference, &tip),
    }
}

/// A present parent locator must name the parent stamp's digest and kind.
pub fn validate_parent_locator(
    reference: &ObjectRef,
    parent: &DecisionStamp,
) -> Result<(), ObjectError> {
    validate_decision_locator(reference, parent).map_err(ObjectError::Frame)
}

fn validate_decision_locator(
    reference: &ObjectRef,
    stamp: &DecisionStamp,
) -> Result<(), FrameError> {
    if reference.kind != ObjectKind::Decision {
        return Err(FrameError::InvalidCount);
    }
    if reference.digest != *stamp.hash.as_bytes() {
        return Err(FrameError::InvalidTerminalStamp);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use super::*;
    use crate::history::command::{CommandMetadata, Limits, UnverifiedOutcome, encode_command};
    use crate::history::decision::{DecisionParts, encode_decision};
    use crate::history::{
        ChangeSummary, CommandId, DatabaseId, DatabaseIdentity, DecisionDigest, IncarnationId,
        ReceiptEpoch, RequestId, SchemaId, StateStamp,
    };
    use crate::store::mem::{MemStore, Op};
    use crate::store::{put_verified, ObjectKind};
    use bumbledb::{ExecutionPolicy, WorkContext};
    use std::time::Duration;

    fn work() -> WorkContext {
        ExecutionPolicy {
            input_bytes: 1 << 20,
            working_bytes: 1 << 20,
            scratch_bytes: 1 << 20,
            result_bytes: 1 << 20,
            rows: 1 << 16,
            work_units: 1_024,
            timeout: Duration::from_secs(30),
        }
        .start()
        .expect("work")
    }

    const LIMITS: Limits = Limits {
        envelope_bytes: 4096,
        change_bytes: 256,
        evidence_bytes: 256,
        result_bytes: 64,
    };

    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        }
    }

    fn state(revision: u64) -> StateStamp {
        StateStamp {
            incarnation: identity().incarnation_id,
            data_revision: revision,
        }
    }

    fn command_bytes() -> Vec<u8> {
        encode_command(
            CommandMetadata {
                identity: identity(),
                id: CommandId {
                    receipt_epoch: ReceiptEpoch::INITIAL,
                    request_id: RequestId::from_core(Id128::from_bytes([4; 16])),
                },
                condition: crate::history::Condition::Unconditional,
            },
            &[0xaa, 0xbb],
            LIMITS,
        )
        .unwrap()
    }

    struct CountingVisitor {
        visits: usize,
        stop_after: Option<usize>,
        refs: Vec<ObjectRef>,
    }

    impl ChainVisitor for CountingVisitor {
        type Error = ObjectError;
        fn visit(
            &mut self,
            _stamp: DecisionStamp,
            _bytes: &[u8],
            reference: ObjectRef,
        ) -> Result<bool, ObjectError> {
            self.visits += 1;
            self.refs.push(reference);
            Ok(self.stop_after.is_none_or(|limit| self.visits < limit))
        }
    }

    #[test]
    fn object_ref_option_sizes_are_one_tag_not_an_extra_tag() {
        assert_eq!(OBJECT_REF_WIRE_BYTES, 49);
        assert_eq!(OBJECT_REF_OPTION_ABSENT_BYTES, 1);
        assert_eq!(OBJECT_REF_OPTION_PRESENT_BYTES, 50);
        assert_eq!(object_ref_option_bytes(false), 1);
        assert_eq!(object_ref_option_bytes(true), 50);
        assert_eq!(parent_locator_field_bytes(true), 50);
        let extra_tag = 1 + 1 + OBJECT_REF_WIRE_BYTES;
        assert_ne!(
            OBJECT_REF_OPTION_PRESENT_BYTES, extra_tag,
            "independent present size must fail the extra-tag formula"
        );
        assert_ne!(OBJECT_REF_WIRE_BYTES, 45);
    }

    #[test]
    fn recovery_locators_refuse_equal_sequence_different_stamp() {
        let base = DecisionStamp {
            seq: 7,
            hash: DecisionDigest::from_bytes([1; 32]),
        };
        let tip = DecisionStamp {
            seq: 7,
            hash: DecisionDigest::from_bytes([2; 32]),
        };
        let tip_object = ObjectRef {
            epoch: 1,
            kind: ObjectKind::Decision,
            digest: [2; 32],
            length: 8,
        };
        assert_eq!(
            validate_recovery_locators(base, tip, Some(tip_object)),
            Err(FrameError::InvalidTerminalStamp)
        );
        assert_eq!(
            validate_recovery_locators(base, base, None),
            Ok(())
        );
        assert_eq!(
            validate_recovery_locators(base, base, Some(tip_object)),
            Err(FrameError::InvalidTerminalStamp)
        );
        assert!(validate_tip_locator(base, None).is_ok());
    }

    fn put_decision(
        store: &MemStore,
        parent: DecisionStamp,
        parent_object: Option<ObjectRef>,
        seq: u64,
    ) -> (DecisionStamp, ObjectRef, Vec<u8>) {
        let command = command_bytes();
        let parts = DecisionParts {
            identity: identity(),
            seq,
            parent,
            parent_object,
            before_state: state(seq - 1),
            after_state: state(seq),
            canonical_command: &command,
            outcome: UnverifiedOutcome::Committed {
                changed: ChangeSummary::new(1, 0).unwrap(),
                result: &[],
            },
        };
        let bytes = encode_decision(parts, LIMITS).unwrap();
        let reference = put_verified(store, "t", 1, ObjectKind::Decision, &bytes).unwrap();
        let stamp = DecisionStamp {
            seq,
            hash: crate::history::decision::decision_digest(&bytes),
        };
        (stamp, reference, bytes)
    }

    #[test]
    fn walker_stops_at_base_without_fetching_it_and_honors_n_equals_one() {
        let store = MemStore::new();
        let genesis = DecisionStamp {
            seq: 0,
            hash: DecisionDigest::from_bytes([9; 32]),
        };
        let (one, ref_one, _) = put_decision(&store, genesis, None, 1);
        let (two, ref_two, _) = put_decision(&store, one, Some(ref_one), 2);
        let (three, ref_three, _) = put_decision(&store, two, Some(ref_two), 3);

        let mut visitor = CountingVisitor {
            visits: 0,
            stop_after: None,
            refs: Vec::new(),
        };
        let mut budget = 8;
        let ctx = work();
        walk_decision_chain(
            &store,
            "t",
            three,
            one,
            Some(ref_three),
            LIMITS,
            &mut budget,
            &ctx,
            &mut visitor,
        )
        .unwrap();
        assert_eq!(visitor.visits, 2, "suffix (1, 3] fetches 3 then 2");
        assert_eq!(visitor.refs, [ref_three, ref_two]);
        let fetched: Vec<_> = store
            .operations()
            .into_iter()
            .filter(|(op, _)| *op == Op::GetObject)
            .map(|(_, key)| key)
            .collect();
        assert_eq!(fetched.len(), 2);
        assert!(fetched.iter().any(|key| key == &ref_three.key("t")));
        assert!(fetched.iter().any(|key| key == &ref_two.key("t")));
        assert!(
            !fetched.iter().any(|key| key == &ref_one.key("t")),
            "must not fetch the captured base"
        );

        let mut one_visit = CountingVisitor {
            visits: 0,
            stop_after: None,
            refs: Vec::new(),
        };
        let mut n_one = 1;
        walk_decision_chain(
            &store,
            "t",
            three,
            one,
            Some(ref_three),
            LIMITS,
            &mut n_one,
            &ctx,
            &mut one_visit,
        )
        .expect_err("budget 1 cannot walk two links");
        assert_eq!(one_visit.visits, 1, "n=1 admits exactly one fetch");

        let mut exact = CountingVisitor {
            visits: 0,
            stop_after: None,
            refs: Vec::new(),
        };
        let mut n_two = 2;
        walk_decision_chain(
            &store,
            "t",
            three,
            one,
            Some(ref_three),
            LIMITS,
            &mut n_two,
            &ctx,
            &mut exact,
        )
        .unwrap();
        assert_eq!(exact.visits, 2);
        assert_eq!(n_two, 0);
    }

    #[test]
    fn missing_interior_locator_and_malformed_parent_kind_refuse() {
        let store = MemStore::new();
        let genesis = DecisionStamp {
            seq: 0,
            hash: DecisionDigest::from_bytes([9; 32]),
        };
        let (one, ref_one, _) = put_decision(&store, genesis, None, 1);
        let (two, ref_two, _) = put_decision(&store, one, Some(ref_one), 2);
        let mut visitor = CountingVisitor {
            visits: 0,
            stop_after: None,
            refs: Vec::new(),
        };
        let mut budget = 8;
        let ctx = work();
        let missing = walk_decision_chain(
            &store,
            "t",
            two,
            genesis,
            None,
            LIMITS,
            &mut budget,
            &ctx,
            &mut visitor,
        );
        assert!(matches!(missing, Err(ObjectError::Missing { .. })));

        let wrong_kind = ObjectRef {
            epoch: 1,
            kind: ObjectKind::Chunk,
            digest: *two.hash.as_bytes(),
            length: 8,
        };
        assert!(validate_parent_locator(&wrong_kind, &two).is_err());
        let wrong_digest = ObjectRef {
            epoch: 1,
            kind: ObjectKind::Decision,
            digest: [0x11; 32],
            length: ref_two.length,
        };
        assert!(validate_parent_locator(&wrong_digest, &two).is_err());
    }

    /// Receive charge stays live through decode/visit; it is not dropped
    /// before the body is interpreted, and it refunds on cleanup.
    #[test]
    fn walker_keeps_receive_charge_through_decode_and_refunds_after() {
        use bumbledb::work::Resource;

        let store = MemStore::new();
        let genesis = DecisionStamp {
            seq: 0,
            hash: DecisionDigest::from_bytes([9; 32]),
        };
        let (one, ref_one, _) = put_decision(&store, genesis, None, 1);
        let (two, ref_two, bytes_two) = put_decision(&store, one, Some(ref_one), 2);
        let ctx = work();
        let baseline = ctx.used(Resource::WorkingBytes);
        struct ChargeProbe<'a> {
            work: &'a WorkContext,
            baseline: u64,
            saw_charge: bool,
        }
        impl ChainVisitor for ChargeProbe<'_> {
            type Error = ObjectError;
            fn visit(
                &mut self,
                _stamp: DecisionStamp,
                bytes: &[u8],
                _reference: ObjectRef,
            ) -> Result<bool, ObjectError> {
                let used = self.work.used(Resource::WorkingBytes);
                assert!(
                    used >= self.baseline + bytes.len() as u64,
                    "reservation must outlive decode into visit: used={used} baseline={} body={}",
                    self.baseline,
                    bytes.len()
                );
                self.saw_charge = true;
                Ok(true)
            }
        }
        let mut probe = ChargeProbe {
            work: &ctx,
            baseline,
            saw_charge: false,
        };
        let mut budget = 2;
        walk_decision_chain(
            &store,
            "t",
            two,
            one,
            Some(ref_two),
            LIMITS,
            &mut budget,
            &ctx,
            &mut probe,
        )
        .unwrap();
        assert!(probe.saw_charge);
        assert_eq!(
            ctx.used(Resource::WorkingBytes),
            baseline,
            "walk cleanup refunds the receive charge"
        );
        let limits = receive_limits_for_object(&ref_two, LIMITS.envelope_bytes);
        assert_eq!(limits.max_bytes, ref_two.length.min(LIMITS.envelope_bytes as u64));
        assert_ne!(limits.max_bytes, u64::MAX);
        assert!(!bytes_two.is_empty());
    }
}
