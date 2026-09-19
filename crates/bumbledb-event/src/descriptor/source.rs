//! Portable fixed-law scalar objects. Parsing never grants mathematical claims;
//! admission replays native constructors and checks every recorded revision result.
use crate::{
    EventPartition, ExactArithmetic, ExactRational, FiniteFunction, FiniteKernel, FunctionLimits,
    FunctionPiece, LawLimits, PartitionLimits, RevisionImpossible, RevisionOutcome,
    RevisionReceipt, SourceRevision,
};

use super::{Budget, Capacity, Control, DescriptorLimits, Error, MapDescriptor, Result};

mod wire;

/// Nonzero scalar cells on a full named context. Plain inputs may use redundant
/// disjoint cells; admission merges equal values and supplies the zero default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDescriptor {
    pub space: Vec<u8>,
    pub pieces: Vec<FunctionPieceDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionPieceDescriptor {
    pub region: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDescriptor {
    pub parent: MapDescriptor,
    pub density: FunctionDescriptor,
}

/// All scalar blobs are canonical BERA values; Event blobs are BEVT values.
/// A Jeffrey receipt's cell roster is indexed, including every empty cell.
/// Its parent is the full prior in the enclosing revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionReceiptDescriptor {
    Condition {
        evidence: Vec<u8>,
        mass: Vec<u8>,
    },
    Likelihood {
        likelihood: FunctionDescriptor,
        normalizer: Vec<u8>,
    },
    Jeffrey {
        cells: Vec<Vec<u8>>,
        old_masses: Vec<Vec<u8>>,
        targets: Vec<Vec<u8>>,
    },
}

/// Claimed result, not a certificate. Admission recomputes it, including all
/// impossible-target positions and the posterior's complete named law/support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionOutcomeDescriptor {
    Revised { posterior: Vec<u8> },
    Impossible(RevisionImpossible),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionDescriptor {
    pub prior: Vec<u8>,
    pub receipt: RevisionReceiptDescriptor,
    pub outcome: RevisionOutcomeDescriptor,
}

/// Inspectable, untrusted fixed-law transport. Source history is separate from
/// canonical Event identity. No provider provenance or fresh draw is inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceDescriptor {
    Function(FunctionDescriptor),
    Kernel(KernelDescriptor),
    Revision(RevisionDescriptor),
}

#[derive(Debug, Clone)]
pub enum AdmittedSourceDescriptor {
    Function(FiniteFunction),
    Kernel(FiniteKernel),
    Revision(SourceRevision),
}

/// Envelope/roster and native-operation bounds. The caller separately supplies
/// exact-arithmetic work shared across scalar decoding and reconstruction.
/// These bounds are not an aggregate retained-memory quota.
#[derive(Debug, Clone, Copy, Default)]
pub struct SourceDescriptorLimits {
    pub descriptors: DescriptorLimits,
    pub functions: FunctionLimits,
    pub laws: LawLimits,
    pub partitions: PartitionLimits,
}

fn shape(count: usize, limit: usize, capacity: Capacity) -> Result<()> {
    if count > limit {
        return Err(Error::Capacity(capacity));
    }
    Ok(())
}

impl Budget {
    fn rational(
        &mut self,
        value: &ExactRational,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<u8>> {
        let bytes = value.to_bytes(work)?;
        self.item(bytes.len())?;
        Ok(bytes)
    }
}

impl FunctionDescriptor {
    fn capture(
        value: &FiniteFunction,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        let space = budget.event(&value.space().full(), work.control())?;
        // Bound the roster before reserving it; every piece contains two items.
        shape(
            value.pieces().len(),
            budget.limits.items.saturating_sub(budget.items) / 2,
            Capacity::DescriptorItems,
        )?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(value.pieces().len())?;
        for (region, value) in value.pieces() {
            pieces.push(FunctionPieceDescriptor {
                region: budget.event(&region, work.control())?,
                value: budget.rational(value, work)?,
            });
        }
        Ok(Self { space, pieces })
    }

