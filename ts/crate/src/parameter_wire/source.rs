use super::output::{Budget, Witness};
use super::{
    Error, ExactArithmetic, Output, ParameterOutput, Result, WorkContext, boolean, bytes, domain,
    function, limits, rational, region, root,
};
use bumbledb::event::{
    AdmittedSourceDescriptor, Event, FiniteFunction, ParameterGuard, ParameterWorld, RealWitness,
    SourceDescriptor, Space, WorldCardinality,
};

#[derive(Clone, Copy)]
pub(super) enum Op {
    New,
    Describe,
    Event,
    Mass,
    Probability,
    Expectation,
    ContainsRational,
    ContainsRoot,
    Witness,
    Cardinality,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "describe" => Self::Describe,
            "event" => Self::Event,
            "mass" => Self::Mass,
            "probability" => Self::Probability,
            "expectation" => Self::Expectation,
            "containsRational" => Self::ContainsRational,
            "containsRoot" => Self::ContainsRoot,
            "witness" => Self::Witness,
            "cardinality" => Self::Cardinality,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        argument == 0
            && match self {
                Self::New => count >= 2 && (count - 2).is_multiple_of(2),
                Self::Event | Self::Probability | Self::Expectation => count == 2,
                Self::ContainsRational | Self::ContainsRoot => count == 3,
                _ => count == 1,
            }
    }
}
pub(super) fn event(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<Event> {
    let limits = limits();
    Event::from_bytes_with_parameter_limits(
        bytes,
        None,
        limits.descriptors.events,
        limits.parameters,
        work,
    )
}
pub(super) fn space(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<Space> {
    let value = event(bytes, work)?;
    if !value.is_full() {
        return Err(Error::InvalidEncoding);
    }
    Ok(value.space())
}
pub(super) fn finite(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<FiniteFunction> {
    if let AdmittedSourceDescriptor::Function(value) =
        SourceDescriptor::import(bytes, limits(), work)?
    {
        return Ok(value);
    }
    Err(Error::RoleMismatch)
}
pub(super) fn outcomes(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(
        bytes.try_into().map_err(|_| Error::InvalidEncoding)?,
    ))
}
pub(super) fn encode(value: &Event, control: &WorkContext) -> Result<Output> {
    bytes(value.to_bytes(control)?)
}
fn witness(value: RealWitness, work: &mut ExactArithmetic<'_>) -> Result<Witness> {
    Ok(match value {
        RealWitness::Rational(value) => Witness::Rational(value.to_bytes(work)?),
        RealWitness::Algebraic(value) => {
            Witness::Algebraic(value.to_bytes(limits().parameters.parameters.algebraic, work)?)
        }
    })
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let limits = limits().parameters;
    if matches!(op, Op::New) {
        return create(inputs, control, work);
    }
    if matches!(op, Op::Expectation) {
        let value = finite(&inputs[0], work)?;
        let evidence = event(&inputs[1], work)?;
        let result = value.parameter_expectation(&evidence, limits, work)?;
        let input = SourceDescriptor::capture(
            &AdmittedSourceDescriptor::Function(result.function().clone()),
            super::limits(),
            work,
        )?
        .to_bytes(super::limits(), control)?;
        return observation(
            "finiteExpectation",
            input,
            result.evidence(),
            result.numerator(),
            result.evidence_mass(),
            result.conditional(),
            control,
            work,
        );
    }
    let value = event(&inputs[0], work)?;
    let space = value.space();
    match op {
        Op::Describe => {
            let domain = space.parameter_domain().ok_or(Error::MissingParameter)?;
            let guards = space.parameter_guards().ok_or(Error::MissingParameter)?;
            let mut budget = Budget::default();
            let domain = budget.blob(domain.to_bytes(limits.parameters, work)?)?;
            let mut values = Vec::new();
            values.try_reserve_exact(guards.len())?;
            for guard in guards {
                values.push((
                    guard.coordinate,
                    budget.blob(guard.region.to_bytes(limits.parameters, work)?)?,
                ));
            }
            Ok(Output::Parameter(ParameterOutput::Source {
                domain,
                guards: values,
                outcomes: space.outcome_coordinates(),
            }))
        }
        Op::Event => encode(
            &space.parameter_event(&region(&inputs[1], work)?, limits, work)?,
            control,
        ),
        Op::Mass => function::encode(value.parameter_mass(limits, work)?, control, work),
        Op::Probability => {
            let evidence = event(&inputs[1], work)?.align_to(&space, control)?;
            let result = value.parameter_probability(&evidence, limits, work)?;
            observation(
                "probability",
                result.event().to_bytes(control)?,
                result.given(),
                result.numerator(),
                result.evidence_mass(),
                result.conditional(),
                control,
                work,
            )
        }
        Op::ContainsRational | Op::ContainsRoot => {
            let parameter = if matches!(op, Op::ContainsRational) {
                RealWitness::Rational(rational(&inputs[1], work)?)
            } else {
                RealWitness::Algebraic(root(&inputs[1], work)?)
            };
            Ok(boolean(value.contains_parameter(
                &ParameterWorld {
                    parameter,
                    outcomes: outcomes(&inputs[2])?,
                },
                limits,
                work,
            )?))
        }
        Op::Witness => {
            let value = value
                .parameter_witness(limits, work)?
                .map(|world| Result::Ok((witness(world.parameter, work)?, world.outcomes)))
                .transpose()?;
            Ok(Output::Parameter(ParameterOutput::World(value)))
        }
        Op::Cardinality => Ok(Output::Parameter(ParameterOutput::Cardinality(match value
            .world_cardinality(control)?
        {
            WorldCardinality::Finite(n) => Some(n),
            WorldCardinality::Continuum => None,
        }))),
        _ => Err(Error::InvalidEncoding),
    }
}
#[allow(
    clippy::too_many_arguments,
    reason = "marshal the complete owned observation without dropping any semantic component"
)]
pub(super) fn observation(
    kind: &'static str,
    input: Vec<u8>,
    evidence: &Event,
    numerator: &bumbledb::event::ParameterFunction,
    mass: &bumbledb::event::ParameterFunction,
    conditional: &bumbledb::event::ParameterFunction,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    Ok(Output::Parameter(observation_with_budget(
        kind,
        input,
        evidence,
        numerator,
        mass,
        conditional,
        &mut Budget::default(),
        control,
        work,
    )?))
}
#[allow(
    clippy::too_many_arguments,
    reason = "one shared budget covers all nested observation bytes"
)]
pub(super) fn observation_with_budget(
    kind: &'static str,
    input: Vec<u8>,
    evidence: &Event,
    numerator: &bumbledb::event::ParameterFunction,
    mass: &bumbledb::event::ParameterFunction,
    conditional: &bumbledb::event::ParameterFunction,
    budget: &mut Budget,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<ParameterOutput> {
    let input = budget.blob(input)?;
    let given = budget.blob(evidence.to_bytes(control)?)?;
    let numerator = budget.blob(function::encoded(numerator.clone(), control, work)?)?;
    let mass = budget.blob(function::encoded(mass.clone(), control, work)?)?;
    let defined = budget.blob(
        conditional
            .defined_on()
            .to_bytes(limits().parameters.parameters, work)?,
    )?;
    let value = budget.blob(function::encoded(conditional.clone(), control, work)?)?;
    Ok(ParameterOutput::Observation {
        kind,
        input,
        given,
        numerator,
        mass,
        value,
        defined,
    })
}

fn create(
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    let limits = limits().parameters;
    let space = space(&inputs[0], work)?;
    let domain = domain(&inputs[1], work)?;
    let count = (inputs.len() - 2) / 2;
    if count > 62 {
        return Err(Error::Capacity(bumbledb::event::Capacity::Coordinates));
    }
    let mut guards = Vec::new();
    guards.try_reserve_exact(count)?;
    for pair in inputs[2..].as_chunks::<2>().0 {
        let [coordinate] = pair[0].as_slice() else {
            return Err(Error::InvalidEncoding);
        };
        guards.push(ParameterGuard {
            coordinate: *coordinate,
            region: region(&pair[1], work)?,
        });
    }
    encode(
        &space.with_parameters(domain, &guards, limits, work)?.full(),
        control,
    )
}
