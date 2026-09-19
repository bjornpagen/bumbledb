//! Checked changes of guard presentation for the same actual parameter/worlds.
//! Logical-code projection alone is not a parameter-preserving Event image.
use std::collections::HashMap;

use super::{Budget, ParameterDensityPiece, ParameterGuard, ParameterSourceLimits, cube};
use crate::arena::{MAX_COORDINATES, Operation, Ref};
use crate::{
    BoolOp4, Capacity, Control, CoordinateMap, Error, Event, ExactArithmetic, Limits,
    ParameterRegion, Result, Space, SpaceId,
};

/// A checked extension of a sealed guard presentation. Original coordinates
/// keep their meaning; appended coordinates are deterministic predicates of the
/// same actual parameter. No outcomes, prior or independence are introduced.
/// Lifting preserves the designated law. Descent additionally checks that the
/// Event is representable in the original presentation.
#[derive(Debug, Clone)]
pub struct ParameterRefinement {
    source: Space,
    refined: Space,
}

impl ParameterRefinement {
    /// Append one guard per authored predicate and retain the same actual worlds
    /// and law. Repeated predicates remain deterministic aliases.
    /// # Errors
    /// Missing/foreign parameter, source/graph/arithmetic capacities or cancellation.
    pub fn new(
        identity: SpaceId,
        source: &Space,
        predicates: &[ParameterRegion],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::with_limits(
            identity,
            source,
            predicates,
            Limits::default(),
            limits,
            work,
        )
    }

    /// As `new`, with an explicit resource policy for the new graph owner.
    /// # Errors
    /// Has `new`'s checked source contract and the supplied graph limits.
    pub fn with_limits(
        identity: SpaceId,
        source: &Space,
        predicates: &[ParameterRegion],
        events: Limits,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        let context = source.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        let dimensions = usize::from(source.dimensions())
            .checked_add(predicates.len())
            .filter(|&n| n <= usize::from(MAX_COORDINATES))
            .ok_or(Error::Capacity(Capacity::Coordinates))?;
        let mut guards = Vec::new();
        guards.try_reserve_exact(context.guards.len() + predicates.len())?;
        guards.extend(context.guards.iter().cloned());
        for (index, predicate) in predicates.iter().enumerate() {
            budget.step(control)?;
            guards.push(ParameterGuard {
                coordinate: source.dimensions()
                    + u8::try_from(index).map_err(|_| Error::Capacity(Capacity::Coordinates))?,
                region: predicate.clone(),
            });
        }
        let mut order = [0; MAX_COORDINATES as usize];
        for (index, slot) in order.iter_mut().enumerate().take(dimensions) {
            *slot = u8::try_from(index).map_err(|_| Error::Capacity(Capacity::Coordinates))?;
        }
        let raw = Space::with_order(identity, &order[..dimensions], events, control)?;
        let support = transfer(&raw, source, source.0.support, control)?;
        let scoped = raw.restrict(&support, control)?.with_parameters(
            context.domain.clone(),
            &guards,
            limits,
            work,
        )?;
        let refined = if let Some(pieces) = source.parameter_density_pieces() {
            if pieces.len() > limits.laws.cells {
                return Err(Error::Capacity(Capacity::LawCells));
            }
            let mut lifted = Vec::new();
            lifted.try_reserve_exact(pieces.len())?;
            for (region, density) in pieces {
                budget.step(control)?;
                lifted.push(ParameterDensityPiece {
                    region: transfer(&scoped, source, region.root, control)?,
                    density: density.clone(),
                });
            }
            scoped.with_parameter_density(&lifted, limits, work)?
        } else {
            scoped
        };
        control.checkpoint()?;
        Ok(Self {
            source: source.clone(),
            refined,
        })
    }

    /// Give each named source a presentation resolving the union of the guard
    /// predicates across the roster. Domains must be equal and retain the same
    /// parameter name. Existing representable predicates need no new coordinate.
    /// Names explicitly identify the resulting presentations, in input order.
    /// # Errors
    /// No sources, mismatched domains, coordinate/solver/graph limits or cancellation.
    pub fn common(
        sources: &[(SpaceId, Space)],
        events: Limits,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<Self>> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        budget.extent(sources.len(), control)?;
        let (_, first) = sources.first().ok_or(Error::NoParameterSources)?;
        let context = first.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        let mut predicates = Vec::new();
        for (_, source) in sources {
            budget.step(control)?;
            let other = source.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
            if other.domain.to_bytes(limits.parameters, work)?.as_slice()
                != context.domain_bytes.as_ref()
            {
                return Err(Error::ParameterDomainMismatch);
            }
            for guard in other.guards.iter() {
                budget.extent(predicates.len() + 1, control)?;
                let region = guard.region.apply(
                    BoolOp4::AND,
                    context.domain.region(),
                    limits.parameters.region,
                    work,
                )?;
                let bytes = region.to_bytes(limits.parameters, work)?;
                predicates.try_reserve(1)?;
                predicates.push((bytes, region));
            }
        }
        predicates.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        predicates.dedup_by(|a, b| a.0 == b.0);
        let mut result = Vec::new();
        result.try_reserve_exact(sources.len())?;
        for (identity, source) in sources {
            let mut additional = Vec::new();
            for (_, predicate) in &predicates {
                budget.step(control)?;
                match source.parameter_event(predicate, limits, work) {
                    Ok(_) => {}
                    Err(Error::ParameterRefinementRequired) => {
                        additional.try_reserve(1)?;
                        additional.push(predicate.clone());
                    }
                    Err(error) => return Err(error),
                }
            }
            result.push(Self::with_limits(
                *identity,
                source,
                &additional,
                events,
                limits,
                work,
            )?);
        }
        Ok(result)
    }

