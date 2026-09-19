//! Explicit fixed-law revisions. All preserve the original legal worlds and
//! retain their interpretation; structural restriction is a different operation.
use std::sync::Arc;

use crate::arena::MAX_COORDINATES;

use crate::{
    CoordinateMap, Error, Event, EventPartition, ExactArithmetic, ExactRational, FiniteFunction,
    FunctionLimits, FunctionPiece, LawLimits, PartitionLimits, Result, Space, SurjectiveMap,
};

/// Mathematical inputs and checks retained in the original designated context.
/// Provider metadata and actual observation identities belong to the application;
/// this receipt neither creates fresh observations nor deduplicates model calls.
#[derive(Debug, Clone)]
pub enum RevisionReceipt {
    Condition {
        evidence: Event,
        mass: ExactRational,
    },
    /// The supplied scale is retained. `normalizer` is not necessarily an
    /// observation probability: a nonnegative likelihood factor can exceed one.
    Likelihood {
        likelihood: FiniteFunction,
        normalizer: ExactRational,
    },
    /// No intrinsic evidence probability belongs to a Jeffrey replacement.
    Jeffrey {
        partition: EventPartition,
        old_masses: Arc<[ExactRational]>,
        targets: Arc<[ExactRational]>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionImpossible {
    ZeroEvidence,
    ZeroLikelihood,
    /// Every positive target whose old conditional is undefined. Indexed cells
    /// include empty and nonempty zero-mass regions; neither is silently repaired.
    UnsupportedTargets {
        cells: Arc<[usize]>,
    },
}

/// A changed law with a checked identity-on-worlds map from posterior to prior.
/// Pull back old Events through this map to use them in the new context. It is
/// structurally bijective, but need not preserve either designated probability.
#[derive(Debug, Clone)]
pub struct RevisedSource {
    translation: SurjectiveMap,
}
impl RevisedSource {
    #[must_use]
    pub fn space(&self) -> &Space {
        self.translation.map().source()
    }
    #[must_use]
    pub fn prior(&self) -> &Space {
        self.translation.map().target()
    }
    #[must_use]
    pub fn translation(&self) -> &CoordinateMap {
        self.translation.map()
    }
    #[must_use]
    pub fn translation_surjective(&self) -> &SurjectiveMap {
        &self.translation
    }

    fn new(prior: &Space, posterior: &Space, work: &mut ExactArithmetic<'_>) -> Result<Self> {
        let mut coordinates = [0; MAX_COORDINATES as usize];
        for bit in 0..prior.dimensions() {
            coordinates[usize::from(bit)] = bit;
        }
        let translation = CoordinateMap::coordinates(
            posterior,
            prior,
            &coordinates[..usize::from(prior.dimensions())],
            work.control(),
        )?
        .certify_surjective(work.control())?;
        Ok(Self { translation })
    }
}

#[derive(Debug, Clone)]
pub enum RevisionOutcome {
    Revised(RevisedSource),
    Impossible(RevisionImpossible),
}

/// A fully validated request and its owned result. Impossible requests retain
/// the prior and all mathematical inputs. Resource/context/malformed-input
/// failures are ordinary errors and do not masquerade as impossibility.
#[derive(Debug, Clone)]
pub struct SourceRevision {
    prior: Space,
    receipt: RevisionReceipt,
    outcome: RevisionOutcome,
}
impl SourceRevision {
    #[must_use]
    pub fn prior(&self) -> &Space {
        &self.prior
    }
    #[must_use]
    pub fn receipt(&self) -> &RevisionReceipt {
        &self.receipt
    }
    #[must_use]
    pub fn outcome(&self) -> &RevisionOutcome {
        &self.outcome
    }
    #[must_use]
    pub fn revised(&self) -> Option<&RevisedSource> {
        match &self.outcome {
            RevisionOutcome::Revised(source) => Some(source),
            RevisionOutcome::Impossible(_) => None,
        }
    }
}

impl Space {
    /// Materialize conditioning without removing zero-posterior legal worlds.
    /// Reusing the same Event is idempotent evidence, not a fresh draw.
    /// # Errors
    /// Wrong context, missing law, resource bounds or cancellation. Zero evidence
    /// is an owned impossible outcome, not an error or an empty posterior space.
    pub fn condition(
        &self,
        evidence: &Event,
        functions: FunctionLimits,
        laws: LawLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<SourceRevision> {
        let evidence = evidence.align_to(self, work.control())?;
        let mass = evidence.mass(work)?;
        let outcome = if mass.is_zero() {
            RevisionOutcome::Impossible(RevisionImpossible::ZeroEvidence)
        } else {
            let indicator = FiniteFunction::new(
                self,
                &[FunctionPiece {
                    region: evidence.clone(),
                    value: ExactRational::one(),
                }],
                functions,
                work,
            )?;
            self.reweight(&indicator, &mass, functions, laws, work)?
        };
        work.control().checkpoint()?;
        Ok(SourceRevision {
            prior: self.clone(),
            receipt: RevisionReceipt::Condition { evidence, mass },
            outcome,
        })
    }

    /// Apply an explicitly interpreted nonnegative likelihood/Pearl factor.
    /// Multiplying factors requires a joint observation interpretation supplied
    /// by the caller; this operation does not assert independent observations.
    /// # Errors
    /// Negative factors, context/missing law, limits or cancellation. An all-zero
    /// normalizer is an owned impossible result retaining the supplied scale.
    pub fn likelihood(
        &self,
        likelihood: &FiniteFunction,
        functions: FunctionLimits,
        laws: LawLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<SourceRevision> {
        let likelihood = likelihood.align_to(self, functions, work)?;
        if !likelihood.is_nonnegative() {
            return Err(Error::NegativeMass);
        }
        let normalizer = likelihood
            .expectation(&self.full(), work)?
            .numerator()
            .clone();
        let outcome = if normalizer.is_zero() {
            RevisionOutcome::Impossible(RevisionImpossible::ZeroLikelihood)
        } else {
            self.reweight(&likelihood, &normalizer, functions, laws, work)?
        };
        work.control().checkpoint()?;
        Ok(SourceRevision {
            prior: self.clone(),
            receipt: RevisionReceipt::Likelihood {
                likelihood,
                normalizer,
            },
            outcome,
        })
    }

    /// Replace a full partition's masses while preserving old within-cell
    /// conditionals. Positive targets require positive old mass. A zero target
    /// skips its undefined conditional; empty cells retain their roster positions.
    /// # Errors
    /// Partial/wrong-context partitions, target arity/negativity/non-normalization,
    /// missing law, limits or cancellation. Unsupported positive targets produce
    /// an owned impossible outcome listing every offending position.
    pub fn jeffrey(
        &self,
        partition: &EventPartition,
        targets: &[ExactRational],
        functions: FunctionLimits,
        laws: LawLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<SourceRevision> {
        partition.parent().align_to(self, work.control())?;
        if !partition.parent().is_full() {
            return Err(Error::PartitionGap);
        }
        if targets.len() != partition.cells().len() {
            return Err(Error::PartitionArity);
        }
        let partition = EventPartition::on(
            &self.full(),
            partition.cells(),
            PartitionLimits {
                cells: functions.cells,
            },
            work.control(),
        )?;
        let mut total = ExactRational::zero();
        for target in targets {
            if target.is_negative() {
                return Err(Error::NegativeMass);
            }
            total = total.add(target, work)?;
        }
        if total != ExactRational::one() {
            return Err(Error::LawNotNormalized);
        }
        let mut old_masses = Vec::new();
        let mut unsupported = Vec::new();
        old_masses.try_reserve_exact(targets.len())?;
        unsupported.try_reserve_exact(targets.len())?;
        for (index, (cell, target)) in partition.cells().iter().zip(targets).enumerate() {
            let mass = cell.mass(work)?;
            if mass.is_zero() && !target.is_zero() {
                unsupported.push(index);
            }
            old_masses.push(mass);
        }
        let outcome = if unsupported.is_empty() {
            let mut pieces = Vec::new();
            pieces.try_reserve_exact(targets.len())?;
            for ((region, old), target) in partition.cells().iter().zip(&old_masses).zip(targets) {
                work.control().checkpoint()?;
                if !target.is_zero() {
                    pieces.push(FunctionPiece {
                        region: region.clone(),
                        value: target.div(old, work)?,
                    });
                }
            }
            let factor = FiniteFunction::new(self, &pieces, functions, work)?;
            self.reweight(&factor, &ExactRational::one(), functions, laws, work)?
        } else {
            RevisionOutcome::Impossible(RevisionImpossible::UnsupportedTargets {
                cells: unsupported.into(),
            })
        };
        let mut owned_targets = Vec::new();
        owned_targets.try_reserve_exact(targets.len())?;
        owned_targets.extend_from_slice(targets);
        work.control().checkpoint()?;
        Ok(SourceRevision {
            prior: self.clone(),
            receipt: RevisionReceipt::Jeffrey {
                partition,
                old_masses: old_masses.into(),
                targets: owned_targets.into(),
            },
            outcome,
        })
    }

    fn reweight(
        &self,
        factor: &FiniteFunction,
        normalizer: &ExactRational,
        functions: FunctionLimits,
        laws: LawLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<RevisionOutcome> {
        let weighted =
            FiniteFunction::density(self, functions, work)?.multiply(factor, functions, work)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(weighted.pieces().len())?;
        for (region, density) in weighted.pieces() {
            pieces.push(FunctionPiece {
                region,
                value: density.div(normalizer, work)?,
            });
        }
        let posterior =
            FiniteFunction::new(self, &pieces, functions, work)?.designate(laws, work)?;
        Ok(RevisionOutcome::Revised(RevisedSource::new(
            self, &posterior, work,
        )?))
    }
}
