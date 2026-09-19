//! Fixed finite laws. Density pieces partition equal per-world rational mass;
//! their regions use the same structural algebra as every other Event. The
//! owner stores raw arena references, never Events that would form an Arc cycle.
use std::collections::HashMap;
use std::sync::Arc;

use crate::arena::Ref;
use crate::{BoolOp4, Capacity, Error, Event, ExactArithmetic, ExactRational, Result, Space};

pub(crate) mod wire;

/// Bounds input pieces and the retained canonical law payload. These are not
/// a process-wide allocation quota; graph and arithmetic limits apply separately.
#[derive(Debug, Clone, Copy)]
pub struct LawLimits {
    pub cells: usize,
    pub bytes: usize,
}

impl Default for LawLimits {
    fn default() -> Self {
        Self {
            cells: 65_536,
            bytes: 16 * 1024 * 1024,
        }
    }
}

/// A constructor input: every legal world in `region` has this exact density.
/// This is per-world density, not total probability of the piece. Inputs must
/// be pairwise disjoint, including explicit zero-density pieces. Omitted worlds
/// have zero mass and remain structurally legal. No normalization is inferred.
#[derive(Debug, Clone)]
pub struct DensityPiece {
    pub region: Event,
    pub density: ExactRational,
}

#[derive(Debug)]
pub(crate) struct Cell {
    root: Ref,
    density: ExactRational,
}

#[derive(Debug)]
pub(crate) struct FiniteLaw {
    cells: Vec<Cell>,
    pub(crate) canonical: Arc<[u8]>,
}

/// An exact fixed-law conditional observation. Both masses and the designated
/// source are retained even when evidence has zero mass. The positive-evidence
/// parameter domain is either the full singleton domain or empty in this finite
/// fragment; parameterized observations will have an explicit domain object.
#[derive(Debug, Clone)]
pub struct ProbabilityObservation {
    space: Space,
    numerator: ExactRational,
    evidence: ExactRational,
}

impl ProbabilityObservation {
    #[must_use]
    pub fn space(&self) -> &Space {
        &self.space
    }
    #[must_use]
    pub fn numerator(&self) -> &ExactRational {
        &self.numerator
    }
    #[must_use]
    pub fn evidence_mass(&self) -> &ExactRational {
        &self.evidence
    }
    /// Zero evidence mass does not imply an empty evidence Event.
    #[must_use]
    pub fn is_impossible(&self) -> bool {
        self.evidence.is_zero()
    }

    /// An exact point view; `None` means undefined conditioning, never zero.
    /// # Errors
    /// Arithmetic resource limits or cancellation.
    pub fn value(&self, work: &mut ExactArithmetic<'_>) -> Result<Option<ExactRational>> {
        work.control().checkpoint()?;
        if self.is_impossible() {
            Ok(None)
        } else {
            self.numerator.div(&self.evidence, work).map(Some)
        }
    }
}

