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

/// Context-checked claims retain exact fraction presentations until the shared
/// finalization budget can normalize them. Empty regions still supply a value.
#[derive(Debug, Clone)]
pub(crate) struct ExpectationInput {
    pub given: Event,
    // sign, numerator magnitude, denominator magnitude; sign is 0 or 1.
    pub payoffs: Vec<([u64; 3], Event)>,
}

/// A checked roster retains supplied zero payoffs and empty indexed buckets.
#[derive(Debug, Clone)]
pub(crate) struct AdmittedExpectationInput {
    pub partition: crate::event::EventPartition,
    pub values: Vec<ExactRational>,
}

impl ExpectationInput {
    pub(crate) fn admit(
        &self,
        control: &crate::WorkContext,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedExpectationInput> {
        use crate::event::{BoolOp4, EventPartition, PartitionLimits};
        let mut merged = std::collections::HashMap::<Vec<u8>, (ExactRational, Event)>::new();
        for ([sign, numerator, denominator], region) in &self.payoffs {
            // No empty-region or zero-numerator shortcut may hide an invalid divisor.
            let value =
                ExactRational::from(*numerator).div(&ExactRational::from(*denominator), work)?;
            let value = if *sign == 0 {
                value
            } else {
                ExactRational::zero().sub(&value, work)?
            };
            let key = value.to_bytes(work)?;
            if let Some((_, prior)) = merged.get_mut(&key) {
                *prior = prior.apply(BoolOp4::OR, region, control)?;
            } else {
                merged.try_reserve(1).map_err(crate::event::Error::from)?;
                merged.insert(key, (value, region.clone()));
            }
        }
        let mut regions = Vec::new();
        let mut values = Vec::new();
        regions
            .try_reserve_exact(merged.len())
            .map_err(crate::event::Error::from)?;
        values
            .try_reserve_exact(merged.len())
            .map_err(crate::event::Error::from)?;
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(merged.len())
            .map_err(crate::event::Error::from)?;
        ordered.extend(merged);
        ordered.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        for (_, (value, region)) in ordered {
            values.push(value);
            regions.push(region);
        }
        Ok(AdmittedExpectationInput {
            partition: EventPartition::on(
                &self.given,
                &regions,
                PartitionLimits::default(),
                control,
            )?,
            values,
        })
    }
}

/// An owned conditional expectation, including its complete evidence-relative
/// payoff. Construction is native and requires structural partition admission.
#[derive(Debug, Clone)]
pub struct ExpectationAnswer {
    input: AdmittedExpectationInput,
    value: ExpectationValue,
}

impl ExpectationAnswer {
    pub(crate) fn new(
        input: AdmittedExpectationInput,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
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
            payoffs: vec![([0, 1, 2], source.full()), ([0, 2, 4], source.full())],
        };
        let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &control);
        let admitted = input.admit(&control, &mut work).unwrap();
        assert_eq!(admitted.values.len(), 1);
        assert_eq!(admitted.values[0].to_string(), "1/2");
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
            payoffs: vec![([0, 0, u64::MAX], source.empty())],
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
