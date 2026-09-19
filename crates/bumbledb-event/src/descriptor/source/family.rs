//! Shared-parameter objects in BESC v2. Plain data never certifies arithmetic,
//! map cells, refinements, normalized channels or recorded update claims.
use super::{Budget, SourceDescriptorLimits, claim, shape};
use crate::parameter::source::wire::{decode_function, encode_function};
use crate::{
    BetaSource, Capacity, Control, Error, ExactArithmetic, ExactRational, FamilyFunction,
    FamilyFunctionPiece, FamilyKernel, MapDescriptor, ParameterConditioning, ParameterDomain,
    ParameterFunction, ParameterJeffrey, ParameterLikelihood, ParameterRefinement, ParameterRegion,
    ParameterRestriction, Result, Space, SpaceId,
};

mod revision;
pub(super) mod wire;
pub use revision::{FamilyPosteriorDescriptor, FamilyReceiptDescriptor, FamilyRevisionDescriptor};

/// Partial parameter function: BEPR ambient domain and ordered BEGF definitions.
/// Holes remain undefined; they are not an implicit zero default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterFunctionDescriptor {
    pub ambient: Vec<u8>,
    pub pieces: Vec<Vec<u8>>,
}
/// Total signed function: full BEVT space, disjoint cells and BEGF coefficients.
/// Explicit zero definitions survive; omitted worlds have the zero default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyFunctionDescriptor {
    pub space: Vec<u8>,
    pub pieces: Vec<super::FunctionPieceDescriptor>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyKernelDescriptor {
    pub parent: MapDescriptor,
    pub density: FamilyFunctionDescriptor,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefinementDescriptor {
    pub identity: SpaceId,
    pub source: Vec<u8>,
    pub predicates: Vec<Vec<u8>>,
    pub refined: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestrictionDescriptor {
    pub refinement: RefinementDescriptor,
    pub predicate: Vec<u8>,
    pub restricted: Vec<u8>,
}
/// Explicit Beta commitment retaining the original full measured source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetaSourceDescriptor {
    pub source: Vec<u8>,
    pub alpha: Vec<u8>,
    pub beta: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyDescriptor {
    Parameter(ParameterFunctionDescriptor),
    Function(FamilyFunctionDescriptor),
    Kernel(FamilyKernelDescriptor),
    Refinement(RefinementDescriptor),
    Restriction(RestrictionDescriptor),
    Revision(FamilyRevisionDescriptor),
    Beta(BetaSourceDescriptor),
}
#[derive(Debug, Clone)]
pub enum AdmittedFamilyDescriptor {
    Parameter(ParameterFunction),
    Function(FamilyFunction),
    Kernel(FamilyKernel),
    Refinement(ParameterRefinement),
    Restriction(ParameterRestriction),
    Conditioning(ParameterConditioning),
    Likelihood(ParameterLikelihood),
    Jeffrey(ParameterJeffrey),
    Beta(BetaSource),
}

fn blob(value: Vec<u8>, budget: &mut Budget) -> Result<Vec<u8>> {
    budget.item(value.len())?;
    Ok(value)
}
fn region(
    value: &ParameterRegion,
    limits: SourceDescriptorLimits,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    blob(value.to_bytes(limits.parameters.parameters, work)?, budget)
}
fn event(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<crate::Event> {
    crate::Event::from_bytes_with_parameter_limits(
        bytes,
        None,
        limits.descriptors.events,
        limits.parameters,
        work,
    )
}
fn full(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Space> {
    let value = event(bytes, limits, work)?;
    if !value.is_full() {
        return Err(Error::InvalidEncoding);
    }
    value
        .space()
        .parameter_domain()
        .ok_or(Error::MissingParameter)?;
    Ok(value.space())
}
fn space_claim(
    bytes: &[u8],
    actual: &Space,
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<()> {
    match full(bytes, limits, work)?
        .full()
        .align_to(actual, work.control())
    {
        Ok(_) => Ok(()),
        Err(Error::SpaceMismatch) => Err(Error::DescriptorClaimMismatch),
        Err(e) => Err(e),
    }
}
fn region_claim(
    bytes: &[u8],
    actual: &ParameterRegion,
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<()> {
    let expected = ParameterRegion::from_bytes(bytes, limits.parameters.parameters, work)?;
    claim(expected.equivalent(actual, limits.parameters.parameters.region, work)?)
}
fn roster(n: usize, units: usize, limit: usize, budget: &Budget) -> Result<()> {
    shape(n, limit, Capacity::FunctionCells)?;
    shape(
        n,
        budget.limits.items.saturating_sub(budget.items) / units,
        Capacity::DescriptorItems,
    )
}

impl ParameterFunctionDescriptor {
    fn capture(
        f: &ParameterFunction,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        let ambient = region(f.ambient().region(), limits, budget, work)?;
        roster(
            f.pieces().len(),
            1,
            limits.parameters.functions.cells,
            budget,
        )?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(f.pieces().len())?;
        for part in f.pieces() {
            pieces.push(blob(
                encode_function(part, limits.parameters, work)?,
                budget,
            )?);
        }
        Ok(Self { ambient, pieces })
    }
    fn preflight(
        &self,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        control: &dyn Control,
    ) -> Result<()> {
        budget.item(0)?;
        budget.item(self.ambient.len())?;
        roster(
            self.pieces.len(),
            1,
            limits.parameters.functions.cells,
            budget,
        )?;
        for p in &self.pieces {
            control.checkpoint()?;
            budget.item(p.len())?;
        }
        Ok(())
    }
    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterFunction> {
        let domain =
            ParameterDomain::from_bytes(&self.ambient, limits.parameters.parameters, work)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.pieces.len())?;
        for p in &self.pieces {
            pieces.push(decode_function(p, &domain, limits.parameters, work)?);
        }
        ParameterFunction::new(
            domain,
            &pieces,
            limits.parameters.parameters.region,
            limits.parameters.functions,
            work,
        )
    }
    fn check(
        &self,
        actual: &ParameterFunction,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        claim(self.admit(limits, work)?.equivalent(
            actual,
            limits.parameters.parameters.region,
            limits.parameters.functions,
            work,
        )?)
    }
}
impl FamilyFunctionDescriptor {
    fn capture(
        f: &FamilyFunction,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        let space = budget.event(&f.space().full(), work.control())?;
        roster(
            f.pieces().len(),
            2,
            limits.parameters.functions.cells,
            budget,
        )?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(f.pieces().len())?;
        for (region, value) in f.pieces() {
            pieces.push(super::FunctionPieceDescriptor {
                region: budget.event(&region, work.control())?,
                value: blob(encode_function(value, limits.parameters, work)?, budget)?,
            });
        }
        Ok(Self { space, pieces })
    }
    fn preflight(
        &self,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        control: &dyn Control,
    ) -> Result<()> {
        budget.item(0)?;
        budget.item(self.space.len())?;
        roster(
            self.pieces.len(),
            2,
            limits.parameters.functions.cells,
            budget,
        )?;
        for p in &self.pieces {
            control.checkpoint()?;
            budget.item(p.region.len())?;
            budget.item(p.value.len())?;
        }
        Ok(())
    }
    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<FamilyFunction> {
        let space = full(&self.space, limits, work)?;
        let domain = space.parameter_domain().ok_or(Error::MissingParameter)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.pieces.len())?;
        for p in &self.pieces {
            pieces.push(FamilyFunctionPiece {
                region: event(&p.region, limits, work)?,
                value: decode_function(&p.value, domain, limits.parameters, work)?,
            });
        }
        FamilyFunction::new(&space, &pieces, limits.parameters, work)
    }
}
impl RefinementDescriptor {
    fn capture(
        r: &ParameterRefinement,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        let source = budget.event(&r.source().full(), work.control())?;
        let mut predicates = Vec::new();
        let guards = r
            .refined()
            .parameter_guards()
            .ok_or(Error::MissingParameter)?;
        let additional = guards
            .iter()
            .filter(|g| g.coordinate >= r.source().dimensions());
        predicates.try_reserve_exact(usize::from(
            r.refined().dimensions() - r.source().dimensions(),
        ))?;
        for g in additional {
            predicates.push(region(&g.region, limits, budget, work)?);
        }
        Ok(Self {
            identity: r.refined().identity(),
            source,
            predicates,
            refined: budget.event(&r.refined().full(), work.control())?,
        })
    }
    fn preflight(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        budget.item(0)?;
        budget.item(self.source.len())?;
        for p in &self.predicates {
            control.checkpoint()?;
            budget.item(p.len())?;
        }
        budget.item(self.refined.len())
    }
    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterRefinement> {
        let source = full(&self.source, limits, work)?;
        let mut predicates = Vec::new();
        predicates.try_reserve_exact(self.predicates.len())?;
        for p in &self.predicates {
            predicates.push(ParameterRegion::from_bytes(
                p,
                limits.parameters.parameters,
                work,
            )?);
        }
        let r = ParameterRefinement::with_limits(
            self.identity,
            &source,
            &predicates,
            limits.descriptors.events,
            limits.parameters,
            work,
        )?;
        space_claim(&self.refined, r.refined(), limits, work)?;
        Ok(r)
    }
}
impl RestrictionDescriptor {
    fn capture(
        r: &ParameterRestriction,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        Ok(Self {
            refinement: RefinementDescriptor::capture(r.refinement(), limits, budget, work)?,
            predicate: region(
                r.space()
                    .parameter_domain()
                    .ok_or(Error::MissingParameter)?
                    .region(),
                limits,
                budget,
                work,
            )?,
            restricted: budget.event(&r.space().full(), work.control())?,
        })
    }
    fn preflight(&self, budget: &mut Budget, control: &dyn Control) -> Result<()> {
        budget.item(0)?;
        self.refinement.preflight(budget, control)?;
        budget.item(self.predicate.len())?;
        budget.item(self.restricted.len())
    }
    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterRestriction> {
        let refinement = self.refinement.admit(limits, work)?;
        let predicate =
            ParameterRegion::from_bytes(&self.predicate, limits.parameters.parameters, work)?;
        let r = ParameterRestriction::from_refinement(
            &refinement,
            &predicate,
            limits.parameters,
            work,
        )?;
        space_claim(&self.restricted, r.space(), limits, work)?;
        Ok(r)
    }
}

impl BetaSourceDescriptor {
    fn capture(
        value: &BetaSource,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        Ok(Self {
            source: budget.event(&value.source().full(), work.control())?,
            alpha: blob(value.alpha().to_bytes(work)?, budget)?,
            beta: blob(value.beta().to_bytes(work)?, budget)?,
        })
    }
    fn preflight(&self, budget: &mut Budget) -> Result<()> {
        budget.item(0)?;
        budget.item(self.source.len())?;
        budget.item(self.alpha.len())?;
        budget.item(self.beta.len())
    }
    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<BetaSource> {
        let source = full(&self.source, limits, work)?;
        let alpha = ExactRational::from_bytes(&self.alpha, work)?;
        let beta = ExactRational::from_bytes(&self.beta, work)?;
        BetaSource::new(&source, alpha, beta, limits.parameters, work)
    }
}

impl FamilyDescriptor {
    pub(super) fn capture(
        value: &AdmittedFamilyDescriptor,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Ok(match value {
            AdmittedFamilyDescriptor::Parameter(f) => Self::Parameter(
                ParameterFunctionDescriptor::capture(f, limits, budget, work)?,
            ),
            AdmittedFamilyDescriptor::Function(f) => {
                Self::Function(FamilyFunctionDescriptor::capture(f, limits, budget, work)?)
            }
            AdmittedFamilyDescriptor::Kernel(k) => {
                budget.item(0)?;
                Self::Kernel(FamilyKernelDescriptor {
                    parent: MapDescriptor::capture(k.parent().map(), budget, work.control())?,
                    density: FamilyFunctionDescriptor::capture(k.density(), limits, budget, work)?,
                })
            }
            AdmittedFamilyDescriptor::Refinement(r) => {
                Self::Refinement(RefinementDescriptor::capture(r, limits, budget, work)?)
            }
            AdmittedFamilyDescriptor::Restriction(r) => {
                Self::Restriction(RestrictionDescriptor::capture(r, limits, budget, work)?)
            }
            AdmittedFamilyDescriptor::Beta(b) => {
                Self::Beta(BetaSourceDescriptor::capture(b, budget, work)?)
            }
            r => Self::Revision(FamilyRevisionDescriptor::capture(r, limits, budget, work)?),
        })
    }
    pub(super) fn preflight(
        &self,
        limits: SourceDescriptorLimits,
        budget: &mut Budget,
        control: &dyn Control,
    ) -> Result<()> {
        match self {
            Self::Parameter(f) => f.preflight(limits, budget, control),
            Self::Function(f) => f.preflight(limits, budget, control),
            Self::Kernel(k) => {
                budget.item(0)?;
                k.parent.preflight(budget, control)?;
                k.density.preflight(limits, budget, control)
            }
            Self::Refinement(r) => r.preflight(budget, control),
            Self::Restriction(r) => r.preflight(budget, control),
            Self::Revision(r) => r.preflight(limits, budget, control),
            Self::Beta(b) => b.preflight(budget),
        }
    }
    pub(super) fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedFamilyDescriptor> {
        Ok(match self {
            Self::Parameter(f) => AdmittedFamilyDescriptor::Parameter(f.admit(limits, work)?),
            Self::Function(f) => AdmittedFamilyDescriptor::Function(f.admit(limits, work)?),
            Self::Kernel(k) => {
                let parent = k.parent.admit_with_parameters(limits, work)?;
                let density = k.density.admit(limits, work)?;
                AdmittedFamilyDescriptor::Kernel(FamilyKernel::new(
                    &parent,
                    &density,
                    limits.parameters,
                    work,
                )?)
            }
            Self::Refinement(r) => AdmittedFamilyDescriptor::Refinement(r.admit(limits, work)?),
            Self::Restriction(r) => AdmittedFamilyDescriptor::Restriction(r.admit(limits, work)?),
            Self::Revision(r) => r.admit(limits, work)?,
            Self::Beta(b) => AdmittedFamilyDescriptor::Beta(b.admit(limits, work)?),
        })
    }
}
impl MapDescriptor {
    /// Reconstruct a same-parameter map using explicit source limits and one
    /// arithmetic budget for all embedded domains, laws, guards and map checks.
    /// # Errors
    /// Malformed markers/readouts, context violations, capacities or cancellation.
    pub fn admit_with_parameters(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<crate::CoordinateMap> {
        self.preflight(&mut Budget::new(limits.descriptors), work.control())?;
        let source = full(&self.source, limits, work)?;
        let target = full(&self.target, limits, work)?;
        if self.readouts.len() != usize::from(target.dimensions()) {
            return Err(Error::MapArity);
        }
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(self.readouts.len())?;
        for r in &self.readouts {
            readouts.push(event(r, limits, work)?);
        }
        crate::CoordinateMap::new_with_parameters(
            &source,
            &target,
            &readouts,
            limits.parameters,
            work,
        )
    }
}
