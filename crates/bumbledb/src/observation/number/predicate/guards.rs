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

    /// Resolve a nonempty roster in one named presentation, in input order.
    /// Canonical guard ordering makes the presentation invariant under roster
    /// permutation and duplication. All written predicates are checked before
    /// duplicate regions are removed; the returned cases retain each origin.
    /// # Errors
    /// Empty roster, domain/source mismatch, source/byte/work limits or cancellation.
    pub fn common(
        identity: SpaceId,
        source: &Space,
        predicates: &[ObservationPredicate],
        events: Limits,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<Self>> {
        Self::common_sources(identity, source, &[], predicates, events, limits, work)
    }

    /// Resolve the predicate roster and every peer source's parameter guards
    /// in one presentation of `source`. Peers supply distinctions, not outcomes
    /// or probability mass. Every peer must have the same named exact domain.
    /// # Errors
    /// Empty predicate roster, invalid domains, source/byte/work bounds or cancellation.
    pub fn common_sources(
        identity: SpaceId,
        source: &Space,
        sources: &[Space],
        predicates: &[ObservationPredicate],
        events: Limits,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<Self>> {
        work.control().checkpoint()?;
        if predicates.is_empty() {
            return Err(Error::NoParameterSources.into());
        }
        if predicates.len() > limits.steps / 3 {
            return Err(Error::Capacity(crate::event::Capacity::ParameterSourceSteps).into());
        }
        source.parameter_domain().ok_or(Error::MissingParameter)?;
        let mut guards = CommonGuards::from_sources(source, sources, limits, work)?;
        for predicate in predicates {
            predicate.check_source(source, limits, work)?;
            if let NumberPredicateView::Parameter { holds, fails, .. } =
                predicate.predicate().view()
            {
                let regions = [holds, fails];
                let count = if predicate.predicate().is_total() {
                    1
                } else {
                    2
                };
                for region in &regions[..count] {
                    guards.push((*region).clone(), limits, work)?;
                }
            }
        }
        let mut additional = Vec::new();
        for region in guards.canonical() {
            match source.parameter_event(&region, limits, work) {
                Ok(_) => {}
                Err(Error::ParameterRefinementRequired) => {
                    additional.try_reserve(1).map_err(Error::from)?;
                    additional.push(region);
                }
                Err(error) => return Err(error.into()),
            }
        }
        let refinement =
            ParameterRefinement::with_limits(identity, source, &additional, events, limits, work)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(predicates.len())
            .map_err(Error::from)?;
        for predicate in predicates {
            result.push(Self {
                events: predicate.events(refinement.refined(), limits, work)?,
                refinement: refinement.clone(),
            });
        }
        Ok(result)
    }

    pub(crate) fn check_sources(
        source: &Space,
        sources: &[Space],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        for region in CommonGuards::from_sources(source, sources, limits, work)?.canonical() {
            source.parameter_event(&region, limits, work)?;
        }
        Ok(())
    }
}

struct CommonGuards {
    regions: Vec<(Vec<u8>, ParameterRegion)>,
    bytes_left: usize,
}
impl CommonGuards {
    fn push(
        &mut self,
        region: ParameterRegion,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        if self.regions.len() >= limits.steps / 3 {
            return Err(Error::Capacity(crate::event::Capacity::ParameterSourceSteps).into());
        }
        let bytes = region.to_bytes(limits.parameters, work)?;
        self.bytes_left = self
            .bytes_left
            .checked_sub(bytes.len())
            .ok_or(Error::Capacity(crate::event::Capacity::DescriptorBytes))?;
        self.regions.try_reserve(1).map_err(Error::from)?;
        self.regions.push((bytes, region));
        Ok(())
    }

    fn from_sources(
        source: &Space,
        sources: &[Space],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut result = Self {
            regions: Vec::new(),
            bytes_left: limits.parameters.bytes,
        };
        if sources.is_empty() {
            return Ok(result);
        }
        if sources.len() > limits.steps / 3 {
            return Err(Error::Capacity(crate::event::Capacity::ParameterSourceSteps).into());
        }
        let domain = source.parameter_domain().ok_or(Error::MissingParameter)?;
        let domain_bytes = domain.to_bytes(limits.parameters, work)?;
        for peer in sources {
            work.control().checkpoint()?;
            let other = peer.parameter_domain().ok_or(Error::MissingParameter)?;
            if other.to_bytes(limits.parameters, work)? != domain_bytes {
                return Err(Error::ParameterDomainMismatch.into());
            }
            for guard in peer.parameter_guards().ok_or(Error::MissingParameter)? {
                result.push(
                    guard.region.apply(
                        crate::event::BoolOp4::AND,
                        domain.region(),
                        limits.parameters.region,
                        work,
                    )?,
                    limits,
                    work,
                )?;
            }
        }
        Ok(result)
    }

    fn canonical(mut self) -> impl Iterator<Item = ParameterRegion> {
        self.regions.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        self.regions.dedup_by(|a, b| a.0 == b.0);
        self.regions.into_iter().map(|(_, region)| region)
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
