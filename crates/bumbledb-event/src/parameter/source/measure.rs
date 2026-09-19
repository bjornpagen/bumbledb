//! Per-outcome rational-function laws, normalized on every real parameter fibre.
use super::{Budget, Context, ParameterSourceLimits, count_root, wire};
use crate::arena::Ref;
use crate::{
    BoolOp4, Capacity, Error, Event, ExactArithmetic, ExactPolynomial, ExactRational,
    GuardedRationalFunction, ParameterDomain, ParameterFunction, ParameterRegion, PolynomialSigns,
    Result, Space,
};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ParameterDensityPiece {
    pub region: Event,
    pub density: GuardedRationalFunction,
}
#[derive(Debug, Clone)]
pub(super) struct Cell {
    pub root: Ref,
    pub density: GuardedRationalFunction,
}
#[derive(Debug)]
pub(crate) struct Law {
    pub(super) cells: Box<[Cell]>,
    pub(crate) canonical: Arc<[u8]>,
}

/// An owned exact observation retaining its source, both original contracted
/// masses and the positive-evidence domain. Undefined never means probability zero.
#[derive(Debug, Clone)]
pub struct ParameterProbabilityObservation {
    space: Space,
    numerator: ParameterFunction,
    evidence: ParameterFunction,
    conditional: ParameterFunction,
}
impl ParameterProbabilityObservation {
    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }
    #[must_use]
    pub fn numerator(&self) -> &ParameterFunction {
        &self.numerator
    }
    #[must_use]
    pub fn evidence_mass(&self) -> &ParameterFunction {
        &self.evidence
    }
    #[must_use]
    pub fn conditional(&self) -> &ParameterFunction {
        &self.conditional
    }
    #[must_use]
    pub fn defined_on(&self) -> &ParameterRegion {
        self.conditional.defined_on()
    }
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.conditional.is_nowhere_defined()
    }
    /// # Errors
    /// Source/function/solver capacities or cancellation.
    pub fn value_at(
        &self,
        value: &ExactRational,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Option<ExactRational>> {
        self.conditional
            .value_at(value, limits.parameters.region, limits.functions, work)
    }
}

