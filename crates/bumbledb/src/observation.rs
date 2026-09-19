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
        let value = if event.space().parameter_domain().is_some() {
            ProbabilityValue::Parameter(event.parameter_probability(
                &given,
                ParameterSourceLimits::default(),
                work,
            )?)
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
}

/// A checked roster retains supplied zero payoffs and empty indexed buckets.
/// The function's zero extension outside evidence is never a coverage claim.
#[derive(Debug, Clone)]
pub(crate) struct ExpectationInput {
    pub partition: crate::event::EventPartition,
    pub values: Vec<ExactRational>,
}

/// An owned conditional expectation, including its complete evidence-relative
/// payoff. Construction is native and requires structural partition admission.
#[derive(Debug, Clone)]
pub struct ExpectationAnswer {
    input: ExpectationInput,
    value: ExpectationValue,
}

impl ExpectationAnswer {
    pub(crate) fn new(input: ExpectationInput, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        let given = input.partition.parent();
        let mut pieces = Vec::new();
        pieces
            .try_reserve_exact(input.values.len())
            .map_err(crate::event::Error::from)?;
        pieces.extend(
            input
                .partition
                .cells()
                .iter()
                .zip(&input.values)
                .map(|(region, value)| crate::event::FunctionPiece {
                    region: region.clone(),
                    value: value.clone(),
                }),
        );
        let function = crate::event::FiniteFunction::new(
            &given.space(),
            &pieces,
            crate::event::FunctionLimits::default(),
            work,
        )?;
        let value = if given.space().parameter_domain().is_some() {
            ExpectationValue::Parameter(function.parameter_expectation(
                given,
                ParameterSourceLimits::default(),
                work,
            )?)
        } else {
            let observation = function.expectation(given, work)?;
            let value = observation.value(work)?;
            ExpectationValue::Fixed { observation, value }
        };
        Ok(Self { input, value })
    }

    #[must_use]
    pub fn given(&self) -> &Event {
        self.input.partition.parent()
    }

    /// The partition and corresponding exact values include zero payoffs.
    #[must_use]
    pub fn partition(&self) -> &crate::event::EventPartition {
        &self.input.partition
    }

    #[must_use]
    pub fn values(&self) -> &[ExactRational] {
        &self.input.values
    }

    #[must_use]
    pub fn value(&self) -> &ExpectationValue {
        &self.value
    }

    #[must_use]
    pub fn function(&self) -> &crate::event::FiniteFunction {
        match &self.value {
            ExpectationValue::Fixed { observation, .. } => observation.function(),
            ExpectationValue::Parameter(observation) => observation.function(),
        }
    }

    #[must_use]
    pub fn is_impossible(&self) -> bool {
        match &self.value {
            ExpectationValue::Fixed { observation, .. } => observation.is_impossible(),
            ExpectationValue::Parameter(observation) => observation.is_impossible(),
        }
    }
}

/// Equality retains source and payoff identity; equal means are insufficient.
/// Canonical functions ignore empty buckets and retain every nonzero region.
impl PartialEq for ExpectationAnswer {
    fn eq(&self, other: &Self) -> bool {
        self.given() == other.given() && self.function().pieces().eq(other.function().pieces())
    }
}
impl Eq for ExpectationAnswer {}
