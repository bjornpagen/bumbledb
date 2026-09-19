//! Owned query observations. Source Event identity is part of every result:
//! equal numerical probabilities do not identify unrelated observations.
use crate::event::{
    ExactArithmetic, ExactRational, ParameterProbabilityObservation, ParameterSourceLimits,
    ProbabilityObservation,
};
use crate::{Event, Result};

#[derive(Debug, Clone)]
pub enum ProbabilityValue {
    Fixed {
        observation: ProbabilityObservation,
        value: Option<ExactRational>,
    },
    Parameter(ParameterProbabilityObservation),
}

/// One exact probability answer with the complete original source pair.
/// Construction is native; a numerical result cannot forge this ownership.
#[derive(Debug, Clone)]
pub struct ProbabilityAnswer {
    event: Event,
    given: Event,
    value: ProbabilityValue,
}
impl ProbabilityAnswer {
    pub(crate) fn new(event: Event, given: Event, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        Self::with_limits(event, given, ParameterSourceLimits::default(), work)
    }
    pub(super) fn with_limits(
        event: Event,
        given: Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let value = if event.space().parameter_domain().is_some() {
            ProbabilityValue::Parameter(event.parameter_probability(&given, limits, work)?)
        } else {
            let observation = event.probability(&given, work)?;
            let value = observation.value(work)?;
            ProbabilityValue::Fixed { observation, value }
        };
        Ok(Self {
            event,
            given,
            value,
        })
    }
    #[must_use]
    pub fn event(&self) -> &Event {
        &self.event
    }
    #[must_use]
    pub fn given(&self) -> &Event {
        &self.given
    }
    #[must_use]
    pub fn value(&self) -> &ProbabilityValue {
        &self.value
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        match &self.value {
            ProbabilityValue::Fixed { observation, .. } => observation.is_impossible(),
            ProbabilityValue::Parameter(v) => v.is_impossible(),
        }
    }
}
/// Identity follows owned Events, as for ordinary `AnswerValue::Event`. Numerical
/// equality alone intentionally cannot collapse differently sourced observations.
impl PartialEq for ProbabilityAnswer {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event && self.given == other.given
    }
}
impl Eq for ProbabilityAnswer {}

/// Exact contraction of a structurally complete payoff on the evidence.
#[derive(Debug, Clone)]
pub enum ExpectationValue {
    Fixed {
        observation: crate::event::ExpectationObservation,
        value: Option<ExactRational>,
    },
    Parameter(crate::event::ParameterExpectationObservation),
    Family(crate::event::ParameterExpectationObservation<crate::event::FamilyFunction>),
}

mod number;
pub use number::{
    ObservationComponent, ObservationNumber, ObservationNumberCodecLimits, ObservationNumberExpr,
    ObservationNumberImport, ObservationNumberLimits, ObservationPredicate,
    ObservationPredicateExpr, ObservationPredicateImport, PredicateEvents, PredicateRefinement,
};

mod payoff;
pub use payoff::ExpectationPayoff;
pub(crate) use payoff::{ExpectationInput, PayoffInput};