    fn preflight(
        &self,
        budget: &mut Budget,
        limits: SourceDescriptorLimits,
        control: &dyn Control,
    ) -> Result<()> {
        shape(
            self.pieces.len(),
            limits.functions.cells,
            Capacity::FunctionCells,
        )?;
        budget.item(0)?;
        budget.item(self.space.len())?;
        for piece in &self.pieces {
            control.checkpoint()?;
            budget.item(piece.region.len())?;
            budget.item(piece.value.len())?;
        }
        Ok(())
    }

    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<FiniteFunction> {
        let space = full_space(&self.space, limits, work)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(self.pieces.len())?;
        for piece in &self.pieces {
            pieces.push(FunctionPiece {
                region: event(&piece.region, limits, work)?,
                value: ExactRational::from_bytes(&piece.value, work)?,
            });
        }
        FiniteFunction::new(&space, &pieces, limits.functions, work)
    }
}

impl RevisionDescriptor {
    fn capture(
        value: &SourceRevision,
        budget: &mut Budget,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        budget.item(0)?;
        let prior = budget.event(&value.prior().full(), work.control())?;
        let receipt = match value.receipt() {
            RevisionReceipt::Condition { evidence, mass } => RevisionReceiptDescriptor::Condition {
                evidence: budget.event(evidence, work.control())?,
                mass: budget.rational(mass, work)?,
            },
            RevisionReceipt::Likelihood {
                likelihood,
                normalizer,
            } => RevisionReceiptDescriptor::Likelihood {
                likelihood: FunctionDescriptor::capture(likelihood, budget, work)?,
                normalizer: budget.rational(normalizer, work)?,
            },
            RevisionReceipt::Jeffrey {
                partition,
                old_masses,
                targets,
            } => {
                let n = partition.cells().len();
                shape(
                    n,
                    budget.limits.items.saturating_sub(budget.items) / 3,
                    Capacity::DescriptorItems,
                )?;
                let (mut cells, mut masses, mut values) = (Vec::new(), Vec::new(), Vec::new());
                cells.try_reserve_exact(n)?;
                masses.try_reserve_exact(n)?;
                values.try_reserve_exact(n)?;
                for ((cell, mass), target) in partition
                    .cells()
                    .iter()
                    .zip(old_masses.iter())
                    .zip(targets.iter())
                {
                    cells.push(budget.event(cell, work.control())?);
                    masses.push(budget.rational(mass, work)?);
                    values.push(budget.rational(target, work)?);
                }
                RevisionReceiptDescriptor::Jeffrey {
                    cells,
                    old_masses: masses,
                    targets: values,
                }
            }
        };
        let outcome = match value.outcome() {
            RevisionOutcome::Revised(value) => RevisionOutcomeDescriptor::Revised {
                posterior: budget.event(&value.space().full(), work.control())?,
            },
            RevisionOutcome::Impossible(value) => {
                if let RevisionImpossible::UnsupportedTargets { cells } = value {
                    for _ in cells.iter() {
                        work.control().checkpoint()?;
                        budget.item(0)?;
                    }
                }
                RevisionOutcomeDescriptor::Impossible(value.clone())
            }
        };
        Ok(Self {
            prior,
            receipt,
            outcome,
        })
    }

    fn preflight(
        &self,
        budget: &mut Budget,
        limits: SourceDescriptorLimits,
        control: &dyn Control,
    ) -> Result<()> {
        budget.item(0)?;
        budget.item(self.prior.len())?;
        match &self.receipt {
            RevisionReceiptDescriptor::Condition { evidence, mass } => {
                budget.item(evidence.len())?;
                budget.item(mass.len())?;
            }
            RevisionReceiptDescriptor::Likelihood {
                likelihood,
                normalizer,
            } => {
                likelihood.preflight(budget, limits, control)?;
                budget.item(normalizer.len())?;
            }
            RevisionReceiptDescriptor::Jeffrey {
                cells,
                old_masses,
                targets,
            } => {
                if cells.len() != old_masses.len() || cells.len() != targets.len() {
                    return Err(Error::PartitionArity);
                }
                shape(
                    cells.len(),
                    limits.partitions.cells,
                    Capacity::PartitionCells,
                )?;
                shape(cells.len(), limits.functions.cells, Capacity::FunctionCells)?;
                for bytes in cells.iter().chain(old_masses).chain(targets) {
                    control.checkpoint()?;
                    budget.item(bytes.len())?;
                }
            }
        }
        match &self.outcome {
            RevisionOutcomeDescriptor::Revised { posterior } => budget.item(posterior.len())?,
            RevisionOutcomeDescriptor::Impossible(RevisionImpossible::UnsupportedTargets {
                cells,
            }) => {
                shape(
                    cells.len(),
                    limits.partitions.cells,
                    Capacity::PartitionCells,
                )?;
                for _ in cells.iter() {
                    control.checkpoint()?;
                    budget.item(0)?;
                }
            }
            RevisionOutcomeDescriptor::Impossible(_) => {}
        }
        Ok(())
    }

    fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<SourceRevision> {
        let control = work.control();
        let prior = full_space(&self.prior, limits, work)?;
        let revision = match &self.receipt {
            RevisionReceiptDescriptor::Condition { evidence, mass } => {
                let evidence = event(evidence, limits, work)?;
                let expected = ExactRational::from_bytes(mass, work)?;
                let value = prior.condition(&evidence, limits.functions, limits.laws, work)?;
                let RevisionReceipt::Condition { mass, .. } = value.receipt() else {
                    unreachable!()
                };
                claim(mass == &expected)?;
                value
            }
            RevisionReceiptDescriptor::Likelihood {
                likelihood,
                normalizer,
            } => {
                let likelihood = likelihood.admit(limits, work)?;
                let expected = ExactRational::from_bytes(normalizer, work)?;
                let value = prior.likelihood(&likelihood, limits.functions, limits.laws, work)?;
                let RevisionReceipt::Likelihood { normalizer, .. } = value.receipt() else {
                    unreachable!()
                };
                claim(normalizer == &expected)?;
                value
            }
            RevisionReceiptDescriptor::Jeffrey {
                cells,
                old_masses,
                targets,
            } => {
                let (mut decoded_cells, mut masses, mut values) =
                    (Vec::new(), Vec::new(), Vec::new());
                decoded_cells.try_reserve_exact(cells.len())?;
                masses.try_reserve_exact(cells.len())?;
                values.try_reserve_exact(cells.len())?;
                for ((cell, mass), target) in cells.iter().zip(old_masses).zip(targets) {
                    decoded_cells.push(event(cell, limits, work)?);
                    masses.push(ExactRational::from_bytes(mass, work)?);
                    values.push(ExactRational::from_bytes(target, work)?);
                }
                let partition =
                    EventPartition::on(&prior.full(), &decoded_cells, limits.partitions, control)?;
                let value =
                    prior.jeffrey(&partition, &values, limits.functions, limits.laws, work)?;
                let RevisionReceipt::Jeffrey { old_masses, .. } = value.receipt() else {
                    unreachable!()
                };
                claim(old_masses.as_ref() == masses)?;
                value
            }
        };
        match (&self.outcome, revision.outcome()) {
            (
                RevisionOutcomeDescriptor::Revised { posterior },
                RevisionOutcome::Revised(actual),
            ) => {
                let expected = full_space(posterior, limits, work)?;
                match expected.full().align_to(actual.space(), control) {
                    Ok(_) => {}
                    Err(Error::SpaceMismatch) => return Err(Error::DescriptorClaimMismatch),
                    Err(error) => return Err(error),
                }
            }
            (
                RevisionOutcomeDescriptor::Impossible(expected),
                RevisionOutcome::Impossible(actual),
            ) => claim(expected == actual)?,
            _ => return Err(Error::DescriptorClaimMismatch),
        }
        Ok(revision)
    }
}

fn claim(valid: bool) -> Result<()> {
    if valid {
        Ok(())
    } else {
        Err(Error::DescriptorClaimMismatch)
    }
}

impl SourceDescriptor {
    /// Capture canonical native inputs and claims, independent of manager order.
    /// # Errors
    /// Encoding, shape/arithmetic limits, allocation or cancellation.
    pub fn capture(
        value: &AdmittedSourceDescriptor,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        work.control().checkpoint()?;
        // Native shape bounds are checked before building the portable roster.
        match value {
            AdmittedSourceDescriptor::Function(f) => shape(
                f.pieces().len(),
                limits.functions.cells,
                Capacity::FunctionCells,
            )?,
            AdmittedSourceDescriptor::Kernel(k) => shape(
                k.density().pieces().len(),
                limits.functions.cells,
                Capacity::FunctionCells,
            )?,
            AdmittedSourceDescriptor::Revision(r) => match r.receipt() {
                RevisionReceipt::Likelihood { likelihood, .. } => shape(
                    likelihood.pieces().len(),
                    limits.functions.cells,
                    Capacity::FunctionCells,
                )?,
                RevisionReceipt::Jeffrey { partition, .. } => {
                    shape(
                        partition.cells().len(),
                        limits.partitions.cells,
                        Capacity::PartitionCells,
                    )?;
                    shape(
                        partition.cells().len(),
                        limits.functions.cells,
                        Capacity::FunctionCells,
                    )?;
                }
                RevisionReceipt::Condition { .. } => {}
            },
        }
        let mut budget = Budget::new(limits.descriptors);
        budget.item(0)?;
        let result = match value {
            AdmittedSourceDescriptor::Function(f) => {
                Self::Function(FunctionDescriptor::capture(f, &mut budget, work)?)
            }
            AdmittedSourceDescriptor::Kernel(k) => {
                budget.item(0)?;
                Self::Kernel(KernelDescriptor {
                    parent: MapDescriptor::capture(k.parent().map(), &mut budget, work.control())?,
                    density: FunctionDescriptor::capture(k.density(), &mut budget, work)?,
                })
            }
            AdmittedSourceDescriptor::Revision(r) => {
                Self::Revision(RevisionDescriptor::capture(r, &mut budget, work)?)
            }
        };
        work.control().checkpoint()?;
        Ok(result)
    }

