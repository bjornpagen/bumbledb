//! Total signed functions of the same actual parameter and finite outcomes.
//! Values are exact guarded rational functions; Event regions select pieces.
use std::sync::Arc;

use super::{
    Budget as SourceBudget, ParameterDensityPiece, ParameterRefinement, ParameterSourceLimits,
    count_root, measure::constant,
};
use crate::arena::Ref;
use crate::function::raw::Budget;
use crate::{
    BoolOp4, Error, Event, ExactArithmetic, ExactRational, FiniteFunction, GuardedRationalFunction,
    ParameterFunction, ParameterRegion, PolynomialSigns, Result, Space,
};

mod arithmetic;
mod image;
mod measure;

/// A disjoint Event cell and its signed parameter-dependent value. The value
/// must be defined at every actual parameter reached by the cell. Unspecified
/// worlds receive an explicit zero default, as with `FiniteFunction`.
#[derive(Debug, Clone)]
pub struct FamilyFunctionPiece {
    pub region: Event,
    pub value: GuardedRationalFunction,
}

#[derive(Debug, Clone)]
struct Term {
    root: Ref,
    value: GuardedRationalFunction,
}

/// A total exact signed function on an owned parameterized world space.
/// Parameter-only partial functions are `ParameterFunction`; this object has
/// a value at every legal `(parameter, outcome)` and retains its whole context.
#[derive(Debug, Clone)]
pub struct FamilyFunction {
    space: Space,
    terms: Arc<[Term]>,
}

