//! BESC v2, fixed-depth family grammar. Blobs remain untrusted until replay.
use super::{
    Capacity, Error, FamilyDescriptor, FamilyFunctionDescriptor, FamilyKernelDescriptor,
    FamilyPosteriorDescriptor, FamilyReceiptDescriptor, FamilyRevisionDescriptor,
    ParameterFunctionDescriptor, RefinementDescriptor, RestrictionDescriptor, Result,
    SourceDescriptorLimits, SpaceId, roster, shape,
};
use crate::descriptor::wire::{Reader, Writer};

impl FamilyDescriptor {
    pub(in crate::descriptor::source) fn write(&self, out: &mut Writer<'_>) -> Result<()> {
        match self {
            Self::Parameter(f) => {
                out.put(&[0])?;
                out.parameter_function(f)?;
            }
            Self::Function(f) => {
                out.put(&[1])?;
                out.family_function(f)?;
            }
            Self::Kernel(k) => {
                out.put(&[2])?;
                out.map(&k.parent)?;
                out.family_function(&k.density)?;
            }
            Self::Refinement(r) => {
                out.put(&[3])?;
                out.refinement(r)?;
            }
            Self::Restriction(r) => {
                out.put(&[4])?;
                out.refinement(&r.refinement)?;
                out.blob(&r.predicate)?;
                out.blob(&r.restricted)?;
            }
            Self::Revision(r) => {
                out.put(&[5])?;
                out.put(&r.identity.0)?;
                out.blob(&r.prior)?;
                match &r.receipt {
                    FamilyReceiptDescriptor::Condition { evidence, mass } => {
                        out.put(&[0])?;
                        out.blob(evidence)?;
                        out.parameter_function(mass)?;
                    }
                    FamilyReceiptDescriptor::Likelihood { factor, normalizer } => {
                        out.put(&[1])?;
                        out.family_function(factor)?;
                        out.parameter_function(normalizer)?;
                    }
                    FamilyReceiptDescriptor::Jeffrey {
                        cells,
                        old_masses,
                        targets,
                        unsupported,
                    } => {
                        out.put(&[2])?;
                        out.number(cells.len())?;
                        for i in 0..cells.len() {
                            out.blob(&cells[i])?;
                            out.parameter_function(&old_masses[i])?;
                            out.parameter_function(&targets[i])?;
                            out.blob(&unsupported[i])?;
                        }
                    }
                }
                out.blob(&r.defined)?;
                match &r.outcome {
                    None => out.put(&[0])?,
                    Some(r) => {
                        out.put(&[1])?;
                        out.blob(&r.refined_prior)?;
                        out.blob(&r.posterior)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(in crate::descriptor::source) fn read(
        input: &mut Reader<'_>,
        limits: SourceDescriptorLimits,
    ) -> Result<Self> {
        input.budget.item(0)?;
        Ok(match input.take(1)?[0] {
            0 => Self::Parameter(input.parameter_function_body(limits)?),
            1 => Self::Function(input.family_function_body(limits)?),
            2 => Self::Kernel(FamilyKernelDescriptor {
                parent: input.map()?,
                density: input.family_function(limits)?,
            }),
            3 => Self::Refinement(input.refinement_body()?),
            4 => {
                input.budget.item(0)?;
                Self::Restriction(RestrictionDescriptor {
                    refinement: input.refinement_body()?,
                    predicate: input.blob()?,
                    restricted: input.blob()?,
                })
            }
            5 => Self::Revision(input.family_revision(limits)?),
            _ => return Err(Error::InvalidEncoding),
        })
    }
}
impl Writer<'_> {
    fn parameter_function(&mut self, f: &ParameterFunctionDescriptor) -> Result<()> {
        self.blob(&f.ambient)?;
        self.number(f.pieces.len())?;
        for p in &f.pieces {
            self.blob(p)?;
        }
        Ok(())
    }
    fn family_function(&mut self, f: &FamilyFunctionDescriptor) -> Result<()> {
        self.blob(&f.space)?;
        self.number(f.pieces.len())?;
        for p in &f.pieces {
            self.blob(&p.region)?;
            self.blob(&p.value)?;
        }
        Ok(())
    }
    fn refinement(&mut self, r: &RefinementDescriptor) -> Result<()> {
        self.put(&r.identity.0)?;
        self.blob(&r.source)?;
        self.number(r.predicates.len())?;
        for p in &r.predicates {
            self.blob(p)?;
        }
        self.blob(&r.refined)
    }
}
impl Reader<'_> {
    fn parameter_function(
        &mut self,
        limits: SourceDescriptorLimits,
    ) -> Result<ParameterFunctionDescriptor> {
        self.budget.item(0)?;
        self.parameter_function_body(limits)
    }
    fn parameter_function_body(
        &mut self,
        limits: SourceDescriptorLimits,
    ) -> Result<ParameterFunctionDescriptor> {
        let ambient = self.blob()?;
        let count = self.count()?;
        roster(count, 1, limits.parameters.functions.cells, &self.budget)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(count)?;
        for _ in 0..count {
            pieces.push(self.blob()?);
        }
        Ok(ParameterFunctionDescriptor { ambient, pieces })
    }
    fn family_function(
        &mut self,
        limits: SourceDescriptorLimits,
    ) -> Result<FamilyFunctionDescriptor> {
        self.budget.item(0)?;
        self.family_function_body(limits)
    }
    fn family_function_body(
        &mut self,
        limits: SourceDescriptorLimits,
    ) -> Result<FamilyFunctionDescriptor> {
        let space = self.blob()?;
        let count = self.count()?;
        roster(count, 2, limits.parameters.functions.cells, &self.budget)?;
        if count > self.bytes.len() / 16 {
            return Err(Error::InvalidEncoding);
        }
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(count)?;
        for _ in 0..count {
            pieces.push(crate::FunctionPieceDescriptor {
                region: self.blob()?,
                value: self.blob()?,
            });
        }
        Ok(FamilyFunctionDescriptor { space, pieces })
    }
    fn refinement_body(&mut self) -> Result<RefinementDescriptor> {
        let identity = SpaceId(
            self.take(32)?
                .try_into()
                .map_err(|_| Error::InvalidEncoding)?,
        );
        let source = self.blob()?;
        let count = self.count()?;
        let mut predicates = Vec::new();
        predicates.try_reserve_exact(count)?;
        for _ in 0..count {
            predicates.push(self.blob()?);
        }
        Ok(RefinementDescriptor {
            identity,
            source,
            predicates,
            refined: self.blob()?,
        })
    }
    fn family_revision(
        &mut self,
        limits: SourceDescriptorLimits,
    ) -> Result<FamilyRevisionDescriptor> {
        let identity = SpaceId(
            self.take(32)?
                .try_into()
                .map_err(|_| Error::InvalidEncoding)?,
        );
        let prior = self.blob()?;
        let receipt = match self.take(1)?[0] {
            0 => FamilyReceiptDescriptor::Condition {
                evidence: self.blob()?,
                mass: self.parameter_function(limits)?,
            },
            1 => FamilyReceiptDescriptor::Likelihood {
                factor: self.family_function(limits)?,
                normalizer: self.parameter_function(limits)?,
            },
            2 => {
                let count = self.count()?;
                shape(count, limits.partitions.cells, Capacity::PartitionCells)?;
                roster(count, 4, limits.parameters.functions.cells, &self.budget)?;
                // Each cell needs two blobs and two parameter-function headers.
                if count > self.bytes.len() / 48 {
                    return Err(Error::InvalidEncoding);
                }
                let (mut cells, mut old_masses, mut targets, mut unsupported) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new());
                cells.try_reserve_exact(count)?;
                old_masses.try_reserve_exact(count)?;
                targets.try_reserve_exact(count)?;
                unsupported.try_reserve_exact(count)?;
                for _ in 0..count {
                    cells.push(self.blob()?);
                    old_masses.push(self.parameter_function(limits)?);
                    targets.push(self.parameter_function(limits)?);
                    unsupported.push(self.blob()?);
                }
                FamilyReceiptDescriptor::Jeffrey {
                    cells,
                    old_masses,
                    targets,
                    unsupported,
                }
            }
            _ => return Err(Error::InvalidEncoding),
        };
        let defined = self.blob()?;
        let outcome = match self.take(1)?[0] {
            0 => None,
            1 => Some(FamilyPosteriorDescriptor {
                refined_prior: self.blob()?,
                posterior: self.blob()?,
            }),
            _ => return Err(Error::InvalidEncoding),
        };
        Ok(FamilyRevisionDescriptor {
            identity,
            prior,
            receipt,
            defined,
            outcome,
        })
    }
}
