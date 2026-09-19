//! Bounded detached graph inspection; worker output never borrows arena roots.
use crate::ingress::event_error;
use crate::marshal::output_vec;
use crate::runtime::RuntimeError;
use bumbledb::event::{BeliefMemory, BeliefTransition, Capacity, DescriptorLimits, Error};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Env, Object, Uint8Array};

pub struct Inspection {
    initial: Vec<Option<u32>>,
    states: Vec<State>,
}
struct State {
    possible: Vec<u8>,
    observation: u32,
    transitions: Vec<BeliefTransition>,
}

pub(super) fn inspect(
    memory: &BeliefMemory,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    let mut bytes = limits.bytes;
    let mut items = limits.items;
    let mut reserve = |count: usize, len: usize| -> Result<(), RuntimeError> {
        work.checkpoint()?;
        items = items
            .checked_sub(count)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorItems)))?;
        bytes = bytes
            .checked_sub(len)
            .ok_or_else(|| event_error(Error::Capacity(Capacity::DescriptorBytes)))?;
        Ok(())
    };
    reserve(1, 0)?;
    reserve(
        memory.initial().len(),
        memory.initial().len().saturating_mul(8),
    )?;
    let mut initial = output_vec(memory.initial().len())?;
    for &state in memory.initial() {
        initial.push(
            state
                .map(|i| u32::try_from(i).map_err(|_| RuntimeError::InvalidArgument))
                .transpose()?,
        );
    }
    reserve(memory.states().len(), 0)?;
    let mut states = output_vec(memory.states().len())?;
    for state in memory.states() {
        let possible = state.possible().to_bytes(work).map_err(event_error)?;
        reserve(1, possible.len())?;
        reserve(
            state.transitions().len(),
            state.transitions().len().saturating_mul(24),
        )?;
        let mut transitions = output_vec(state.transitions().len())?;
        for edge in state.transitions() {
            work.checkpoint()?;
            // JS indices are exact and bounded by the native admission policy.
            for index in [edge.action, edge.observation, edge.target] {
                u32::try_from(index).map_err(|_| RuntimeError::InvalidArgument)?;
            }
            transitions.push(*edge);
        }
        states.push(State {
            possible,
            observation: u32::try_from(state.observation())
                .map_err(|_| RuntimeError::InvalidArgument)?,
            transitions,
        });
    }
    work.checkpoint()?;
    Ok(Inspection { initial, states })
}

impl Inspection {
    pub(super) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("initial", self.initial)?;
        let mut states =
            output_vec(self.states.len()).map_err(|e| crate::runtime_wire::thrown(*env, e))?;
        for state in self.states {
            let mut value = Object::new(env)?;
            value.set("possible", Uint8Array::from(state.possible))?;
            value.set("observation", state.observation)?;
            let mut edges = output_vec(state.transitions.len())
                .map_err(|e| crate::runtime_wire::thrown(*env, e))?;
            for edge in state.transitions {
                let mut value = Object::new(env)?;
                value.set(
                    "action",
                    u32::try_from(edge.action).expect("bounded on worker"),
                )?;
                value.set(
                    "observation",
                    u32::try_from(edge.observation).expect("bounded on worker"),
                )?;
                value.set(
                    "target",
                    u32::try_from(edge.target).expect("bounded on worker"),
                )?;
                edges.push(value);
            }
            value.set("transitions", edges)?;
            states.push(value);
        }
        object.set("states", states)?;
        Ok(object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{BeliefLimits, EventPartition, PartitionLimits, Space, SpaceId};
    #[test]
    fn graph_inspection_bounds_combined_output_and_retains_empty_observations() {
        let work = WorkContext::new();
        let source = Space::new(SpaceId([91; 32]), 0, &work).unwrap();
        let observed = EventPartition::on(
            &source.full(),
            &[source.empty(), source.full()],
            PartitionLimits::default(),
            &work,
        )
        .unwrap();
        let memory = BeliefMemory::new(
            &[],
            &observed,
            &source.full(),
            BeliefLimits::default(),
            &work,
        )
        .unwrap();
        let output = inspect(&memory, DescriptorLimits::default(), &work).unwrap();
        assert_eq!(output.initial, [None, Some(0)]);
        assert_eq!(output.states.len(), 1);
        assert_eq!(output.states[0].observation, 1);
        for limits in [
            DescriptorLimits {
                bytes: 1,
                ..DescriptorLimits::default()
            },
            DescriptorLimits {
                items: 1,
                ..DescriptorLimits::default()
            },
        ] {
            assert!(matches!(
                inspect(&memory, limits, &work),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
        work.cancel();
        assert!(inspect(&memory, DescriptorLimits::default(), &work).is_err());
    }
}