    #[must_use]
    pub fn source(&self) -> &Space {
        &self.source
    }

    #[must_use]
    pub fn refined(&self) -> &Space {
        &self.refined
    }

    /// Exact lift to the new presentation, preserving Boolean algebra, actual
    /// worlds and any designated law. Every input owner is checked.
    /// # Errors
    /// Foreign context, graph limits or cancellation.
    pub fn lift(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let event = event.align_to(&self.source, control)?;
        transfer(&self.refined, &self.source, event.root, control)
    }

    /// Return the unique Event in the old presentation with the same actual
    /// worlds, or refuse when the new guards are essential. An existential
    /// envelope is only a candidate; lifting it must recover the full input.
    /// # Errors
    /// Unrepresentable Event, foreign context, graph limits or cancellation.
    pub fn descend(&self, event: &Event, control: &dyn Control) -> Result<Event> {
        let event = event.align_to(&self.refined, control)?;
        let root = {
            let mut arena = self.refined.0.lock()?;
            let hidden = arena.mask() & !((1u64 << self.source.dimensions()) - 1);
            let mut op = Operation::new(&mut arena, control)?;
            let legal = op.apply(BoolOp4::AND, self.refined.0.support, event.root)?;
            op.exists(legal, hidden)?
        };
        let candidate = transfer(&self.source, &self.refined, root, control)?;
        if !self.lift(&candidate, control)?.equivalent(&event)? {
            return Err(Error::ParameterGuardEssential);
        }
        control.checkpoint()?;
        Ok(candidate)
    }
}

impl Space {
    /// Represent an exact parameter predicate using this presentation's existing
    /// guards. Refuses a predicate cutting any cell, rather than rounding it to
    /// a union of cells. Refine the source explicitly to admit new distinctions.
    /// # Errors
    /// Missing/foreign parameter, refinement required, capacities or cancellation.
    pub fn parameter_event(
        &self,
        predicate: &ParameterRegion,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Event> {
        let control = work.control();
        let mut budget = Budget::new(limits, control)?;
        let context = self.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        budget.extent(context.fibres.len(), control)?;
        let predicate = predicate.apply(
            BoolOp4::AND,
            context.domain.region(),
            limits.parameters.region,
            work,
        )?;
        let mut result = self.empty();
        for fibre in context.fibres.iter() {
            budget.step(control)?;
            let part =
                predicate.apply(BoolOp4::AND, &fibre.region, limits.parameters.region, work)?;
            if !part.is_empty() {
                if !fibre
                    .region
                    .included(&predicate, limits.parameters.region, work)?
                {
                    return Err(Error::ParameterRefinementRequired);
                }
                result = result.apply(
                    BoolOp4::OR,
                    &cube(self, context.mask, fibre.code, control)?,
                    control,
                )?;
            }
        }
        Ok(result)
    }
}

impl CoordinateMap {
    /// Lift an existing actual-parameter map to two refined presentations.
    /// Old readouts are lifted exactly; appended target guards are represented
    /// by their real predicates on the refined source. The ordinary constructor
    /// rechecks support and equal exact guard partitions before publication.
    /// # Errors
    /// Foreign endpoints, further refinement required, capacities or cancellation.
    pub fn refine_parameters(
        &self,
        source: &ParameterRefinement,
        target: &ParameterRefinement,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        self.source().full().align_to(source.source(), control)?;
        self.target().full().align_to(target.source(), control)?;
        let mut budget = Budget::new(limits, control)?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(usize::from(target.refined().dimensions()))?;
        for readout in self.readouts() {
            budget.step(control)?;
            readouts.push(source.lift(readout, control)?);
        }
        for guard in target
            .refined()
            .parameter_guards()
            .ok_or(Error::MissingParameter)?
        {
            if guard.coordinate >= target.source().dimensions() {
                budget.step(control)?;
                readouts.push(
                    source
                        .refined()
                        .parameter_event(&guard.region, limits, work)?,
                );
            }
        }
        Self::new(source.refined(), target.refined(), &readouts, control)
    }
}

// Builder-only transfer at identical old coordinate positions. Callers either
// append deterministic guards or have eliminated them and checked exact descent.
// Mask/completion uses the target's support, never a source decoder alias.
fn transfer(target: &Space, source: &Space, root: Ref, control: &dyn Control) -> Result<Event> {
    let root = target.with_arena_pair(source, |arena, other| {
        let mut op = Operation::new(arena, control)?;
        let root = match other {
            Some(other) => op.transfer(other, root, &mut HashMap::new())?,
            None => root,
        };
        target.0.complete(&mut op, root)
    })?;
    control.checkpoint()?;
    Ok(target.event(root))
}
