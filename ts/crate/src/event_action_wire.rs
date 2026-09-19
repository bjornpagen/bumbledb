//! Checked arenas and strategy recipes on the shared worker. All algebra,
//! policy checks and fixed-point solving delegate to the native Event core.
use crate::event_inspection::{Export, Inspection};
use crate::ingress::{CopyContext, event_error};
use crate::marshal::{output_vec, req_at};
use crate::runtime::{Output, QueuedBytes, RuntimeError};
use crate::runtime_wire::{
    OperationHandle, RuntimeHandle, notification, operation_handle, owner, take_output, thrown,
};
use bumbledb::event::{
    ActionArena, ActionDescriptor, ActionDescriptorLimits, AdmittedActionDescriptor,
    AdmittedDescriptor, ArithmeticLimits, BeliefArenaDescriptor, BeliefDescriptorLimits, Capacity,
    Descriptor, DescriptorLimits, Error, Event, ExactArithmetic, WorldRelation,
};
use bumbledb::work::WorkContext;
use napi::bindgen_prelude::{Array, Env, External, Function, Object, Unknown};
use napi_derive::napi;
mod data;

pub enum ActionOutput {
    Description(ActionDescriptor),
    Inspection(Inspection),
}
#[derive(Clone, Copy)]
enum Op {
    Admit,
    Describe,
    Inspect,
    FromMemory,
    Reach,
    Safe,
    Restrict,
    Enabled,
    Good,
    Predecessor,
}
impl Op {
    fn arity(self) -> usize {
        match self {
            Self::Reach | Self::Safe | Self::Restrict | Self::Good | Self::Predecessor => 1,
            _ => 0,
        }
    }
}
enum Input {
    Data(ActionDescriptor),
    Bytes(Vec<u8>),
}
fn arena(value: &AdmittedActionDescriptor) -> &ActionArena {
    match value {
        AdmittedActionDescriptor::Arena(arena) => arena,
        AdmittedActionDescriptor::Reach(strategy) => strategy.arena(),
        AdmittedActionDescriptor::Safe(strategy) => strategy.arena(),
    }
}
fn inspect(
    value: &AdmittedActionDescriptor,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Inspection, RuntimeError> {
    let kind = match value {
        AdmittedActionDescriptor::Arena(_) => "arena",
        AdmittedActionDescriptor::Reach(_) => "reach",
        AdmittedActionDescriptor::Safe(_) => "safe",
    };
    let mut out = Export::new(kind, limits, work)?;
    let arena = arena(value);
    out.event("states", &arena.states().full())?;
    out.event("choices", &arena.choices().full())?;
    out.event("outcomes", &arena.outcomes().full())?;
    out.descriptor(
        "actions",
        &AdmittedDescriptor::Fibre(arena.actions().clone()),
    )?;
    out.descriptor(
        "transition",
        &AdmittedDescriptor::Relation(arena.transition().clone()),
    )?;
    match value {
        AdmittedActionDescriptor::Reach(strategy) => {
            out.event("goal", strategy.goal())?;
            out.event("winning", strategy.winning())?;
            out.descriptor(
                "policy",
                &AdmittedDescriptor::Relation(strategy.policy().clone()),
            )?;
            out.list("ranks", strategy.ranked().layers().cells().len(), |i| {
                Ok(strategy.ranked().layers().cells()[i].clone())
            })?;
        }
        AdmittedActionDescriptor::Safe(strategy) => {
            out.event("invariant", strategy.invariant())?;
            out.event("winning", strategy.winning())?;
            out.descriptor(
                "policy",
                &AdmittedDescriptor::Relation(strategy.policy().clone()),
            )?;
        }
        AdmittedActionDescriptor::Arena(_) => {}
    }
    out.finish()
}
fn bounded_bytes(
    bytes: Vec<u8>,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    if bytes.len() > limits.bytes {
        return Err(event_error(Error::Capacity(Capacity::DescriptorBytes)));
    }
    Ok(Output::Bytes(QueuedBytes { bytes }))
}
fn encoded(
    value: &AdmittedActionDescriptor,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    let bytes = ActionDescriptor::capture(value, limits, work)
        .map_err(event_error)?
        .to_bytes(limits, work)
        .map_err(event_error)?;
    bounded_bytes(bytes, limits, work)
}
fn relation_bytes(
    value: WorldRelation,
    limits: DescriptorLimits,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    let bytes = Descriptor::capture(&AdmittedDescriptor::Relation(value), limits, work)
        .map_err(event_error)?
        .to_bytes(limits, work)
        .map_err(event_error)?;
    bounded_bytes(bytes, limits, work)
}
fn with_event(
    op: Op,
    value: &AdmittedActionDescriptor,
    event: &Event,
    limits: &ActionDescriptorLimits,
    arithmetic: &mut ExactArithmetic<'_>,
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    let a = arena(value);
    Ok(match op {
        Op::Reach => encoded(
            &AdmittedActionDescriptor::Reach(
                a.winning_reach_with_parameters(
                    event,
                    limits.fixed_points,
                    limits.partitions,
                    limits.parameters,
                    arithmetic,
                )
                .map_err(event_error)?,
            ),
            limits.descriptors,
            work,
        )?,
        Op::Safe => encoded(
            &AdmittedActionDescriptor::Safe(
                a.winning_safe_with_parameters(
                    event,
                    limits.fixed_points,
                    limits.parameters,
                    arithmetic,
                )
                .map_err(event_error)?,
            ),
            limits.descriptors,
            work,
        )?,
        Op::Restrict => {
            let policy = WorldRelation::new(a.actions(), event, work).map_err(event_error)?;
            let selected = match value {
                AdmittedActionDescriptor::Reach(strategy) => AdmittedActionDescriptor::Reach(
                    strategy
                        .with_policy_and_parameters(&policy, limits.parameters, arithmetic)
                        .map_err(event_error)?,
                ),
                AdmittedActionDescriptor::Safe(strategy) => AdmittedActionDescriptor::Safe(
                    strategy
                        .with_policy_and_parameters(&policy, limits.parameters, arithmetic)
                        .map_err(event_error)?,
                ),
                AdmittedActionDescriptor::Arena(_) => {
                    return Err(RuntimeError::InvalidArgument);
                }
            };
            encoded(&selected, limits.descriptors, work)?
        }
        Op::Good => relation_bytes(
            a.good(event, work).map_err(event_error)?,
            limits.descriptors,
            work,
        )?,
        Op::Predecessor => bounded_bytes(
            a.controllable_predecessor(event, work)
                .map_err(event_error)?
                .to_bytes(work)
                .map_err(event_error)?,
            limits.descriptors,
            work,
        )?,
        _ => unreachable!(),
    })
}

fn execute(
    op: Op,
    input: Input,
    operands: &[Vec<u8>],
    work: &WorkContext,
) -> Result<Output, RuntimeError> {
    work.checkpoint()?;
    if operands.len() != op.arity() {
        return Err(RuntimeError::InvalidArgument);
    }
    let limits = ActionDescriptorLimits::default();
    let mut arithmetic = ExactArithmetic::new(ArithmeticLimits::default(), work);
    let value = if matches!(op, Op::FromMemory) {
        let Input::Bytes(bytes) = input else {
            return Err(RuntimeError::InvalidArgument);
        };
        let memory = BeliefArenaDescriptor::import(
            &bytes,
            BeliefDescriptorLimits::default(),
            &mut arithmetic,
        )
        .map_err(event_error)?;
        AdmittedActionDescriptor::Arena(memory.arena().clone())
    } else {
        let data = match input {
            Input::Data(data) => data,
            Input::Bytes(bytes) => ActionDescriptor::from_bytes(&bytes, limits.descriptors, work)
                .map_err(event_error)?,
        };
        data.admit(limits, &mut arithmetic).map_err(event_error)?
    };
    let output = match op {
        Op::Admit | Op::FromMemory => encoded(&value, limits.descriptors, work)?,
        Op::Describe => {
            let data =
                ActionDescriptor::capture(&value, limits.descriptors, work).map_err(event_error)?;
            // Bound the complete output envelope as well as nested payloads.
            data.to_bytes(limits.descriptors, work)
                .map_err(event_error)?;
            Output::EventAction(ActionOutput::Description(data))
        }
        Op::Inspect => Output::EventAction(ActionOutput::Inspection(inspect(
            &value,
            limits.descriptors,
            work,
        )?)),
        Op::Enabled => relation_bytes(
            arena(&value).enabled(work).map_err(event_error)?,
            limits.descriptors,
            work,
        )?,
        Op::Reach | Op::Safe | Op::Restrict | Op::Good | Op::Predecessor => {
            let event = Event::from_bytes_with_parameter_limits(
                &operands[0],
                None,
                limits.descriptors.events,
                limits.parameters,
                &mut arithmetic,
            )
            .map_err(event_error)?;
            with_event(op, &value, &event, &limits, &mut arithmetic, work)?
        }
    };
    work.checkpoint()?;
    Ok(output)
}

#[napi]
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_event_action(
    env: Env,
    handle: &External<RuntimeHandle>,
    operation: String,
    input: Unknown,
    operands: Array,
    callback: Function<(), ()>,
) -> napi::Result<External<OperationHandle>> {
    let runtime = owner(handle).map_err(|e| thrown(env, e))?;
    let op = match operation.as_str() {
        "admit" => Op::Admit,
        "describe" => Op::Describe,
        "inspect" => Op::Inspect,
        "fromMemory" => Op::FromMemory,
        "reach" => Op::Reach,
        "safe" => Op::Safe,
        "restrict" => Op::Restrict,
        "enabled" => Op::Enabled,
        "good" => Op::Good,
        "predecessor" => Op::Predecessor,
        _ => return Err(thrown(env, RuntimeError::InvalidArgument)),
    };
    let operation = runtime
        .submit(WorkContext::new(), notification(callback)?, |work| {
            let copy = CopyContext::new(env, work);
            let (input, operands) = copy.finish((|| {
                if operands.len() as usize != op.arity() {
                    return copy.checked(Err(RuntimeError::InvalidArgument));
                }
                let mut remaining = DescriptorLimits::default().bytes;
                let input = if matches!(op, Op::Admit) {
                    Input::Data(data::parse(input, &copy)?)
                } else {
                    let bytes = copy.bytes(input, remaining)?;
                    remaining -= bytes.len();
                    Input::Bytes(bytes)
                };
                let mut values = copy.checked(output_vec(op.arity()))?;
                for i in 0..operands.len() {
                    let bytes =
                        copy.bytes(req_at(&operands, i, "Event action operands")?, remaining)?;
                    remaining -= bytes.len();
                    values.push(bytes);
                }
                Ok((input, values))
            })())?;
            Ok(
                Box::new(move |work: &WorkContext| execute(op, input, &operands, work))
                    as crate::runtime::Work,
            )
        })
        .map_err(|e| thrown(env, e))?;
    Ok(operation_handle(runtime, operation))
}
#[napi]
pub fn runtime_event_action_take(
    env: Env,
    handle: &External<OperationHandle>,
) -> napi::Result<ActionOutput> {
    match take_output(env, handle)? {
        Output::EventAction(value) => Ok(value),
        _ => Err(thrown(env, RuntimeError::InvalidArgument)),
    }
}
impl napi::bindgen_prelude::ToNapiValue for ActionOutput {
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
            Self::Description(value) => data::object(&handle, value)?,
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
        CoordinateMap, FibreProduct, FixedPointLimits, PartitionLimits, Space, SpaceId,
    };
    use bumbledb::work::WorkError;
    fn fixture() -> (ActionDescriptor, Event) {
        let s = Space::new(SpaceId([110; 32]), 1, &()).unwrap();
        let a = Space::new(SpaceId([111; 32]), 1, &()).unwrap();
        let e = Space::new(SpaceId([112; 32]), 0, &()).unwrap();
        let base = |s: &Space| {
            CoordinateMap::new(s, &e, &[], &())
                .unwrap()
                .certify_surjective(&())
                .unwrap()
        };
        let actions = FibreProduct::new(SpaceId([113; 32]), &base(&s), &base(&a), &()).unwrap();
        let steps =
            FibreProduct::new(SpaceId([114; 32]), &base(actions.space()), &base(&s), &()).unwrap();
        let transition = WorldRelation::new(
            &steps,
            &steps.space().table(7, &[0b1110_0001], &()).unwrap(),
            &(),
        )
        .unwrap();
        let arena = ActionArena::new(&actions, &transition, &()).unwrap();
        (
            ActionDescriptor::capture(
                &AdmittedActionDescriptor::Arena(arena),
                DescriptorLimits::default(),
                &(),
            )
            .unwrap(),
            s.coordinate(0, &()).unwrap(),
        )
    }
    fn bytes(output: Output) -> Vec<u8> {
        let Output::Bytes(value) = output else {
            panic!()
        };
        value.bytes
    }
    #[test]
    #[allow(clippy::too_many_lines)]
    fn worker_replays_strategies_and_rejects_stalling_incomplete_foreign_and_cancelled_work() {
        let work = WorkContext::new();
        let (data, goal) = fixture();
        let arena = bytes(execute(Op::Admit, Input::Data(data.clone()), &[], &work).unwrap());
        let Output::EventAction(ActionOutput::Description(restored)) =
            execute(Op::Describe, Input::Bytes(arena.clone()), &[], &work).unwrap()
        else {
            panic!()
        };
        assert_eq!(restored, data);
        let goal = goal.to_bytes(&work).unwrap();
        let reach = bytes(
            execute(
                Op::Reach,
                Input::Bytes(arena.clone()),
                std::slice::from_ref(&goal),
                &work,
            )
            .unwrap(),
        );
        let Output::EventAction(ActionOutput::Inspection(inspected)) =
            execute(Op::Inspect, Input::Bytes(reach.clone()), &[], &work).unwrap()
        else {
            panic!()
        };
        assert_eq!(inspected.kind, "reach");
        assert_eq!(inspected.fields.len(), 8);
        assert_eq!(inspected.lists[0].1.len(), 2);
        let ActionDescriptor::Reach {
            arena: model,
            policy: Some(policy),
            ..
        } = ActionDescriptor::from_bytes(&reach, DescriptorLimits::default(), &work).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            bytes(
                execute(
                    Op::Restrict,
                    Input::Bytes(reach.clone()),
                    std::slice::from_ref(&policy),
                    &work
                )
                .unwrap()
            ),
            reach
        );
        let region = Event::from_bytes(&policy, &work).unwrap();
        for policy in [region.space().empty(), region.space().full()] {
            assert!(matches!(
                execute(
                    Op::Restrict,
                    Input::Bytes(reach.clone()),
                    &[policy.to_bytes(&work).unwrap()],
                    &work
                ),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
        assert!(execute(Op::Restrict, Input::Bytes(arena.clone()), &[policy], &work).is_err());
        for op in [Op::Good, Op::Predecessor, Op::Safe] {
            assert!(matches!(
                execute(
                    op,
                    Input::Bytes(arena.clone()),
                    std::slice::from_ref(&goal),
                    &work
                ),
                Ok(Output::Bytes(_))
            ));
        }
        assert!(matches!(
            execute(Op::Enabled, Input::Bytes(arena.clone()), &[], &work),
            Ok(Output::Bytes(_))
        ));
        let mut malformed = data;
        let ActionDescriptor::Arena(data) = &mut malformed else {
            panic!()
        };
        data.transition.region = vec![];
        assert!(execute(Op::Admit, Input::Data(malformed), &[], &work).is_err());
        assert!(execute(Op::Inspect, Input::Bytes(b"BEAC\x01".to_vec()), &[], &work).is_err());
        assert!(matches!(
            execute(
                Op::Inspect,
                Input::Bytes(arena.clone()),
                std::slice::from_ref(&goal),
                &work
            ),
            Err(RuntimeError::InvalidArgument)
        ));
        let foreign = Space::new(SpaceId([115; 32]), 1, &work)
            .unwrap()
            .full()
            .to_bytes(&work)
            .unwrap();
        assert!(matches!(
            execute(Op::Reach, Input::Bytes(arena.clone()), &[foreign], &work),
            Err(RuntimeError::Engine { kind: "event", .. })
        ));
        // Malformed selected choices cannot hide behind an empty objective.
        let bad = ActionDescriptor::Reach {
            arena: model,
            goal: region.space().empty().to_bytes(&work).unwrap(),
            policy: Some(vec![]),
        };
        assert!(execute(Op::Admit, Input::Data(bad), &[], &work).is_err());
        work.cancel();
        for op in [
            Op::Admit,
            Op::Describe,
            Op::Inspect,
            Op::FromMemory,
            Op::Reach,
            Op::Safe,
            Op::Restrict,
            Op::Enabled,
            Op::Good,
            Op::Predecessor,
        ] {
            assert!(matches!(
                execute(op, Input::Bytes(arena.clone()), &[], &work),
                Err(RuntimeError::Work(WorkError::Cancelled))
            ));
        }
    }
    #[test]
    fn inspection_bounds_combined_arenas_objectives_and_rank_exports() {
        let work = WorkContext::new();
        let (data, goal) = fixture();
        let value = data
            .admit(
                ActionDescriptorLimits::default(),
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
            )
            .unwrap();
        let predecessor = bytes(
            with_event(
                Op::Predecessor,
                &value,
                &goal,
                &ActionDescriptorLimits::default(),
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
                &work,
            )
            .unwrap(),
        );
        let short = ActionDescriptorLimits {
            descriptors: DescriptorLimits {
                bytes: predecessor.len() - 1,
                ..DescriptorLimits::default()
            },
            ..ActionDescriptorLimits::default()
        };
        assert!(matches!(
            with_event(
                Op::Predecessor,
                &value,
                &goal,
                &short,
                &mut ExactArithmetic::new(ArithmeticLimits::default(), &work),
                &work
            ),
            Err(RuntimeError::Engine { kind: "event", .. })
        ));
        let reach = arena(&value)
            .winning_reach(
                &goal,
                FixedPointLimits::default(),
                PartitionLimits::default(),
                &work,
            )
            .unwrap();
        let strategy = AdmittedActionDescriptor::Reach(reach);
        let full = inspect(&strategy, DescriptorLimits::default(), &work).unwrap();
        let bytes = full.fields.iter().map(|(_, b)| b.len()).sum::<usize>()
            + full
                .lists
                .iter()
                .flat_map(|(_, rows)| rows.iter())
                .map(Vec::len)
                .sum::<usize>();
        let items = 1
            + full.fields.len()
            + full
                .lists
                .iter()
                .map(|(_, list)| 1 + list.len())
                .sum::<usize>();
        let exact = DescriptorLimits {
            bytes,
            items,
            ..DescriptorLimits::default()
        };
        assert!(inspect(&strategy, exact, &work).is_ok());
        for short in [
            DescriptorLimits {
                bytes: bytes - 1,
                ..exact
            },
            DescriptorLimits {
                items: items - 1,
                ..exact
            },
        ] {
            assert!(matches!(
                inspect(&strategy, short, &work),
                Err(RuntimeError::Engine { kind: "event", .. })
            ));
        }
    }
}
