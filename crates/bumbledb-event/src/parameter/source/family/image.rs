use super::{Budget, FamilyFunction, FamilyFunctionPiece, ParameterSourceLimits, constant};
use crate::{
    CoordinateMap, Error, ExactArithmetic, ExactRational, FiniteFunction, FunctionPiece,
    GuardedRationalFunction, Result, SurjectiveMap,
};

impl FamilyFunction {
    /// Descend exactly when the function is constant on every readout fibre.
    /// The fibre mean is only a candidate: its pullback must equal the original
    /// at every actual world. This introduces no probability law or prior.
    /// Equivalent coefficient formulas and different cell decompositions are
    /// accepted. Surjectivity ensures a positive finite divisor everywhere.
    /// # Errors
    /// `RoleMismatch` for a nonconstant fibre; foreign context, capacities or cancellation.
    pub fn descend(
        &self,
        map: &SurjectiveMap,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let map = map.map();
        let checked = self.align_to(map.source(), limits, work)?;
        let one =
            FiniteFunction::constant(map.source(), ExactRational::one(), limits.functions, work)?;
        let counts =
            Self::from_finite(&one.pushforward(map, limits.functions, work)?, limits, work)?;
        let candidate = checked
            .pushforward(map, limits, work)?
            .divide(&counts, limits, work)?;
        if !candidate
            .pullback(map, limits, work)?
            .equivalent(&checked, limits, work)?
        {
            return Err(Error::RoleMismatch);
        }
        Ok(candidate)
    }

    /// Substitute a checked same-parameter map, including a smaller source domain.
    /// # Errors
    /// Foreign context, parameter-domain mismatch, capacities or cancellation.
    pub fn pullback(
        &self,
        map: &CoordinateMap,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let checked = self.align_to(map.target(), limits, work)?;
        let domain = map
            .source()
            .parameter_domain()
            .ok_or(Error::MissingParameter)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(checked.terms.len())?;
        for (region, value) in checked.pieces() {
            pieces.push(FamilyFunctionPiece {
                region: map.pullback(&region, work.control())?,
                value: value.on_domain(domain, limits.parameters.region, work)?,
            });
        }
        Self::new(map.source(), &pieces, limits, work)
    }

    /// Sum values over all finite outcome witnesses at the same actual parameter.
    /// Guard cases are never random alternatives. Unreachable target worlds get
    /// zero. This does not designate or normalize the target's law.
    /// # Errors
    /// Foreign source, function/graph/solver capacities or cancellation.
    pub fn pushforward(
        &self,
        map: &CoordinateMap,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let checked = self.align_to(map.source(), limits, work)?;
        let domain = map
            .target()
            .parameter_domain()
            .ok_or(Error::MissingParameter)?;
        let mut budget = Budget::new(limits.functions, work.control())?;
        let mut result = Self::new(map.target(), &[], limits, work)?;
        for (region, value) in checked.pieces() {
            budget.step(work.control())?;
            let multiplicity = FiniteFunction::new(
                map.source(),
                &[FunctionPiece {
                    region,
                    value: ExactRational::one(),
                }],
                limits.functions,
                work,
            )?
            .pushforward(map, limits.functions, work)?;
            // Re-express the arithmetic over the target ambient domain, retaining
            // the old defined set. Image regions cannot reach beyond it.
            let value = GuardedRationalFunction::new(
                domain.clone(),
                value.numerator().clone(),
                value.denominator().clone(),
                limits.parameters.region,
                work,
            )?
            .restrict(value.defined_on(), limits.parameters.region, work)?;
            let mut pieces = Vec::new();
            budget.cells(multiplicity.pieces().len())?;
            pieces.try_reserve_exact(multiplicity.pieces().len())?;
            for (region, count) in multiplicity.pieces() {
                budget.step(work.control())?;
                pieces.push(FamilyFunctionPiece {
                    region,
                    value: value.mul(
                        &constant(domain, count.clone(), limits, work)?,
                        limits.parameters.region,
                        work,
                    )?,
                });
            }
            result = result.add(
                &Self::new(map.target(), &pieces, limits, work)?,
                limits,
                work,
            )?;
        }
        Ok(result)
    }
}