impl FamilyFunction {
    /// Admit disjoint cells, with zero on omitted worlds. Every written value
    /// is checked even on empty cells. Holes outside a cell's active parameter
    /// fibres are allowed; a hole at any world of that cell refuses.
    /// # Errors
    /// Missing/foreign parameter or context, overlaps, undefined values,
    /// capacities or cancellation.
    pub fn new(
        space: &Space,
        pieces: &[FamilyFunctionPiece],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let context = space.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        let mut budget = Budget::new(limits.functions, control)?;
        let mut source_budget = SourceBudget::new(limits, control)?;
        budget.cells(pieces.len())?;
        source_budget.extent(context.fibres.len(), control)?;
        context.domain.region().equivalent(
            context.domain.region(),
            limits.parameters.region,
            work,
        )?;
        let mut covered = space.empty();
        let mut terms = Vec::new();
        terms.try_reserve_exact(pieces.len())?;
        for piece in pieces {
            budget.step(control)?;
            let region = piece.region.align_to(space, control)?;
            if !context.domain.region().equivalent(
                piece.value.ambient().region(),
                limits.parameters.region,
                work,
            )? {
                return Err(Error::ParameterDomainMismatch);
            }
            let value =
                piece
                    .value
                    .restrict(context.domain.region(), limits.parameters.region, work)?;
            if !covered.apply(BoolOp4::AND, &region, control)?.is_empty() {
                return Err(Error::FunctionOverlap);
            }
            for fibre in context.fibres.iter() {
                source_budget.step(control)?;
                if count_root(space, region.root, context.mask, fibre.code, control)? != 0
                    && !fibre
                        .region
                        .included(value.defined_on(), limits.parameters.region, work)?
                {
                    return Err(Error::UndefinedFunction);
                }
            }
            covered = covered.apply(BoolOp4::OR, &region, control)?;
            if !region.is_empty() {
                terms.push(Term {
                    root: region.root,
                    value,
                });
            }
        }
        control.checkpoint()?;
        Ok(Self {
            space: space.clone(),
            terms: terms.into(),
        })
    }

    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }

    pub fn pieces(&self) -> impl ExactSizeIterator<Item = (Event, &GuardedRationalFunction)> {
        self.terms
            .iter()
            .map(|term| (self.space.event(term.root), &term.value))
    }

    /// # Errors
    /// Same admission as `new`; the value must be defined on the whole domain.
    pub fn constant(
        space: &Space,
        value: GuardedRationalFunction,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::new(
            space,
            &[FamilyFunctionPiece {
                region: space.full(),
                value,
            }],
            limits,
            work,
        )
    }

    /// Embed an existing total rational payoff without changing its source.
    /// # Errors
    /// Missing parameter, capacities or cancellation, including a zero payoff.
    pub fn from_finite(
        value: &FiniteFunction,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let domain = value
            .space()
            .parameter_domain()
            .ok_or(Error::MissingParameter)?;
        let mut budget = Budget::new(limits.functions, work.control())?;
        budget.cells(value.pieces().len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(value.pieces().len())?;
        for (region, scalar) in value.pieces() {
            budget.step(work.control())?;
            pieces.push(FamilyFunctionPiece {
                region,
                value: constant(domain, scalar.clone(), limits, work)?,
            });
        }
        Self::new(value.space(), &pieces, limits, work)
    }

    /// Embed a parameter-only function only when total on the source domain.
    /// Its piece boundaries must be expressible in the sealed presentation.
    /// # Errors
    /// Foreign domain, undefined points, refinement required, capacities or cancellation.
    pub fn from_parameter(
        space: &Space,
        value: &ParameterFunction,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let domain = space.parameter_domain().ok_or(Error::MissingParameter)?;
        if !domain
            .region()
            .equivalent(value.ambient().region(), limits.parameters.region, work)?
        {
            return Err(Error::ParameterDomainMismatch);
        }
        value.where_sign(
            PolynomialSigns::ANY,
            limits.parameters.region,
            limits.functions,
            work,
        )?;
        if !domain
            .region()
            .included(value.defined_on(), limits.parameters.region, work)?
        {
            return Err(Error::UndefinedFunction);
        }
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(value.pieces().len())?;
        for part in value.pieces() {
            pieces.push(FamilyFunctionPiece {
                region: space.parameter_event(part.defined_on(), limits, work)?,
                value: part.clone(),
            });
        }
        Self::new(space, &pieces, limits, work)
    }

    /// Capture the designated family density as an ordinary function.
    /// # Errors
    /// Missing parameter/law, capacities or cancellation.
    pub fn density(
        space: &Space,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        space.parameter_domain().ok_or(Error::MissingParameter)?;
        let roster = space.parameter_density_pieces().ok_or(Error::MissingLaw)?;
        Budget::new(limits.functions, work.control())?.cells(roster.len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(roster.len())?;
        for (region, value) in roster {
            work.control().checkpoint()?;
            pieces.push(FamilyFunctionPiece {
                region,
                value: value.clone(),
            });
        }
        Self::new(space, &pieces, limits, work)
    }

    /// Designate an already normalized nonnegative family. Never rescale it.
    /// # Errors
    /// Undefined/negative/non-unit laws, capacities or cancellation.
    pub fn designate(
        &self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Space> {
        let checked = self.align_to(&self.space, limits, work)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(checked.terms.len())?;
        for (region, value) in checked.pieces() {
            work.control().checkpoint()?;
            pieces.push(ParameterDensityPiece {
                region,
                density: value.clone(),
            });
        }
        self.space.with_parameter_density(&pieces, limits, work)
    }

    /// Inspect one actual rational parameter/outcome world. `None` means the
    /// parameter is outside the domain, not an undefined payoff at a legal world.
    /// # Errors
    /// Illegal outcome encoding or structural support, capacities or cancellation.
    pub fn value_at(
        &self,
        parameter: &ExactRational,
        outcomes: u64,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<ExactRational>> {
        let checked = self.align_to(&self.space, limits, work)?;
        work.validate(parameter)?;
        let world = crate::ParameterWorld {
            parameter: crate::RealWitness::Rational(parameter.clone()),
            outcomes,
        };
        if outcomes & !self.space.outcome_coordinates() != 0 {
            return Err(Error::IllegalWorld(outcomes));
        }
        if !self
            .space
            .parameter_domain()
            .ok_or(Error::MissingParameter)?
            .region()
            .contains(&world.parameter, limits.parameters.region, work)?
        {
            return Ok(None);
        }
        self.space.full().contains_parameter(&world, limits, work)?;
        for (region, value) in checked.pieces() {
            if region.contains_parameter(&world, limits, work)? {
                return value
                    .value_at(parameter, limits.parameters.region, work)?
                    .ok_or(Error::FunctionInvariant)
                    .map(Some);
            }
        }
        Ok(Some(ExactRational::zero()))
    }

    /// Align equal named contexts; differing laws or domains still refuse.
    /// # Errors
    /// Context mismatch, capacities or cancellation.
    pub fn align_to(
        &self,
        space: &Space,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        self.space.full().align_to(space, work.control())?;
        Budget::new(limits.functions, work.control())?.cells(self.terms.len())?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.terms.len())?;
        for (region, value) in self.pieces() {
            work.control().checkpoint()?;
            pieces.push(FamilyFunctionPiece {
                region,
                value: value.clone(),
            });
        }
        Self::new(space, &pieces, limits, work)
    }

    /// Lift to an explicitly refined guard presentation without changing values.
    /// # Errors
    /// Foreign source, capacities or cancellation.
    pub fn refine(
        &self,
        refinement: &ParameterRefinement,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let checked = self.align_to(refinement.source(), limits, work)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(checked.terms.len())?;
        for (region, value) in checked.pieces() {
            pieces.push(FamilyFunctionPiece {
                region: refinement.lift(&region, work.control())?,
                value: value.clone(),
            });
        }
        Self::new(refinement.refined(), &pieces, limits, work)
    }

    fn active(
        &self,
        event: &Event,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterRegion> {
        let context = self
            .space
            .0
            .parameter
            .as_ref()
            .ok_or(Error::MissingParameter)?;
        let mut budget = SourceBudget::new(limits, work.control())?;
        budget.extent(context.fibres.len(), work.control())?;
        let mut active = ParameterRegion::empty(context.domain.parameter());
        for fibre in context.fibres.iter() {
            budget.step(work.control())?;
            if count_root(
                &self.space,
                event.root,
                context.mask,
                fibre.code,
                work.control(),
            )? != 0
            {
                active =
                    active.apply(BoolOp4::OR, &fibre.region, limits.parameters.region, work)?;
            }
        }
        Ok(active)
    }
}
