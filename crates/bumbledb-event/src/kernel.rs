//! Conditional joint laws as exact scalar functions on an explicitly captured
//! extension map. Row normalization is checked on every structurally possible
//! parent world, including those with zero mass in a particular prior.
use crate::{
    Control, CoordinateMap, Error, ExactArithmetic, ExactRational, FiniteFunction, FunctionLimits,
    LawLimits, Result, Space, SurjectiveMap,
};

/// A normalized nonnegative conditional law on fibres of `parent`. Distinct
/// extended worlds are the outcomes; arbitrary deterministic readouts may name
/// the parent. No independence or information-access policy is inferred.
#[derive(Debug, Clone)]
pub struct FiniteKernel {
    parent: SurjectiveMap,
    density: FiniteFunction,
}

/// A new measured joint source plus a checked map back to its original prior.
/// Pullback translates old Events without copying or resampling their outcomes.
#[derive(Debug, Clone)]
pub struct SourceExtension {
    space: Space,
    parent: SurjectiveMap,
}
impl SourceExtension {
    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }
    #[must_use]
    pub fn parent(&self) -> &CoordinateMap {
        self.parent.map()
    }
    /// The complete old structural space remains represented, including its
    /// zero-mass worlds. This is separate from the marginal-law guarantee.
    #[must_use]
    pub fn parent_surjective(&self) -> &SurjectiveMap {
        &self.parent
    }
}

impl FiniteKernel {
    /// Check nonnegativity and unit fibre sum everywhere. This is stronger than
    /// checking only that closure under one prior happens to normalize.
    /// # Errors
    /// Context mismatch, negative values, non-unit fibres, capacity/cancellation.
    pub fn new(
        parent: &CoordinateMap,
        density: &FiniteFunction,
        limits: FunctionLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        density
            .space()
            .full()
            .align_to(parent.source(), work.control())?;
        if !density.is_nonnegative() {
            return Err(Error::NegativeMass);
        }
        let rows = density.pushforward(parent, limits, work)?;
        let one = FiniteFunction::constant(parent.target(), ExactRational::one(), limits, work)?;
        if !rows.equivalent(&one, work.control())? {
            return Err(Error::KernelNotNormalized);
        }
        let parent = parent.certify_surjective(work.control())?;
        Ok(Self {
            parent,
            density: density.clone(),
        })
    }
    #[must_use]
    pub fn parent(&self) -> &SurjectiveMap {
        &self.parent
    }
    #[must_use]
    pub fn density(&self) -> &FiniteFunction {
        &self.density
    }

    /// Close the conditional law under an explicitly supplied prior. The prior
    /// must have exactly the parent's named structural space; its designated law
    /// may differ from the map's original target law. This explicit rebinding
    /// licenses reuse of a fixed channel, never reinterpretation of evidence.
    /// # Errors
    /// Missing law, structural/context mismatch, capacities or cancellation.
    pub fn close(
        &self,
        prior: &Space,
        functions: FunctionLimits,
        laws: LawLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<SourceExtension> {
        let control = work.control();
        prior
            .unmeasured(control)?
            .full()
            .align_to(&self.parent.map().target().unmeasured(control)?, control)?;
        let rebound = CoordinateMap::new(
            self.parent.map().source(),
            prior,
            self.parent.map().readouts(),
            control,
        )?;
        let density =
            FiniteFunction::density(prior, functions, work)?.pullback(&rebound, functions, work)?;
        let joint = self.density.multiply(&density, functions, work)?;
        let space = joint.designate(laws, work)?;
        let mut readouts = Vec::new();
        readouts.try_reserve_exact(rebound.readouts().len())?;
        for readout in rebound.readouts() {
            readouts.push(readout.in_space(&space, control)?);
        }
        let parent =
            CoordinateMap::new(&space, prior, &readouts, control)?.certify_surjective(control)?;
        Ok(SourceExtension { space, parent })
    }

    /// Whether this whole conditional density depends only on the supplied
    /// extension readout (for example actor-visible state plus the new outcome).
    /// This checks a functional dependency; choosing an adequate readout is an
    /// explicit modeling responsibility, not something model confidence proves.
    /// # Errors
    /// Context mismatch, graph limits or cancellation.
    pub fn factors_through(
        &self,
        observation: &SurjectiveMap,
        control: &dyn Control,
    ) -> Result<bool> {
        observation
            .map()
            .source()
            .full()
            .align_to(self.density.space(), control)?;
        for (region, _) in self.density.pieces() {
            match observation.descend(&region, control) {
                Ok(_) => {}
                Err(Error::RoleMismatch) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }
}
