//! Compiled possibility memory on the shared native executor. All controller
//! construction and knowledge/strategy calculations delegate to the Event core.
use crate::ingress::{CopyContext, event_error};
use crate::marshal::{output_vec, req_at};
use crate::runtime::{Output, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, take_output, thrown,
};
use bumbledb::event::{
    ArithmeticLimits, BeliefArenaDescriptor, BeliefDescriptor, BeliefDescriptorLimits,
    BeliefSpaceIds, DescriptorLimits, Event, ExactArithmetic, FixedPointLimits, PartitionLimits,
    SpaceId,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Array, Env, External, Function, Object, Unknown};
use napi_derive::napi;
mod inspection;

pub enum ArenaOutput {
    Description {
        memory: Vec<u8>,
        identities: BeliefSpaceIds,
    },
    Inspection(inspection::Inspection),
}
#[derive(Clone, Copy)]
enum Op {
    Compile,
    Describe,
    Inspect,
    Known,
    Possible,
    Reach,
    Safe,
}
impl Op {
    fn arity(self) -> usize {
        match self {
            Self::Compile => 6,
            Self::Describe | Self::Inspect => 1,
            _ => 2,
        }
    }
}
fn description(
    op: Op,
    inputs: &[Vec<u8>],
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<BeliefArenaDescriptor, RuntimeError> {
    Ok(if matches!(op, Op::Compile) {
        let name = |i: usize| -> Result<SpaceId, RuntimeError> {
            Ok(SpaceId(
                inputs[i]
                    .as_slice()
                    .try_into()
                    .map_err(|_| RuntimeError::InvalidArgument)?,
            ))
        };
        BeliefArenaDescriptor {
            memory: BeliefDescriptor::from_bytes(&inputs[0], limits, work).map_err(event_error)?,
            identities: BeliefSpaceIds {
                states: name(1)?,
                actions: name(2)?,
                environment: name(3)?,
                state_actions: name(4)?,
                transitions: name(5)?,
            },
        }
    } else {
        BeliefArenaDescriptor::from_bytes(&inputs[0], limits, work).map_err(event_error)?
    })
}

fn execute(op: Op, inputs: &[Vec<u8>], work: &WorkContext) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    if inputs.len() != op.arity() {
        return Err(RuntimeError::InvalidArgument);
    }
    let limits = BeliefDescriptorLimits::default();
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), work);
    let data = description(op, inputs, limits.descriptors, work)?;
    let arena = data.admit(limits, &mut arithmetic).map_err(event_error)?;
    drop(data);
    let output = match op {
        Op::Compile | Op::Describe => {
            let data = BeliefArenaDescriptor::capture(&arena, limits.descriptors, work)
                .map_err(event_error)?;
            let bytes = data
                .to_bytes(limits.descriptors, work)
                .map_err(event_error)?;
            if matches!(op, Op::Compile) {
                Output::Bytes(QueuedBytes { bytes })
            } else {
                Output::EventMemoryArena(ArenaOutput::Description {
                    memory: data
                        .memory
                        .to_bytes(limits.descriptors, work)
                        .map_err(event_error)?,
                    identities: data.identities,
                })
            }
        }
        Op::Inspect => Output::EventMemoryArena(ArenaOutput::Inspection(inspection::arena(
            &arena,
            limits.descriptors,
            work,
        )?)),
        Op::Known | Op::Possible | Op::Reach | Op::Safe => {
            let event = Event::from_bytes_with_parameter_limits(
                &inputs[1],
                None,
                limits.descriptors.events,
                limits.parameters,
                &mut arithmetic,
            )
            .map_err(event_error)?;
            match op {
                Op::Known | Op::Possible => {
                    let value = if matches!(op, Op::Known) {
                        arena.known(&event, work)
                    } else {
                        arena.possible(&event, work)
                    }
                    .map_err(event_error)?;
                    Output::Bytes(QueuedBytes {
                        bytes: value.to_bytes(work).map_err(event_error)?,
                    })
                }
                Op::Reach => {
                    let strategy = arena
                        .arena()
                        .winning_reach(
                            &event,
                            FixedPointLimits::default(),
                            PartitionLimits::default(),
                            work,
                        )
                        .map_err(event_error)?;
                    Output::EventMemoryArena(ArenaOutput::Inspection(inspection::reach(
                        &strategy,
                        limits.descriptors,
                        work,
                    )?))
                }
                Op::Safe => {
                    let strategy = arena
                        .arena()
                        .winning_safe(&event, FixedPointLimits::default(), work)
                        .map_err(event_error)?;
                    Output::EventMemoryArena(ArenaOutput::Inspection(inspection::safe(
                        &strategy,
                        limits.descriptors,
                        work,
                    )?))
                }
                _ => unreachable!(),
            }
        }
    };
    work.checkpoint()?;
    Ok(output)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event_memory_arena(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    inputs: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|e| thrown(env, e))?;
    let op = match operation.as_str() {
        "compile" => Op::Compile,
        "describe" => Op::Describe,
        "inspect" => Op::Inspect,
        "known" => Op::Known,
        "possible" => Op::Possible,
        "reach" => Op::Reach,
        "safe" => Op::Safe,
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    };
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let copy = CopyContext::new(env, work);
            let data = copy.finish((|| {
                if inputs.len() as usize != op.arity() {
                    return copy.checked(Err(RuntimeError::InvalidArgument));
                }
                let mut bytes = DescriptorLimits::default().bytes;
                let mut values = copy.checked(output_vec(op.arity()))?;
                for i in 0..inputs.len() {
                    let value =
                        copy.bytes(req_at::<Unknown>(&inputs, i, "Event memory arena")?, bytes)?;
                    bytes -= value.len();
                    values.push(value);
                }
                Ok(values)
            })())?;
            Ok(Box::new(move |work: &WorkContext| execute(op, &data, work))
                as crate::runtime::Work)
        })
        .map_err(|e| thrown(env, e))?;
    Ok(operation_handle(runtime, operation))
}
#[napi]
pub fn runtime_event_memory_arena_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<ArenaOutput> {
    match take_output(env, handle)? {
        Output::EventMemoryArena(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}
impl napi::bindgen_prelude::ToNapiValue for ArenaOutput {
    #[expect(
        unsafe_code,
        reason = "N-API requires an unsafe conversion entry; this transfers owned data through its object API"
    )]
    unsafe fn to_napi_value(
        env: napi::sys::napi_env,
        value: Self,
    ) -> napi::Result<napi::sys::napi_value> {
        let handle = Env::from_raw(env);
        let object = match value {
            Self::Description { memory, identities } => {
                inspection::description(&handle, memory, identities)?
            }
            Self::Inspection(value) => value.object(&handle)?,
        };
        // SAFETY: the object was created in this live environment.
        unsafe { Object::to_napi_value(env, object) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bumbledb::event::{
        BeliefLimits, BeliefMemory, CoordinateMap, EventPartition, FibreProduct, Space,
        WorldRelation,
    };
    use bumbledb::work::WorkError;
    pub(super) fn input() -> (Vec<Vec<u8>>, Event) {
        let source = Space::new(SpaceId([95; 32]), 1, &()).unwrap();
        let env = Space::new(SpaceId([96; 32]), 0, &()).unwrap();
        let base = CoordinateMap::new(&source, &env, &[], &())
            .unwrap()
            .certify_surjective(&())
            .unwrap();
        let pairs = FibreProduct::new(SpaceId([97; 32]), &base, &base, &()).unwrap();
        let actions = [WorldRelation::identity(&pairs, &()).unwrap()];
        let obs = EventPartition::on(
            &source.full(),
            &[source.full()],
            PartitionLimits::default(),
            &(),
        )
        .unwrap();
        let memory =
            BeliefMemory::new(&actions, &obs, &source.full(), BeliefLimits::default(), &())
                .unwrap();
        let mut input = vec![
            BeliefDescriptor::capture(&memory, DescriptorLimits::default(), &())
                .unwrap()
                .to_bytes(DescriptorLimits::default(), &())
                .unwrap(),
        ];
        input.extend((98..103).map(|i| vec![i; 32]));
        (input, source.coordinate(0, &()).unwrap())
    }
    #[test]
    fn compiled_worker_owns_context_and_distinguishes_knowledge_from_possibility() {
        let work = WorkContext::new();
        let (input, hidden) = input();
        let Output::Bytes(bytes) = execute(Op::Compile, &input, &work).unwrap() else {
            panic!()
        };
        let Output::EventMemoryArena(ArenaOutput::Description { memory, identities }) =
            execute(Op::Describe, std::slice::from_ref(&bytes.bytes), &work).unwrap()
        else {
            panic!()
        };
        assert_eq!(memory, input[0]);
        assert_eq!(identities.states, SpaceId([98; 32]));
        for (op, expected) in [(Op::Known, false), (Op::Possible, true)] {
            let Output::Bytes(output) = execute(
                op,
                &[bytes.bytes.clone(), hidden.to_bytes(&work).unwrap()],
                &work,
            )
            .unwrap() else {
                panic!()
            };
            let value = Event::from_bytes(&output.bytes, &work).unwrap();
            assert_eq!(value.is_full(), expected);
            assert_eq!(value.is_empty(), !expected);
        }
        assert!(matches!(
            execute(Op::Inspect, std::slice::from_ref(&bytes.bytes), &work),
            Ok(Output::EventMemoryArena(ArenaOutput::Inspection(_)))
        ));
        let arena = BeliefArenaDescriptor::import(
            &bytes.bytes,
            BeliefDescriptorLimits::default(),
            &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
        )
        .unwrap();
        let objective = arena.arena().states().full().to_bytes(&work).unwrap();
        for op in [Op::Reach, Op::Safe] {
            assert!(matches!(
                execute(op, &[bytes.bytes.clone(), objective.clone()], &work),
                Ok(Output::EventMemoryArena(ArenaOutput::Inspection(_)))
            ));
            assert!(matches!(
                execute(
                    op,
                    &[bytes.bytes.clone(), hidden.to_bytes(&work).unwrap()],
                    &work
                ),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
        assert!(matches!(
            execute(Op::Compile, &input[..5], &work),
            Err(RuntimeError::InvalidArgument)
        ));
        let mut wrong = input.clone();
        wrong[1].pop();
        assert!(matches!(
            execute(Op::Compile, &wrong, &work),
            Err(RuntimeError::InvalidArgument)
        ));
        assert!(matches!(
            execute(Op::Inspect, &[b"BEBA\x01".to_vec()], &work),
            Err(RuntimeError::Engine { kind: "event", .. })
        ));
        work.cancel();
        for op in [
            Op::Compile,
            Op::Describe,
            Op::Inspect,
            Op::Known,
            Op::Possible,
            Op::Reach,
            Op::Safe,
        ] {
            assert!(matches!(
                execute(op, &[], &work),
                Err(RuntimeError::Work(WorkError::Cancelled))
            ));
        }
    }
}