impl Space {
    /// Designate a normalized joint law without changing structural support.
    /// Every coordinate in this finite fragment is an outcome coordinate; this
    /// constructor does not accept symbolic parameter/guard interpretations.
    /// Equal densities are merged, zero pieces omitted, and the resulting law
    /// has canonical bytes independent of piece order or graph working order.
    /// An existing measured owner may be revised explicitly with a new law;
    /// existing Events remain immutable. Transfer Events with `in_space`.
    /// # Errors
    /// Foreign pieces, negative density, overlap, non-unit total, capacities or
    /// cancellation. Malformed distributions are never silently rescaled.
    pub fn with_density(
        &self,
        pieces: &[DensityPiece],
        limits: LawLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let control = work.control();
        control.checkpoint()?;
        if let Some(domain) = self.parameter_domain() {
            if pieces.len() > limits.cells {
                return Err(Error::Capacity(Capacity::LawCells));
            }
            let parameters = crate::ParameterSourceLimits {
                laws: limits,
                ..crate::ParameterSourceLimits::default()
            };
            let mut family = Vec::new();
            family.try_reserve_exact(pieces.len())?;
            for piece in pieces {
                self.full().aligned(&piece.region)?;
                piece.density.to_bytes(work)?;
                if piece.density.is_negative() {
                    return Err(Error::NegativeMass);
                }
                family.push(crate::ParameterDensityPiece {
                    region: piece.region.clone(),
                    density: crate::GuardedRationalFunction::new(
                        domain.clone(),
                        crate::ExactPolynomial::constant(piece.density.clone()),
                        crate::ExactPolynomial::one(),
                        parameters.parameters.region,
                        work,
                    )?,
                });
            }
            return self.with_parameter_density(&family, parameters, work);
        }
        if pieces.len() > limits.cells {
            return Err(Error::Capacity(Capacity::LawCells));
        }
        // All written inputs participate, including zero-density and empty pieces.
        for piece in pieces {
            self.full().aligned(&piece.region)?;
            if piece.density.is_negative() {
                return Err(Error::NegativeMass);
            }
            piece.density.to_bytes(work)?;
        }
        let mut covered = self.empty();
        let mut total = ExactRational::zero();
        let mut cells = Vec::<(Vec<u8>, Cell)>::new();
        let mut indices = HashMap::<Vec<u8>, usize>::new();
        for piece in pieces {
            control.checkpoint()?;
            if !covered
                .apply(BoolOp4::AND, &piece.region, control)?
                .is_empty()
            {
                return Err(Error::LawOverlap);
            }
            covered = covered.apply(BoolOp4::OR, &piece.region, control)?;
            let count = piece.region.count(control)?;
            let mass = piece.density.mul(&ExactRational::from(count), work)?;
            total = total.add(&mass, work)?;
            if count == 0 || piece.density.is_zero() {
                continue;
            }
            let encoding = piece.density.to_bytes(work)?;
            if let Some(&index) = indices.get(&encoding) {
                let cell: &mut Cell = &mut cells[index].1;
                cell.root = self
                    .event(cell.root)
                    .apply(BoolOp4::OR, &piece.region, control)?
                    .root;
            } else {
                indices.try_reserve(1)?;
                cells.try_reserve(1)?;
                indices.insert(encoding.clone(), cells.len());
                cells.push((
                    encoding,
                    Cell {
                        root: piece.region.root,
                        density: piece.density.clone(),
                    },
                ));
            }
        }
        if total != ExactRational::one() {
            return Err(Error::LawNotNormalized);
        }
        cells.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        let canonical = wire::encode_law(self, &cells, limits, control)?;
        let mut retained = Vec::new();
        retained.try_reserve_exact(cells.len())?;
        retained.extend(cells.into_iter().map(|(_, cell)| cell));
        self.with_measurement(
            Some(Arc::new(FiniteLaw {
                cells: retained,
                canonical: canonical.into(),
            })),
            control,
        )
    }

    #[must_use]
    pub fn is_measured(&self) -> bool {
        self.0.law.is_some()
            || self
                .0
                .parameter
                .as_ref()
                .is_some_and(|context| context.law.is_some())
    }

    /// Inspect the canonical nonzero density partition. Returned regions retain
    /// the measured owner; they can be combined, queried and persisted normally.
    #[must_use]
    pub fn density_pieces(&self) -> Option<impl ExactSizeIterator<Item = (Event, &ExactRational)>> {
        self.0.law.as_ref().map(|law| {
            law.cells
                .iter()
                .map(|cell| (self.event(cell.root), &cell.density))
        })
    }

    /// Explicitly forget the designated measurement while retaining support.
    /// # Errors
    /// Owner-token capacity or cancellation.
    pub fn unmeasured(&self, control: &dyn crate::Control) -> Result<Self> {
        self.with_measurement(None, control)
    }
}

impl Event {
    /// Contract the Boolean region with its designated finite joint law.
    /// Legal counts account for every skipped outcome bit and exclude decoder
    /// aliases. Correlation is carried by the joint density pieces themselves.
    /// # Errors
    /// Missing law, graph/arithmetic limits or cancellation.
    pub fn mass(&self, work: &mut ExactArithmetic<'_>) -> Result<ExactRational> {
        let control = work.control();
        control.checkpoint()?;
        let space = self.space();
        if space
            .0
            .parameter
            .as_ref()
            .is_some_and(|context| context.law.is_some())
        {
            return Err(Error::ParameterizedMeasurement);
        }
        let law = space.0.law.as_ref().ok_or(Error::MissingLaw)?;
        let mut result = ExactRational::zero();
        for cell in &law.cells {
            let region = self.apply(BoolOp4::AND, &space.event(cell.root), control)?;
            let count = region.count(control)?;
            result = result.add(&cell.density.mul(&ExactRational::from(count), work)?, work)?;
        }
        control.checkpoint()?;
        Ok(result)
    }

    /// Observe this Event given evidence in the same designated context.
    /// # Errors
    /// Foreign evidence (even if empty), missing law, capacities or cancellation.
    pub fn probability(
        &self,
        evidence: &Self,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ProbabilityObservation> {
        let intersection = self.apply(BoolOp4::AND, evidence, work.control())?;
        let numerator = intersection.mass(work)?;
        let evidence = evidence.mass(work)?;
        Ok(ProbabilityObservation {
            space: self.space(),
            numerator,
            evidence,
        })
    }
}
