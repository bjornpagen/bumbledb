//! Explicit restriction of the parameter domain and conditional family updates.
//! Parameter restrictions retain the law at each remaining parameter value;
//! conditioning additionally reweights outcomes and retains its evidence mass.
use super::{Budget, ParameterDensityPiece, ParameterRefinement, ParameterSourceLimits};
use crate::{
    BoolOp4, Control, CoordinateMap, Error, Event, ExactArithmetic, ParameterDomain,
    ParameterFunction, ParameterRegion, PolynomialSigns, Result, Space, SpaceId,
};

/// A captured smaller parameter domain, with the original outcome fibres and
/// per-parameter law. A checked inclusion targets a refined prior presentation
/// in which the new domain is expressible. The original source remains owned.
#[derive(Debug, Clone)]
pub struct ParameterRestriction {
    refinement: ParameterRefinement,
    inclusion: CoordinateMap,
}

impl ParameterRestriction {
    /// Capture `domain & predicate` without deleting outcomes or conditioning
    /// their law. Empty parameter intersections refuse as source construction.
    /// # Errors
    /// Missing/foreign parameter, empty domain, capacities or cancellation.
    pub fn new(
        identity: SpaceId,
        source: &Space,
        predicate: &ParameterRegion,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        let domain = source.parameter_domain().ok_or(Error::MissingParameter)?;
        ParameterDomain::new(domain.region().apply(
            BoolOp4::AND,
            predicate,
            limits.parameters.region,
            work,
        )?)?;
        let refinement = ParameterRefinement::new(
            identity,
            source,
            std::slice::from_ref(predicate),
            limits,
            work,
        )?;
        let prior = refinement.refined();
        let guard = prior.parameter_event(predicate, limits, work)?;
        let restricted = prior.restrict_with_parameters(&guard, limits, work)?;
        let domain = restricted
            .parameter_domain()
            .ok_or(Error::MissingParameter)?;
        let space = if let Some(pieces) = prior.parameter_density_pieces() {
            let mut narrowed = Vec::new();
            narrowed.try_reserve_exact(pieces.len())?;
            for (region, density) in pieces {
                budget.step(control)?;
                narrowed.push(ParameterDensityPiece {
                    region: region.in_space(&restricted, control)?,
                    density: density.on_domain(domain, limits.parameters.region, work)?,
                });
            }
            restricted.with_parameter_density(&narrowed, limits, work)?
        } else {
            restricted
        };
        let inclusion = coordinate_inclusion(&space, prior, limits, work)?;
        Ok(Self {
            refinement,
            inclusion,
        })
    }

    #[must_use]
    pub fn prior(&self) -> &Space {
        self.refinement.source()
    }
    #[must_use]
    pub fn refined_prior(&self) -> &Space {
        self.refinement.refined()
    }
    #[must_use]
    pub fn space(&self) -> &Space {
        self.inclusion.source()
    }
    #[must_use]
    pub fn refinement(&self) -> &ParameterRefinement {
        &self.refinement
    }
    #[must_use]
    pub fn inclusion(&self) -> &CoordinateMap {
        &self.inclusion
    }

    /// Translate an Event from the original source to the captured subdomain.
    /// # Errors
    /// Foreign context, graph capacities or cancellation.
    pub fn pullback(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        self.inclusion
            .pullback(&self.refinement.lift(event, control)?, control)
    }
}

/// A family posterior and its checked translation into a refined prior.
/// The translation is not necessarily onto: zero-evidence parameter values are
/// outside the posterior domain. At admitted parameters, all outcomes remain.
#[derive(Debug, Clone)]
pub struct ParameterRevisedSource {
    restriction: ParameterRestriction,
    translation: CoordinateMap,
}
impl ParameterRevisedSource {
    #[must_use]
    pub fn prior(&self) -> &Space {
        self.restriction.prior()
    }
    #[must_use]
    pub fn refined_prior(&self) -> &Space {
        self.restriction.refined_prior()
    }
    #[must_use]
    pub fn space(&self) -> &Space {
        self.translation.source()
    }
    #[must_use]
    pub fn restriction(&self) -> &ParameterRestriction {
        &self.restriction
    }
    #[must_use]
    pub fn translation(&self) -> &CoordinateMap {
        &self.translation
    }
    /// Translate original prior Events, including zero-posterior possibilities.
    /// # Errors
    /// Foreign context, graph capacities or cancellation.
    pub fn pullback(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        self.translation
            .pullback(&self.restriction.refinement.lift(event, control)?, control)
    }
}