    fn preflight(&self, limits: SourceDescriptorLimits, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        let mut budget = Budget::new(limits.descriptors);
        budget.item(0)?;
        match self {
            Self::Function(f) => f.preflight(&mut budget, limits, control),
            Self::Kernel(k) => {
                budget.item(0)?;
                k.parent.preflight(&mut budget, control)?;
                k.density.preflight(&mut budget, limits, control)
            }
            Self::Revision(r) => r.preflight(&mut budget, limits, control),
        }
    }

    /// Rebuild functions/channels and replay revisions from their complete prior
    /// and inputs. Every stored mass, normalizer and outcome is checked anew.
    /// # Errors
    /// Malformed/context/normalization errors, false claims, limits or cancellation.
    /// No error is converted into an impossible mathematical outcome.
    pub fn admit(
        &self,
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedSourceDescriptor> {
        self.preflight(limits, work.control())?;
        let result = match self {
            Self::Function(f) => AdmittedSourceDescriptor::Function(f.admit(limits, work)?),
            Self::Kernel(k) => {
                let parent = admit_map(&k.parent, limits, work)?;
                let density = k.density.admit(limits, work)?;
                AdmittedSourceDescriptor::Kernel(FiniteKernel::new(
                    &parent,
                    &density,
                    limits.functions,
                    work,
                )?)
            }
            Self::Revision(r) => AdmittedSourceDescriptor::Revision(r.admit(limits, work)?),
        };
        work.control().checkpoint()?;
        Ok(result)
    }
}

fn event(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<crate::Event> {
    if bytes.get(..5) == Some(b"BEVT\x02") {
        crate::measure::wire::decode_with_work(
            bytes,
            None,
            limits.descriptors.events,
            limits.laws,
            work,
        )
    } else {
        crate::Event::from_bytes_with_order(bytes, None, limits.descriptors.events, work.control())
    }
}

fn full_space(
    bytes: &[u8],
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<crate::Space> {
    let value = event(bytes, limits, work)?;
    if !value.is_full() {
        return Err(Error::InvalidEncoding);
    }
    Ok(value.space())
}

fn admit_map(
    value: &MapDescriptor,
    limits: SourceDescriptorLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<crate::CoordinateMap> {
    let source = full_space(&value.source, limits, work)?;
    let target = full_space(&value.target, limits, work)?;
    if value.readouts.len() != usize::from(target.dimensions()) {
        return Err(Error::MapArity);
    }
    let mut readouts = Vec::new();
    readouts.try_reserve_exact(value.readouts.len())?;
    for bytes in &value.readouts {
        readouts.push(event(bytes, limits, work)?);
    }
    crate::CoordinateMap::new(&source, &target, &readouts, work.control())
}
