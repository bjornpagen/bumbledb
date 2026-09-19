//! Explicit prior binding, with original observations and contraction certificates.
use super::output::Budget;
use super::source::{event, finite, observation_with_budget, space};
use super::{
    Error, ExactArithmetic, Output, ParameterOutput, Result, WorkContext, bytes, dynamics, family,
    function, limits, rational,
};
use bumbledb::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, BetaIntegral, BetaObservation, BetaSource,
    SourceDescriptor,
};
use napi::bindgen_prelude::{Env, Object, Uint8Array};

#[derive(Clone, Copy)]
pub(super) enum Op {
    New,
    Validate,
    Describe,
    Integrate,
    Probability,
    Expectation,
    FamilyExpectation,
}
impl Op {
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "new" => Self::New,
            "validate" => Self::Validate,
            "describe" => Self::Describe,
            "integrate" => Self::Integrate,
            "probability" => Self::Probability,
            "expectation" => Self::Expectation,
            "familyExpectation" => Self::FamilyExpectation,
            _ => return None,
        })
    }
    pub(super) fn valid(self, count: usize, argument: u8) -> bool {
        argument == 0
            && count
                == match self {
                    Self::Validate | Self::Describe => 1,
                    Self::Integrate => 2,
                    _ => 3,
                }
    }
}
fn encoded(
    value: &BetaSource,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    dynamics::encoded(AdmittedFamilyDescriptor::Beta(value.clone()), control, work)
}
fn import(bytes: &[u8], work: &mut ExactArithmetic<'_>) -> Result<BetaSource> {
    if let AdmittedSourceDescriptor::Family(value) =
        SourceDescriptor::import(bytes, limits(), work)?
        && let AdmittedFamilyDescriptor::Beta(value) = *value
    {
        return Ok(value);
    }
    Err(Error::RoleMismatch)
}
pub(super) fn execute(
    op: Op,
    inputs: &[Vec<u8>],
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Output> {
    if matches!(op, Op::New) {
        let source = space(&inputs[0], work)?;
        let alpha = rational(&inputs[1], work)?;
        let beta = rational(&inputs[2], work)?;
        let value = BetaSource::new(&source, alpha, beta, limits().parameters, work)?;
        return bytes(encoded(&value, control, work)?);
    }
    let source = import(&inputs[0], work)?;
    let mut budget = Budget::default();
    let details = match op {
        Op::Validate => return bytes(encoded(&source, control, work)?),
        Op::Describe => Details::Source {
            space: budget.blob(source.source().full().to_bytes(control)?)?,
            alpha: budget.blob(source.alpha().to_bytes(work)?)?,
            beta: budget.blob(source.beta().to_bytes(work)?)?,
        },
        Op::Integrate => {
            let function = function::function(&inputs[1], work)?;
            let result = source.integrate(&function, limits().parameters, work)?;
            Details::Integral {
                source: budget.blob(encoded(&source, control, work)?)?,
                integral: Integral::capture(&result, &mut budget, control, work)?,
            }
        }
        Op::Probability => {
            let value = event(&inputs[1], work)?;
            let evidence = event(&inputs[2], work)?;
            let result = source.probability(&value, &evidence, limits().parameters, work)?;
            let original = result.original();
            let original = observation_with_budget(
                "probability",
                original.event().to_bytes(control)?,
                original.given(),
                original.numerator(),
                original.evidence_mass(),
                original.conditional(),
                &mut budget,
                control,
                work,
            )?;
            observed(&result, original, &mut budget, control, work)?
        }
        Op::Expectation => {
            let payoff = finite(&inputs[1], work)?;
            let evidence = event(&inputs[2], work)?;
            let result = source.expectation(&payoff, &evidence, limits().parameters, work)?;
            let original = result.original();
            let input = SourceDescriptor::capture(
                &AdmittedSourceDescriptor::Function(original.function().clone()),
                limits(),
                work,
            )?
            .to_bytes(limits(), control)?;
            let original = observation_with_budget(
                "finiteExpectation",
                input,
                original.evidence(),
                original.numerator(),
                original.evidence_mass(),
                original.conditional(),
                &mut budget,
                control,
                work,
            )?;
            observed(&result, original, &mut budget, control, work)?
        }
        Op::FamilyExpectation => {
            let payoff = family::family(&inputs[1], work)?;
            let evidence = event(&inputs[2], work)?;
            let result =
                source.family_expectation(&payoff, &evidence, limits().parameters, work)?;
            let original = result.original();
            let input = family::encoded(original.function().clone(), control, work)?;
            let original = observation_with_budget(
                "familyExpectation",
                input,
                original.evidence(),
                original.numerator(),
                original.evidence_mass(),
                original.conditional(),
                &mut budget,
                control,
                work,
            )?;
            observed(&result, original, &mut budget, control, work)?
        }
        Op::New => return Err(Error::InvalidEncoding),
    };
    Ok(Output::Parameter(ParameterOutput::Prior(Box::new(details))))
}

fn observed<T>(
    value: &BetaObservation<T>,
    original: ParameterOutput,
    budget: &mut Budget,
    control: &WorkContext,
    work: &mut ExactArithmetic<'_>,
) -> Result<Details> {
    Ok(Details::Observation {
        source: budget.blob(encoded(value.source(), control, work)?)?,
        original: Box::new(original),
        numerator: Integral::capture(value.numerator(), budget, control, work)?,
        evidence: Integral::capture(value.evidence_mass(), budget, control, work)?,
        value: value
            .value()
            .map(|v| budget.blob(v.to_bytes(work)?))
            .transpose()?,
    })
}
pub struct Integral {
    function: Vec<u8>,
    polynomial: Vec<u8>,
    exceptions: Vec<u8>,
    value: Vec<u8>,
}
impl Integral {
    fn capture(
        value: &BetaIntegral,
        budget: &mut Budget,
        control: &WorkContext,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Ok(Self {
            function: budget.blob(function::encoded(value.function().clone(), control, work)?)?,
            polynomial: budget.blob(
                value
                    .polynomial()
                    .to_bytes(limits().parameters.parameters.region.polynomial, work)?,
            )?,
            exceptions: budget.blob(
                value
                    .exceptions()
                    .to_bytes(limits().parameters.parameters, work)?,
            )?,
            value: budget.blob(value.value().to_bytes(work)?)?,
        })
    }
    fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        object.set("function", Uint8Array::from(self.function))?;
        object.set("polynomial", Uint8Array::from(self.polynomial))?;
        object.set("exceptions", Uint8Array::from(self.exceptions))?;
        object.set("value", Uint8Array::from(self.value))?;
        Ok(object)
    }
}
pub enum Details {
    Source {
        space: Vec<u8>,
        alpha: Vec<u8>,
        beta: Vec<u8>,
    },
    Integral {
        source: Vec<u8>,
        integral: Integral,
    },
    Observation {
        source: Vec<u8>,
        original: Box<ParameterOutput>,
        numerator: Integral,
        evidence: Integral,
        value: Option<Vec<u8>>,
    },
}
impl Details {
    pub(super) fn object(self, env: &Env) -> napi::Result<Object<'_>> {
        let mut object = Object::new(env)?;
        match self {
            Self::Source { space, alpha, beta } => {
                object.set("space", Uint8Array::from(space))?;
                object.set("alpha", Uint8Array::from(alpha))?;
                object.set("beta", Uint8Array::from(beta))?;
            }
            Self::Integral { source, integral } => {
                object.set("source", Uint8Array::from(source))?;
                object.set("integral", integral.object(env)?)?;
            }
            Self::Observation {
                source,
                original,
                numerator,
                evidence,
                value,
            } => {
                object.set("source", Uint8Array::from(source))?;
                object.set("original", original.source_object(env)?)?;
                object.set("numerator", numerator.object(env)?)?;
                object.set("evidenceMass", evidence.object(env)?)?;
                object.set("value", value.map(Uint8Array::from))?;
            }
        }
        Ok(object)
    }
}
