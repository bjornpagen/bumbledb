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
