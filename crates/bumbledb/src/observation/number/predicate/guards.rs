//! Explicitly interpret numerical truth on a named source. New distinctions
//! require a checked refinement; an unresolved region is never rounded.
use super::ObservationPredicate;
use crate::Result;
use crate::event::{
    Error, Event, ExactArithmetic, Limits, NumberLimits, NumberPredicateView, ParameterRefinement,
    ParameterRegion, ParameterSourceLimits, Space, SpaceId,
};

/// A complete disjoint truth partition in one checked Event presentation,
/// retaining both the numerical derivation and the source it was interpreted on.
#[derive(Debug, Clone)]
pub struct PredicateEvents {
    predicate: ObservationPredicate,
    source: Space,
    holds: Event,
    fails: Event,
    undefined: Event,
}
impl PredicateEvents {
    #[must_use]
    pub fn predicate(&self) -> &ObservationPredicate {
        &self.predicate
    }
    #[must_use]
    pub fn source(&self) -> &Space {
        &self.source
    }
    #[must_use]
    pub fn holds(&self) -> &Event {
        &self.holds
    }
    #[must_use]
    pub fn fails(&self) -> &Event {
        &self.fails
    }
    #[must_use]
    pub fn undefined(&self) -> &Event {
        &self.undefined
    }
}

/// The explicit guard extension and resulting truth partition. Its ordinary
/// `ParameterRefinement` lifts other Events from the original source without
/// conditioning, renormalizing, binding a prior or dropping zero-mass worlds.
#[derive(Debug, Clone)]
pub struct PredicateRefinement {
    refinement: ParameterRefinement,
    events: PredicateEvents,
}
impl PredicateRefinement {
    #[must_use]
    pub fn refinement(&self) -> &ParameterRefinement {
        &self.refinement
    }
    #[must_use]
    pub fn events(&self) -> &PredicateEvents {
        &self.events
    }
}

impl ObservationPredicate {
    fn check_source(
        &self,
        source: &Space,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        self.predicate().validate(
            NumberLimits {
                parameters: limits.parameters.region,
                functions: limits.functions,
            },
            work,
        )?;
        // Even a fixed/constant truth result validates the chosen source owner.
        source.full().align_to(source, work.control())?;
        if let NumberPredicateView::Parameter { ambient, .. } = self.predicate().view() {
            let domain = source.parameter_domain().ok_or(Error::MissingParameter)?;
            if !ambient
                .region()
                .equivalent(domain.region(), limits.parameters.region, work)?
            {
                return Err(Error::ParameterDomainMismatch.into());
            }
        }
        Ok(())
    }

    /// Interpret all three regions using the source's existing guards. A
    /// parameter predicate must cover exactly the source's ambient domain.
    /// # Errors
    /// Domain mismatch, missing/unresolved guards, solver/graph limits or cancellation.
    pub fn events(
        &self,
        source: &Space,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<PredicateEvents> {
        self.check_source(source, limits, work)?;
        let [holds, fails, undefined] = match self.predicate().view() {
            NumberPredicateView::Fixed(value) => [Some(true), Some(false), None].map(|case| {
                if value == case {
                    source.full()
                } else {
                    source.empty()
                }
            }),
            NumberPredicateView::Parameter {
                holds,
                fails,
                undefined,
                ..
            } => [
                source.parameter_event(holds, limits, work)?,
                source.parameter_event(fails, limits, work)?,
                source.parameter_event(undefined, limits, work)?,
            ],
        };
        work.control().checkpoint()?;
        Ok(PredicateEvents {
            predicate: self.clone(),
            source: source.clone(),
            holds,
            fails,
            undefined,
        })
    }

    /// Append the truth distinctions missing from a parameter presentation.
    /// True and false suffice: undefined is their ambient complement. A total
    /// predicate needs only true; false is then its complement. Existing
    /// representable cases add no coordinate. The supplied identity explicitly
    /// names the new presentation; the returned map transports old Events.
    /// # Errors
    /// Missing/foreign/different parameter domain, capacities or cancellation.
    pub fn refine(
        &self,
        identity: SpaceId,
        source: &Space,
        events: Limits,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<PredicateRefinement> {
        self.check_source(source, limits, work)?;
        let mut predicates = Vec::<ParameterRegion>::new();
        if let NumberPredicateView::Parameter { holds, fails, .. } = self.predicate().view() {
            let cases = [holds, fails];
            let count = if self.predicate().is_total() { 1 } else { 2 };
            predicates.try_reserve_exact(count).map_err(Error::from)?;
            for &region in &cases[..count] {
                match source.parameter_event(region, limits, work) {
                    Ok(_) => {}
                    Err(Error::ParameterRefinementRequired) => predicates.push(region.clone()),
                    Err(error) => return Err(error.into()),
                }
            }
        }
        let refinement =
            ParameterRefinement::with_limits(identity, source, &predicates, events, limits, work)?;
        let events = self.events(refinement.refined(), limits, work)?;
        Ok(PredicateRefinement { refinement, events })
    }
}
