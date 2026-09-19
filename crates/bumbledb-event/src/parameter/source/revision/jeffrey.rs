//! Replacement partition probabilities, with exact unsupported parameter regions.
use std::sync::Arc;

use super::super::measure::constant;
use super::{Budget, ParameterRestriction, ParameterRevisedSource, coordinate_inclusion};
use crate::{
    BoolOp4, Error, EventPartition, ExactArithmetic, ExactRational, FamilyFunction,
    FamilyFunctionPiece, ParameterFunction, ParameterRegion, ParameterSourceLimits,
    PartitionLimits, PolynomialSigns, Result, Space, SpaceId,
};

/// Owned replacement targets and their per-cell validity failures. There is no
/// intrinsic evidence probability. Empty partition positions remain observable.
#[derive(Debug, Clone)]
pub struct ParameterJeffrey {
    identity: SpaceId,
    prior: Space,
    partition: EventPartition,
    targets: Arc<[ParameterFunction]>,
    old_masses: Arc<[ParameterFunction]>,
    unsupported: Arc<[ParameterRegion]>,
    defined: ParameterRegion,
    revised: Option<ParameterRevisedSource>,
}

impl ParameterJeffrey {
    /// Requested presentation identity, including an impossible request.
    #[must_use]
    pub fn identity(&self) -> SpaceId {
        self.identity
    }
    #[must_use]
    pub fn prior(&self) -> &Space {
        &self.prior
    }
    #[must_use]
    pub fn partition(&self) -> &EventPartition {
        &self.partition
    }
    #[must_use]
    pub fn targets(&self) -> &[ParameterFunction] {
        &self.targets
    }
    #[must_use]
    pub fn old_masses(&self) -> &[ParameterFunction] {
        &self.old_masses
    }
    /// One region per ordered cell: target > 0 and old mass = 0.
    #[must_use]
    pub fn unsupported_regions(&self) -> &[ParameterRegion] {
        &self.unsupported
    }
    #[must_use]
    pub fn defined_on(&self) -> &ParameterRegion {
        &self.defined
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
    /// Replace partition masses by total nonnegative parameter functions summing
    /// to one. Keep prior within-cell conditionals wherever each positive target
    /// has positive old mass. A zero target skips its undefined conditional.
    /// Invalid parameter values remain explicit in the receipt; all outcomes at
    /// valid values survive, including outcomes receiving posterior zero.
    /// # Errors
    /// Partial/foreign partitions, target arity, partial/negative/non-unit targets,
    /// missing laws, capacities or cancellation. All-invalid requests return an
    /// owned impossible receipt rather than constructing an empty source.
    pub fn parameter_jeffrey(
        &self,
        identity: SpaceId,
        partition: &EventPartition,
        targets: &[ParameterFunction],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterJeffrey> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        partition.parent().align_to(self, control)?;
        if !partition.parent().is_full() {
            return Err(Error::PartitionGap);
        }
        if partition.cells().len() != targets.len() {
            return Err(Error::PartitionArity);
        }
        let partition = EventPartition::on(
            &self.full(),
            partition.cells(),
            PartitionLimits {
                cells: limits.functions.cells,
            },
            control,
        )?;
        let domain = self.parameter_domain().ok_or(Error::MissingParameter)?;
        let parameters = limits.parameters.region;
        let functions = limits.functions;
        validate_targets(domain, targets, limits, &mut budget, work)?;
        let mut old_masses = Vec::new();
        let mut unsupported = Vec::new();
        let mut factors = Vec::new();
        for roster in [&mut old_masses, &mut factors] {
            roster.try_reserve_exact(targets.len())?;
        }
        unsupported.try_reserve_exact(targets.len())?;
        let mut defined = domain.region().clone();
        let mut predicates = Vec::new();
        for (cell, target) in partition.cells().iter().zip(targets) {
            budget.step(control)?;
            let old = cell.parameter_mass(limits, work)?;
            let positive =
                target.where_sign(PolynomialSigns::POSITIVE, parameters, functions, work)?;
            let invalid = positive.apply(
                BoolOp4::AND,
                &old.where_sign(PolynomialSigns::ZERO, parameters, functions, work)?,
                parameters,
                work,
            )?;
            defined = defined.apply(BoolOp4::DIFFERENCE, &invalid, parameters, work)?;
            // Only positive-target terms exist. In particular 0/0 is never a
            // value, nor is a cancelled denominator allowed to fill a hole.
            let factor = target
                .restrict(&positive, parameters, functions, work)?
                .div(&old, parameters, functions, work)?;
            for piece in factor.pieces() {
                budget.extent(predicates.len() + 1, control)?;
                predicates.try_reserve(1)?;
                predicates.push(piece.defined_on().clone());
            }
            old_masses.push(old);
            unsupported.push(invalid);
            factors.push(factor);
        }
        let revised = if defined.is_empty() {
            None
        } else {
            let restriction = ParameterRestriction::with_predicates(
                identity,
                self,
                &defined,
                &predicates,
                limits,
                work,
            )?;
            let posterior = replacement_law(
                &restriction,
                &partition,
                &factors,
                limits,
                &mut budget,
                work,
            )?;
            let translation =
                coordinate_inclusion(&posterior, restriction.refined_prior(), limits, work)?;
            Some(ParameterRevisedSource {
                restriction,
                translation,
            })
        };
        let mut retained_targets = Vec::new();
        retained_targets.try_reserve_exact(targets.len())?;
        retained_targets.extend_from_slice(targets);
        control.checkpoint()?;
        Ok(ParameterJeffrey {
            identity,
            prior: self.clone(),
            partition,
            targets: retained_targets.into(),
            old_masses: old_masses.into(),
            unsupported: unsupported.into(),
            defined,
            revised,
        })
    }
}

fn validate_targets(
    domain: &crate::ParameterDomain,
    targets: &[ParameterFunction],
    limits: ParameterSourceLimits,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<()> {
    let control = work.control();
    let parameters = limits.parameters.region;
    let functions = limits.functions;
    let one = ParameterFunction::new(
        domain.clone(),
        &[constant(domain, ExactRational::one(), limits, work)?],
        parameters,
        functions,
        work,
    )?;
    let mut total = ParameterFunction::new(
        domain.clone(),
        &[constant(domain, ExactRational::zero(), limits, work)?],
        parameters,
        functions,
        work,
    )?;
    for target in targets {
        budget.step(control)?;
        if !domain
            .region()
            .equivalent(target.ambient().region(), parameters, work)?
        {
            return Err(Error::ParameterDomainMismatch);
        }
        if !domain
            .region()
            .included(target.defined_on(), parameters, work)?
        {
            return Err(Error::UndefinedFunction);
        }
        if !target
            .where_sign(PolynomialSigns::NEGATIVE, parameters, functions, work)?
            .is_empty()
        {
            return Err(Error::NegativeMass);
        }
        total = total.add(target, parameters, functions, work)?;
    }
    if !total.equivalent(&one, parameters, functions, work)? {
        return Err(Error::LawNotNormalized);
    }
    Ok(())
}

fn replacement_law(
    restriction: &ParameterRestriction,
    partition: &EventPartition,
    factors: &[ParameterFunction],
    limits: ParameterSourceLimits,
    budget: &mut Budget,
    work: &mut ExactArithmetic<'_>,
) -> Result<Space> {
    let control = work.control();
    let parameters = limits.parameters.region;
    let functions = limits.functions;
    let space = restriction.space();
    let narrowed = space.parameter_domain().ok_or(Error::MissingParameter)?;
    let mut pieces = Vec::new();
    for (cell, factor) in partition.cells().iter().zip(factors) {
        let cell = restriction.pullback(cell, control)?;
        let factor = factor.on_domain(narrowed, parameters, functions, work)?;
        for piece in factor.pieces() {
            budget.step(control)?;
            crate::function::raw::Budget::new(functions, control)?.cells(pieces.len() + 1)?;
            pieces.try_reserve(1)?;
            pieces.push(FamilyFunctionPiece {
                region: cell.apply(
                    BoolOp4::AND,
                    &space.parameter_event(piece.defined_on(), limits, work)?,
                    control,
                )?,
                value: piece.clone(),
            });
        }
    }
    let factor = FamilyFunction::new(space, &pieces, limits, work)?;
    let posterior = FamilyFunction::density(space, limits, work)?
        .multiply(&factor, limits, work)?
        .designate(limits, work)?;
    Ok(posterior)
}