/// The payoff carrier remains explicit in native observations.
#[derive(Debug, Clone, Copy)]
pub enum ExpectationFunction<'a> {
    Finite(&'a crate::event::FiniteFunction),
    Family(&'a crate::event::FamilyFunction),
}

/// An owned conditional expectation, including its complete evidence-relative
/// payoff. Construction requires structural partition or function-cover admission.
#[derive(Debug, Clone)]
pub struct ExpectationAnswer {
    input: ExpectationPayoff,
    value: ExpectationValue,
    pub(crate) function_identity: Box<[u8]>,
}

impl ExpectationAnswer {
    pub(crate) fn new(input: ExpectationPayoff, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        Self::with_limits(
            input,
            &crate::event::SourceDescriptorLimits::default(),
            work,
        )
    }
    pub(super) fn with_limits(
        input: ExpectationPayoff,
        limits: &crate::event::SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let given = input.given();
        let (value, original) = if let ExpectationPayoff::Family(cover) = &input {
            let function = cover.function();
            (
                ExpectationValue::Family(function.expectation(given, limits.parameters, work)?),
                crate::ImportedPayoff::Family(function.clone()),
            )
        } else {
            let function = input.finite_function(limits.functions, work)?;
            let value = if given.space().parameter_domain().is_some() {
                ExpectationValue::Parameter(function.parameter_expectation(
                    given,
                    limits.parameters,
                    work,
                )?)
            } else {
                let observation = function.expectation(given, work)?;
                let value = observation.value(work)?;
                ExpectationValue::Fixed { observation, value }
            };
            (value, crate::ImportedPayoff::Finite(function))
        };
        let identity = crate::PayoffImport::capture(original, *limits, work)?;
        Ok(Self {
            input,
            value,
            function_identity: identity.bytes().into(),
        })
    }

    #[must_use]
    pub fn given(&self) -> &Event {
        self.input.given()
    }

    /// Scalar rosters retain their partition and zero payoffs. Function covers
    /// return `None`; inspect `payoff()` for their checked patches.
    #[must_use]
    pub fn partition(&self) -> Option<&crate::event::EventPartition> {
        match &self.input {
            ExpectationPayoff::Scalar { partition, .. } => Some(partition),
            _ => None,
        }
    }

    #[must_use]
    pub fn values(&self) -> Option<&[ExactRational]> {
        match &self.input {
            ExpectationPayoff::Scalar { values, .. } => Some(values),
            _ => None,
        }
    }

    #[must_use]
    pub fn payoff(&self) -> &ExpectationPayoff {
        &self.input
    }

    #[must_use]
    pub fn value(&self) -> &ExpectationValue {
        &self.value
    }

    #[must_use]
    pub fn function(&self) -> ExpectationFunction<'_> {
        match &self.value {
            ExpectationValue::Fixed { observation, .. } => {
                ExpectationFunction::Finite(observation.function())
            }
            ExpectationValue::Parameter(observation) => {
                ExpectationFunction::Finite(observation.function())
            }
            ExpectationValue::Family(observation) => {
                ExpectationFunction::Family(observation.function())
            }
        }
    }

    #[must_use]
    pub fn is_impossible(&self) -> bool {
        match &self.value {
            ExpectationValue::Fixed { observation, .. } => observation.is_impossible(),
            ExpectationValue::Parameter(observation) => observation.is_impossible(),
            ExpectationValue::Family(observation) => observation.is_impossible(),
        }
    }
}

/// Equality retains source and payoff identity; equal means are insufficient.
/// Finite functions ignore empty buckets and retain every nonzero region.
/// Family identity uses the encoded presentation, not solver-checked numerical
/// equivalence. Use `FamilyFunction::equivalent` for the latter.
impl PartialEq for ExpectationAnswer {
    fn eq(&self, other: &Self) -> bool {
        self.given() == other.given() && self.function_identity == other.function_identity
    }
}
impl Eq for ExpectationAnswer {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{ArithmeticLimits, Capacity, Error, Space, SpaceId};

    #[test]
    fn payoff_admission_shares_exact_work_and_checks_even_empty_zero_values() {
        let control = crate::WorkContext::new();
        let source = Space::new(SpaceId([235; 32]), 0, &control).unwrap();
        let input = ExpectationInput {
            given: source.full(),
            payoffs: vec![
                (PayoffInput::Ratio([0, 1, 2]), source.full()),
                (PayoffInput::Ratio([0, 2, 4]), source.full()),
            ],
        };
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
        let admitted = input.admit(&control, &mut work).unwrap();
        let ExpectationPayoff::Scalar { values, .. } = admitted else {
            panic!("scalar")
        };
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].to_string(), "1/2");
        let mut bounded = ExactArithmetic::new(
            ArithmeticLimits {
                operations: work.operations(),
                ..ArithmeticLimits::default()
            },
            &control,
        );
        input.admit(&control, &mut bounded).unwrap();
        assert!(matches!(
            input.admit(&control, &mut bounded),
            Err(crate::Error::Event(Error::Capacity(
                Capacity::ArithmeticSteps
            )))
        ));
        let zero = ExpectationInput {
            given: source.empty(),
            payoffs: vec![(PayoffInput::Ratio([0, 0, u64::MAX]), source.empty())],
        };
        let mut narrow = ExactArithmetic::new(
            ArithmeticLimits {
                bits: 16,
                ..ArithmeticLimits::default()
            },
            &control,
        );
        assert!(matches!(
            zero.admit(&control, &mut narrow),
            Err(crate::Error::Event(Error::Capacity(
                Capacity::ArithmeticBits
            )))
        ));
    }
}