/// Owned conditioning receipt. An everywhere-zero evidence function has no
/// posterior, but retains its original source, Event and exact mass function.
/// Missing laws and operational failures remain errors, not impossible results.
#[derive(Debug, Clone)]
pub struct ParameterConditioning {
    prior: Space,
    evidence: Event,
    mass: ParameterFunction,
    positive: ParameterRegion,
    revised: Option<ParameterRevisedSource>,
}
impl ParameterConditioning {
    #[must_use]
    pub fn prior(&self) -> &Space {
        &self.prior
    }
    #[must_use]
    pub fn evidence(&self) -> &Event {
        &self.evidence
    }
    #[must_use]
    pub fn evidence_mass(&self) -> &ParameterFunction {
        &self.mass
    }
    #[must_use]
    pub fn defined_on(&self) -> &ParameterRegion {
        &self.positive
    }
    #[must_use]
    pub fn revised(&self) -> Option<&ParameterRevisedSource> {
        self.revised.as_ref()
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.revised.is_none()
    }
}

impl Space {
    /// Materialize exact family conditioning. Retain all outcomes at each
    /// positive-evidence parameter, including those receiving posterior zero.
    /// The caller names the refined presentation; no prior over parameters is
    /// introduced. Every zero-evidence parameter remains explicit in the receipt.
    /// # Errors
    /// Foreign evidence, missing parameter/law, capacities or cancellation.
    pub fn parameter_condition(
        &self,
        identity: SpaceId,
        evidence: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterConditioning> {
        let control = work.control();
        let evidence = evidence.align_to(self, control)?;
        let mass = evidence.parameter_mass(limits, work)?;
        let positive = mass.where_sign(
            PolynomialSigns::POSITIVE,
            limits.parameters.region,
            limits.functions,
            work,
        )?;
        let revised = if positive.is_empty() {
            None
        } else {
            let restriction = ParameterRestriction::new(identity, self, &positive, limits, work)?;
            let posterior = conditioned_law(&restriction, &evidence, &mass, limits, work)?;
            let translation =
                coordinate_inclusion(&posterior, restriction.refined_prior(), limits, work)?;
            Some(ParameterRevisedSource {
                restriction,
                translation,
            })
        };
        control.checkpoint()?;
        Ok(ParameterConditioning {
            prior: self.clone(),
            evidence,
            mass,
            positive,
            revised,
        })
    }
}

fn conditioned_law(
    restriction: &ParameterRestriction,
    evidence: &Event,
    mass: &ParameterFunction,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Space> {
    let control = work.control();
    let mut budget = Budget::new(limits, control)?;
    let space = restriction.space();
    let domain = space.parameter_domain().ok_or(Error::MissingParameter)?;
    let evidence = restriction.pullback(evidence, control)?;
    let normalizer = mass.on_domain(domain, limits.parameters.region, limits.functions, work)?;
    let densities = space.parameter_density_pieces().ok_or(Error::MissingLaw)?;
    let mut pieces = Vec::new();
    for (region, density) in densities {
        for part in normalizer.pieces() {
            budget.step(control)?;
            let case = space.parameter_event(part.defined_on(), limits, work)?;
            let region = region.apply(BoolOp4::AND, &evidence, control)?.apply(
                BoolOp4::AND,
                &case,
                control,
            )?;
            let density = density.div(part, limits.parameters.region, work)?;
            if !region.is_empty() {
                if pieces.len() >= limits.laws.cells {
                    return Err(Error::Capacity(crate::Capacity::LawCells));
                }
                pieces.try_reserve(1)?;
                pieces.push(ParameterDensityPiece { region, density });
            }
        }
    }
    space.with_parameter_density(&pieces, limits, work)
}

fn coordinate_inclusion(
    source: &Space,
    target: &Space,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<CoordinateMap> {
    if source.dimensions() != target.dimensions() {
        return Err(Error::MapArity);
    }
    let mut readouts = Vec::new();
    readouts.try_reserve_exact(usize::from(source.dimensions()))?;
    for coordinate in 0..source.dimensions() {
        readouts.push(source.coordinate(coordinate, work.control())?);
    }
    CoordinateMap::new_with_parameters(source, target, &readouts, limits, work)
}
