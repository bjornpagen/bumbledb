use super::{
    AdmittedFamilyDescriptor, Budget, Capacity, Control, Error, ExactArithmetic,
    FamilyFunctionDescriptor, ParameterFunctionDescriptor, ParameterRegion, Result,
    SourceDescriptorLimits, SpaceId, event, full, region, region_claim, roster, shape, space_claim,
};
use crate::{EventPartition, ParameterRevisedSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyReceiptDescriptor {
    Condition {
        evidence: Vec<u8>,
        mass: ParameterFunctionDescriptor,
    },
    Likelihood {
        factor: FamilyFunctionDescriptor,
        normalizer: ParameterFunctionDescriptor,
    },
    Jeffrey {
        cells: Vec<Vec<u8>>,
        old_masses: Vec<ParameterFunctionDescriptor>,
        targets: Vec<ParameterFunctionDescriptor>,
        unsupported: Vec<Vec<u8>>,
    },
}
/// Both full contexts are claims. Replaying the update reconstructs the source,
/// guard refinement and original-prior translation before checking these bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyPosteriorDescriptor {
    pub refined_prior: Vec<u8>,
    pub posterior: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyRevisionDescriptor {
    pub identity: SpaceId,
    pub prior: Vec<u8>,
    pub receipt: FamilyReceiptDescriptor,
    pub defined: Vec<u8>,
    /// None asserts that every original parameter is unsupported for this update.
    pub outcome: Option<FamilyPosteriorDescriptor>,
}

impl FamilyRevisionDescriptor {
    pub(super) fn capture(
        value: &AdmittedFamilyDescriptor,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        let (identity, prior, defined, revised, receipt) = match value {
            AdmittedFamilyDescriptor::Conditioning(r) => (
                r.identity(),
                r.prior(),
                r.defined_on(),
                r.revised(),
                FamilyReceiptDescriptor::Condition {
                    evidence: budget.event(r.evidence(), work.control())?,
                    mass: ParameterFunctionDescriptor::capture(
                        r.evidence_mass(),
                        limits,
                        budget,
                        work,
                    )?,
                },
            ),
            AdmittedFamilyDescriptor::Likelihood(r) => (
                r.identity(),
                r.prior(),
                r.defined_on(),
                r.revised(),
                FamilyReceiptDescriptor::Likelihood {
                    factor: FamilyFunctionDescriptor::capture(
                        r.likelihood(),
                        limits,
                        budget,
                        work,
                    )?,
                    normalizer: ParameterFunctionDescriptor::capture(
                        r.normalizer(),
                        limits,
                        budget,
                        work,
                    )?,
                },
            ),
            AdmittedFamilyDescriptor::Jeffrey(r) => {
                shape(
                    r.partition().cells().len(),
                    limits.partitions.cells,
                    Capacity::PartitionCells,
                )?;
                roster(
                    r.partition().cells().len(),
                    4,
                    limits.parameters.functions.cells,
                    budget,
                )?;
                let (mut cells, mut old_masses, mut targets, mut unsupported) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                cells.try_reserve_exact(r.targets().len())?;
                old_masses.try_reserve_exact(r.targets().len())?;
                targets.try_reserve_exact(r.targets().len())?;
                unsupported.try_reserve_exact(r.targets().len())?;
                for (i, cell) in r.partition().cells().iter().enumerate() {
                    cells.push(budget.event(cell, work.control())?);
                    old_masses.push(ParameterFunctionDescriptor::capture(
                        &r.old_masses()[i],
                        limits,
                        budget,
                        work,
                    )?);
                    targets.push(ParameterFunctionDescriptor::capture(
                        &r.targets()[i],
                        limits,
                        budget,
                        work,
                    )?);
                    unsupported.push(region(&r.unsupported_regions()[i], limits, budget, work)?);
                }
                (
                    r.identity(),
                    r.prior(),
                    r.defined_on(),
                    r.revised(),
                    FamilyReceiptDescriptor::Jeffrey {
                        cells,
                        old_masses,
                        targets,
                        unsupported,
                    },
                )
            }
            _ => return Err(Error::InvalidEncoding),
        };
        Ok(Self {
            identity,
            prior: budget.event(&prior.full(), work.control())?,
            receipt,
            defined: region(defined, limits, budget, work)?,
            outcome: revised
                .map(|r| -> Result<_> {
                    Ok(FamilyPosteriorDescriptor {
                        refined_prior: budget.event(&r.refined_prior().full(), work.control())?,
                        posterior: budget.event(&r.space().full(), work.control())?,
                    })
                })
                .transpose()?,
        })
    }
    pub(super) fn preflight(
        &self,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        control: &dyn Control,
    ) -> Result<()> {
        budget.item(0)?;
        budget.item(self.prior.len())?;
        match &self.receipt {
            FamilyReceiptDescriptor::Condition { evidence, mass } => {
                budget.item(evidence.len())?;
                mass.preflight(limits, budget, control)?;
            }
            FamilyReceiptDescriptor::Likelihood { factor, normalizer } => {
                factor.preflight(limits, budget, control)?;
                normalizer.preflight(limits, budget, control)?;
            }
            FamilyReceiptDescriptor::Jeffrey {
                cells,
                old_masses,
                targets,
                unsupported,
            } => {
                if cells.len() != old_masses.len()
                    || cells.len() != targets.len()
                    || cells.len() != unsupported.len()
                {
                    return Err(Error::PartitionArity);
                }
                shape(
                    cells.len(),
                    limits.partitions.cells,
                    Capacity::PartitionCells,
                )?;
                roster(cells.len(), 4, limits.parameters.functions.cells, budget)?;
                for i in 0..cells.len() {
                    control.checkpoint()?;
                    budget.item(cells[i].len())?;
                    old_masses[i].preflight(limits, budget, control)?;
                    targets[i].preflight(limits, budget, control)?;
                    budget.item(unsupported[i].len())?;
                }
            }
        }
        budget.item(self.defined.len())?;
        if let Some(r) = &self.outcome {
            budget.item(r.refined_prior.len())?;
            budget.item(r.posterior.len())?;
        }
        Ok(())
    }
    pub(super) fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedFamilyDescriptor> {
        let prior = full(&self.prior, limits, work)?;
        match &self.receipt {
            FamilyReceiptDescriptor::Condition { evidence, mass } => {
                let evidence = event(evidence, limits, work)?;
                let r =
                    prior.parameter_condition(self.identity, &evidence, limits.parameters, work)?;
                mass.check(r.evidence_mass(), limits, work)?;
                self.check(r.defined_on(), r.revised(), limits, work)?;
                Ok(AdmittedFamilyDescriptor::Conditioning(r))
            }
            FamilyReceiptDescriptor::Likelihood { factor, normalizer } => {
                let factor = factor.admit(limits, work)?;
                let r =
                    prior.parameter_likelihood(self.identity, &factor, limits.parameters, work)?;
                normalizer.check(r.normalizer(), limits, work)?;
                self.check(r.defined_on(), r.revised(), limits, work)?;
                Ok(AdmittedFamilyDescriptor::Likelihood(r))
            }
            FamilyReceiptDescriptor::Jeffrey {
                cells,
                old_masses,
                targets,
                unsupported,
            } => {
                let mut admitted_cells = Vec::new();
                admitted_cells.try_reserve_exact(cells.len())?;
                let mut admitted_targets = Vec::new();
                admitted_targets.try_reserve_exact(cells.len())?;
                for (cell, target) in cells.iter().zip(targets) {
                    admitted_cells.push(event(cell, limits, work)?);
                    admitted_targets.push(target.admit(limits, work)?);
                }
                let partition = EventPartition::on(
                    &prior.full(),
                    &admitted_cells,
                    limits.partitions,
                    work.control(),
                )?;
                let r = prior.parameter_jeffrey(
                    self.identity,
                    &partition,
                    &admitted_targets,
                    limits.parameters,
                    work,
                )?;
                for i in 0..cells.len() {
                    old_masses[i].check(&r.old_masses()[i], limits, work)?;
                    region_claim(&unsupported[i], &r.unsupported_regions()[i], limits, work)?;
                }
                self.check(r.defined_on(), r.revised(), limits, work)?;
                Ok(AdmittedFamilyDescriptor::Jeffrey(r))
            }
        }
    }
    fn check(
        &self,
        defined: &ParameterRegion,
        revised: Option<&ParameterRevisedSource>,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        region_claim(&self.defined, defined, limits, work)?;
        match (&self.outcome, revised) {
            (None, None) => Ok(()),
            (Some(expected), Some(actual)) => {
                space_claim(
                    &expected.refined_prior,
                    actual.refined_prior(),
                    limits,
                    work,
                )?;
                space_claim(&expected.posterior, actual.space(), limits, work)
            }
            _ => Err(Error::DescriptorClaimMismatch),
        }
    }
}
