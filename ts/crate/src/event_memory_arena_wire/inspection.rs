//! Owned export data, bounded cumulatively before JS delivery.
use crate::event_inspection::{Export, Inspection};
use crate::runtime::RuntimeError;
use bumbledb::event::{
    AdmittedDescriptor, BeliefArena, BeliefSpaceIds, DescriptorLimits, ReachStrategy,
    SafetyStrategy,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

pub(super) fn arena(
    value: &BeliefArena,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    let mut out = Export::new("arena", limits, work)?;
    out.event("states", &value.arena().states().full())?;
    out.event("choices", &value.arena().choices().full())?;
    out.event("initial", value.initial())?;
    out.descriptor(
        "actions",
        &AdmittedDescriptor::Fibre(value.arena().actions().clone()),
    )?;
    out.descriptor(
        "transition",
        &AdmittedDescriptor::Relation(value.arena().transition().clone()),
    )?;
    out.list("stateCodes", value.memory().states().len(), |i| {
        value.state_code(i, work)
    })?;
    out.list("actionCodes", value.memory().actions().len(), |i| {
        value.action_code(i, work)
    })?;
    out.finish()
}
pub(super) fn reach(
    value: &ReachStrategy,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    let mut out = Export::new("reach", limits, work)?;
    out.event("goal", value.goal())?;
    out.event("winning", value.winning())?;
    out.descriptor(
        "policy",
        &AdmittedDescriptor::Relation(value.policy().clone()),
    )?;
    out.list("ranks", value.ranked().layers().cells().len(), |i| {
        Ok(value.ranked().layers().cells()[i].clone())
    })?;
    out.finish()
}
pub(super) fn safe(
    value: &SafetyStrategy,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    let mut out = Export::new("safe", limits, work)?;
    out.event("invariant", value.invariant())?;
    out.event("winning", value.winning())?;
    out.descriptor(
        "policy",
        &AdmittedDescriptor::Relation(value.policy().clone()),
    )?;
    out.finish()
}
pub(super) fn description(
    env: &Env,
    memory: Vec<u8>,
    ids: BeliefSpaceIds,
) -> napi::Result<Object<'_>> {
    let mut out = Object::new(env)?;
    out.set("memory", Uint8Array::from(memory))?;
    let mut names = Object::new(env)?;
    for (name, id) in [
        ("states", ids.states),
        ("actions", ids.actions),
        ("environment", ids.environment),
        ("stateActions", ids.state_actions),
        ("transitions", ids.transitions),
    ] {
        names.set(name, Uint8Array::from(id.0.to_vec()))?;
    }
    out.set("identities", names)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{
        ArithmeticLimits, BeliefArenaDescriptor, BeliefDescriptor, BeliefDescriptorLimits,
        ExactArithmetic, SpaceId,
    };
    #[test]
    fn compiled_inspection_bounds_all_fields_and_codes_together() {
        let work = WorkContext::new();
        let (input, _) = super::super::tests::input();
        let memory =
            BeliefDescriptor::from_bytes(&input[0], DescriptorLimits::default(), &work).unwrap();
        let data = BeliefArenaDescriptor {
            memory,
            identities: BeliefSpaceIds {
                states: SpaceId([1; 32]),
                actions: SpaceId([2; 32]),
                environment: SpaceId([3; 32]),
                state_actions: SpaceId([4; 32]),
                transitions: SpaceId([5; 32]),
            },
        };
        let value = data
            .admit(
                BeliefDescriptorLimits::default(),
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
            )
            .unwrap();
        let result = arena(&value, DescriptorLimits::default(), &work).unwrap();
        assert_eq!(result.fields.len(), 5);
        assert_eq!(result.lists.len(), 2);
        for limits in [
            DescriptorLimits {
                bytes: 1,
                ..DescriptorLimits::default()
            },
            DescriptorLimits {
                items: 4,
                ..DescriptorLimits::default()
            },
        ] {
            assert!(matches!(
                arena(&value, limits, &work),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
        work.cancel();
        assert!(arena(&value, DescriptorLimits::default(), &work).is_err());
    }
}
