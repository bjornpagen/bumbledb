//! Context admission precedes this module. One finalization arithmetic budget
//! resolves scalars, glues local functions and measures the resulting observable.
use crate::event::{
    BoolOp4, EventPartition, ExactArithmetic, ExactRational, FamilyFunction, FiniteFunction,
    FunctionCover, FunctionLimits, FunctionPatch, FunctionPiece, ParameterSourceLimits,
    PartitionLimits,
};
use crate::{Event, ImportedPayoff, PayoffImport, Result};

#[derive(Debug, Clone)]
pub(crate) enum PayoffInput {
    Ratio([u64; 3]),
    Imported(PayoffImport),
}
impl PayoffInput {
    fn scalar(&self, work: &mut ExactArithmetic<'_>) -> Result<Option<ExactRational>> {
        Ok(match self {
            Self::Ratio([sign, numerator, denominator]) => {
                let value = ExactRational::from(*numerator)
                    .div(&ExactRational::from(*denominator), work)?;
                Some(if *sign == 0 {
                    value
                } else {
                    ExactRational::zero().sub(&value, work)?
                })
            }
            Self::Imported(import) => match import.value() {
                ImportedPayoff::Rational(value) => Some(value.clone()),
                _ => None,
            },
        })
    }
    fn is_function(&self) -> bool {
        matches!(self, Self::Imported(value) if !matches!(value.value(), ImportedPayoff::Rational(_)))
    }
    fn is_family(&self) -> bool {
        matches!(self, Self::Imported(value) if matches!(value.value(), ImportedPayoff::Family(_)))
    }
    fn finite(&self, given: &Event, work: &mut ExactArithmetic<'_>) -> Result<FiniteFunction> {
        if let Some(value) = self.scalar(work)? {
            return Ok(FiniteFunction::constant(
                &given.space(),
                value,
                FunctionLimits::default(),
                work,
            )?);
        }
        let Self::Imported(import) = self else {
            unreachable!("scalar")
        };
        let ImportedPayoff::Finite(value) = import.value() else {
            unreachable!("finite payoff")
        };
        Ok(value.clone())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExpectationInput {
    pub given: Event,
    pub payoffs: Vec<(PayoffInput, Event)>,
}

/// Scalar rosters and function covers are distinct admitted objects. A family
/// cover can contain explicitly promoted finite functions without choosing a prior.
#[derive(Debug, Clone)]
pub enum ExpectationPayoff {
    Scalar {
        partition: EventPartition,
        values: Vec<ExactRational>,
    },
    Finite(FunctionCover<FiniteFunction>),
    Family(FunctionCover<FamilyFunction>),
}
impl ExpectationPayoff {
    #[must_use]
    pub fn given(&self) -> &Event {
        match self {
            Self::Scalar { partition, .. } => partition.parent(),
            Self::Finite(cover) => cover.parent(),
            Self::Family(cover) => cover.parent(),
        }
    }
    pub(super) fn finite_function(&self, work: &mut ExactArithmetic<'_>) -> Result<FiniteFunction> {
        match self {
            Self::Finite(cover) => Ok(cover.function().clone()),
            Self::Family(_) => unreachable!("family contraction"),
            Self::Scalar { partition, values } => {
                let mut pieces = Vec::new();
                pieces
                    .try_reserve_exact(values.len())
                    .map_err(crate::event::Error::from)?;
                pieces.extend(partition.cells().iter().zip(values).map(|(region, value)| {
                    FunctionPiece {
                        region: region.clone(),
                        value: value.clone(),
                    }
                }));
                Ok(FiniteFunction::new(
                    &partition.parent().space(),
                    &pieces,
                    FunctionLimits::default(),
                    work,
                )?)
            }
        }
    }
}

impl ExpectationInput {
    pub(crate) fn admit(
        &self,
        control: &crate::WorkContext,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ExpectationPayoff> {
        if self.payoffs.iter().any(|(p, _)| p.is_family()) {
            let mut patches = Vec::new();
            patches
                .try_reserve_exact(self.payoffs.len())
                .map_err(crate::event::Error::from)?;
            for (value, region) in &self.payoffs {
                let function = if let PayoffInput::Imported(import) = value {
                    if let ImportedPayoff::Family(function) = import.value() {
                        function.clone()
                    } else {
                        FamilyFunction::from_finite(
                            &value.finite(&self.given, work)?,
                            ParameterSourceLimits::default(),
                            work,
                        )?
                    }
                } else {
                    FamilyFunction::from_finite(
                        &value.finite(&self.given, work)?,
                        ParameterSourceLimits::default(),
                        work,
                    )?
                };
                patches.push(FunctionPatch {
                    region: region.clone(),
                    function,
                });
            }
            return Ok(ExpectationPayoff::Family(FamilyFunction::glue(
                &self.given,
                &patches,
                ParameterSourceLimits::default(),
                work,
            )?));
        }
        if self.payoffs.iter().any(|(p, _)| p.is_function()) {
            let mut patches = Vec::new();
            patches
                .try_reserve_exact(self.payoffs.len())
                .map_err(crate::event::Error::from)?;
            for (value, region) in &self.payoffs {
                patches.push(FunctionPatch {
                    region: region.clone(),
                    function: value.finite(&self.given, work)?,
                });
            }
            return Ok(ExpectationPayoff::Finite(FiniteFunction::glue(
                &self.given,
                &patches,
                FunctionLimits::default(),
                work,
            )?));
        }
        let mut merged = std::collections::HashMap::<Vec<u8>, (ExactRational, Event)>::new();
        for (input, region) in &self.payoffs {
            let value = input.scalar(work)?.expect("scalar roster");
            let key = value.to_bytes(work)?;
            if let Some((_, prior)) = merged.get_mut(&key) {
                *prior = prior.apply(BoolOp4::OR, region, control)?;
            } else {
                merged.try_reserve(1).map_err(crate::event::Error::from)?;
                merged.insert(key, (value, region.clone()));
            }
        }
        let mut ordered = Vec::new();
        ordered
            .try_reserve_exact(merged.len())
            .map_err(crate::event::Error::from)?;
        ordered.extend(merged);
        ordered.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let mut regions = Vec::new();
        let mut values = Vec::new();
        regions
            .try_reserve_exact(ordered.len())
            .map_err(crate::event::Error::from)?;
        values
            .try_reserve_exact(ordered.len())
            .map_err(crate::event::Error::from)?;
        for (_, (value, region)) in ordered {
            values.push(value);
            regions.push(region);
        }
        Ok(ExpectationPayoff::Scalar {
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