pub(super) fn constant(
    domain: &ParameterDomain,
    value: ExactRational,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<GuardedRationalFunction> {
    GuardedRationalFunction::new(
        domain.clone(),
        ExactPolynomial::constant(value),
        ExactPolynomial::one(),
        limits.parameters.region,
        work,
    )
}

impl Space {
    /// Designate per-outcome densities on an existing sealed parameter scope.
    /// Pieces are disjoint Events; omitted worlds have zero mass and remain
    /// possible. Every used density is defined and nonnegative, and each fibre
    /// sums to one. Guards select cases and are never assigned probability mass.
    /// # Errors
    /// Foreign/overlapping pieces, undefined or negative mass, non-unit fibre
    /// totals, arithmetic/graph/solver limits or cancellation.
    pub fn with_parameter_density(
        &self,
        pieces: &[ParameterDensityPiece],
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        let context = self.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        let mut budget = Budget::new(limits, control)?;
        if pieces.len() > limits.laws.cells {
            return Err(Error::Capacity(Capacity::LawCells));
        }
        let mut covered = self.empty();
        let mut cells: Vec<(Vec<u8>, Cell)> = Vec::new();
        let mut indices = HashMap::<Vec<u8>, usize>::new();
        for piece in pieces {
            budget.step(control)?;
            self.full().aligned(&piece.region)?;
            if !context.domain.region().equivalent(
                piece.density.ambient().region(),
                limits.parameters.region,
                work,
            )? {
                return Err(Error::ParameterDomainMismatch);
            }
            // The complete written function participates even for an empty Event.
            let definition = wire::encode_function(&piece.density, limits, work)?;
            if !covered
                .apply(BoolOp4::AND, &piece.region, control)?
                .is_empty()
            {
                return Err(Error::LawOverlap);
            }
            covered = covered.apply(BoolOp4::OR, &piece.region, control)?;
            if piece.region.is_empty() {
                continue;
            }
            if let Some(&index) = indices.get(&definition) {
                cells[index].1.root = self
                    .event(cells[index].1.root)
                    .apply(BoolOp4::OR, &piece.region, control)?
                    .root;
            } else {
                cells.try_reserve(1)?;
                indices.try_reserve(1)?;
                indices.insert(definition.clone(), cells.len());
                cells.push((
                    definition,
                    Cell {
                        root: piece.region.root,
                        density: piece.density.clone(),
                    },
                ));
            }
        }
        context.validate_law(self, &cells, limits, &mut budget, work)?;
        cells.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let canonical = wire::encode_law(self, &cells, limits, control)?.into();
        let mut retained = Vec::new();
        retained.try_reserve_exact(cells.len())?;
        retained.extend(cells.into_iter().map(|(_, cell)| cell));
        let context = Context {
            law: Some(Arc::new(Law {
                cells: retained.into_boxed_slice(),
                canonical,
            })),
            ..context.as_ref().clone()
        };
        self.with_parameter_context(Arc::new(context), control)
    }

    #[must_use]
    pub fn parameter_density_pieces(
        &self,
    ) -> Option<impl ExactSizeIterator<Item = (Event, &GuardedRationalFunction)>> {
        self.0
            .parameter
            .as_ref()
            .and_then(|p| p.law.as_ref())
            .map(|law| {
                law.cells
                    .iter()
                    .map(|cell| (self.event(cell.root), &cell.density))
            })
    }
}

impl Event {
    /// Sum outcomes within each exact parameter case. Skipped outcome bits
    /// contribute multiplicity; guard cases remain separate function pieces.
    /// # Errors
    /// Missing parameter/law, capacities or cancellation.
    pub fn parameter_mass(
        &self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterFunction> {
        let control = work.control();
        let space = self.space();
        let context = space.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
        let law = context.law.as_ref().ok_or(Error::MissingLaw)?;
        let mut budget = Budget::new(limits, control)?;
        if law.cells.len() > limits.laws.cells {
            return Err(Error::Capacity(Capacity::LawCells));
        }
        budget.extent(context.fibres.len(), control)?;
        for cell in &law.cells {
            budget.step(control)?;
            cell.density
                .restrict(context.domain.region(), limits.parameters.region, work)?;
        }
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(context.fibres.len())?;
        for fibre in context.fibres.iter() {
            budget.step(control)?;
            let mut total = constant(&context.domain, ExactRational::zero(), limits, work)?
                .restrict(&fibre.region, limits.parameters.region, work)?;
            for cell in &law.cells {
                budget.step(control)?;
                let intersection = self.apply(BoolOp4::AND, &space.event(cell.root), control)?;
                let count =
                    count_root(&space, intersection.root, context.mask, fibre.code, control)?;
                if count != 0 {
                    let density =
                        cell.density
                            .restrict(&fibre.region, limits.parameters.region, work)?;
                    let term = density.mul(
                        &constant(&context.domain, ExactRational::from(count), limits, work)?,
                        limits.parameters.region,
                        work,
                    )?;
                    total = total.add(&term, limits.parameters.region, work)?;
                }
            }
            pieces.push(total);
        }
        ParameterFunction::new(
            context.domain.clone(),
            &pieces,
            limits.parameters.region,
            limits.functions,
            work,
        )
    }

    /// Exact source-owned conditional observation, retaining original masses
    /// and the precise positive-evidence domain, including endpoint holes.
    /// # Errors
    /// Foreign evidence, missing parameter/law, capacities or cancellation.
    pub fn parameter_probability(
        &self,
        evidence: &Self,
        limits: ParameterSourceLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ParameterProbabilityObservation> {
        let intersection = self.apply(BoolOp4::AND, evidence, work.control())?;
        let numerator = intersection.parameter_mass(limits, work)?;
        let evidence = evidence.parameter_mass(limits, work)?;
        let conditional =
            numerator.div(&evidence, limits.parameters.region, limits.functions, work)?;
        Ok(ParameterProbabilityObservation {
            space: self.space(),
            numerator,
            evidence,
            conditional,
        })
    }
}

impl Context {
    fn validate_law(
        &self,
        space: &Space,
        cells: &[(Vec<u8>, Cell)],
        limits: ParameterSourceLimits,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        let control = work.control();
        budget.extent(self.fibres.len(), control)?;
        // Densities may have holes outside their Event's active guard cases.
        // Legal cells with zero mass are not deleted from structural support.
        for fibre in self.fibres.iter() {
            budget.step(control)?;
            let mut total = constant(&self.domain, ExactRational::zero(), limits, work)?.restrict(
                &fibre.region,
                limits.parameters.region,
                work,
            )?;
            for (_, cell) in cells {
                budget.step(control)?;
                let count = count_root(space, cell.root, self.mask, fibre.code, control)?;
                if count == 0 {
                    continue;
                }
                if !fibre.region.included(
                    cell.density.defined_on(),
                    limits.parameters.region,
                    work,
                )? {
                    return Err(Error::UndefinedDensity);
                }
                let density =
                    cell.density
                        .restrict(&fibre.region, limits.parameters.region, work)?;
                if !density
                    .where_sign(PolynomialSigns::NEGATIVE, limits.parameters.region, work)?
                    .is_empty()
                {
                    return Err(Error::NegativeMass);
                }
                let term = density.mul(
                    &constant(&self.domain, ExactRational::from(count), limits, work)?,
                    limits.parameters.region,
                    work,
                )?;
                total = total.add(&term, limits.parameters.region, work)?;
            }
            let difference = total.sub(
                &constant(&self.domain, ExactRational::one(), limits, work)?,
                limits.parameters.region,
                work,
            )?;
            if !fibre
                .region
                .included(total.defined_on(), limits.parameters.region, work)?
                || !difference
                    .where_sign(PolynomialSigns::NON_ZERO, limits.parameters.region, work)?
                    .is_empty()
            {
                return Err(Error::LawNotNormalized);
            }
        }
        Ok(())
    }
}
