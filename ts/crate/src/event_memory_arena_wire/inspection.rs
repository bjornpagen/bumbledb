//! Owned export data, bounded cumulatively before JS delivery.
use crate::ingress::event_error;
use crate::marshal::output_vec;
use crate::runtime::RuntimeError;
use bumbledb::event::{
    AdmittedDescriptor, BeliefArena, BeliefSpaceIds, Capacity, Descriptor, DescriptorLimits, Error,
    Event, ReachStrategy, SafetyStrategy,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

pub struct Inspection {
    kind: &'static str,
    fields: Vec<(&'static str, Vec<u8>)>,
    lists: Vec<(&'static str, Vec<Vec<u8>>)>,
}
struct Export<'a> {
    value: Inspection,
    limits: DescriptorLimits,
    bytes: usize,
    items: usize,
    work: &'a WorkContext,
}
impl<'a> Export<'a> {
    fn new(
        kind: &'static str,
        limits: DescriptorLimits,
        work: &'a WorkContext,
    ) -> Result<Self, RuntimeError> {
        let mut result = Self {
            value: Inspection {
                kind,
                fields: Vec::new(),
                lists: Vec::new(),
            },
            limits,
            bytes: limits.bytes,
            items: limits.items,
            work,
        };
        result.reserve(0)?;
        Ok(result)
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), RuntimeError> {
        self.work.checkpoint()?;
        self.items = self
            .items
            .checked_sub(1)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorItems)))?;
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorBytes)))?;
        Ok(())
    }
    fn field(&mut self, name: &'static str, bytes: Vec<u8>) -> Result<(), RuntimeError> {
        self.reserve(bytes.len())?;
        self.value
            .fields
            .try_reserve(1)
            .map_err(|_| RuntimeError::Work(bumbledb::work::WorkError::Allocation))?;
        self.value.fields.push((name, bytes));
        Ok(())
    }
    fn event(&mut self, name: &'static str, event: &Event) -> Result<(), RuntimeError> {
        self.field(name, event.to_bytes(self.work).map_err(event_error)?)
    }
    fn descriptor(
        &mut self,
        name: &'static str,
        value: &AdmittedDescriptor,
    ) -> Result<(), RuntimeError> {
        let bytes = Descriptor::capture(value, self.limits, self.work)
            .map_err(event_error)?
            .to_bytes(self.limits, self.work)
            .map_err(event_error)?;
        self.field(name, bytes)
    }
    fn list(
        &mut self,
        name: &'static str,
        len: usize,
        event: impl Fn(usize) -> bumbledb::event::Result<Event>,
    ) -> Result<(), RuntimeError> {
        self.reserve(0)?;
        if len > self.items {
            return Err(event_error(Error::Capacity(Capacity::DescriptorItems)));
        }
        let mut values = output_vec(len)?;
        for i in 0..len {
            let value = event(i)
                .map_err(event_error)?
                .to_bytes(self.work)
                .map_err(event_error)?;
            self.reserve(value.len())?;
            values.push(value);
        }
        self.value
            .lists
            .try_reserve(1)
            .map_err(|_| RuntimeError::Work(bumbledb::work::WorkError::Allocation))?;
        self.value.lists.push((name, values));
        Ok(())
    }
    fn finish(self) -> Result<Inspection, RuntimeError> {
        self.work.checkpoint()?;
        Ok(self.value)
    }
}
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
impl Inspection {
    pub(super) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("kind", self.kind)?;
        for (name, bytes) in self.fields {
            object.set(name, Uint8Array::from(bytes))?;
        }
        for (name, list) in self.lists {
            let mut values =
                output_vec(list.len()).map_err(|e| crate::runtime_wire::thrown(*env, e))?;
            values.extend(list.into_iter().map(Uint8Array::from));
            object.set(name, values)?;
        }
        Ok(object)
    }
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
