//! BESC v1; fixed-depth, explicitly versioned scalar/source envelope. The BEDC
//! map grammar and BEVT/BERA blobs retain their existing byte contracts.
use super::super::wire::{Reader, Writer};
use super::{
    AdmittedSourceDescriptor, Budget, Capacity, Control, Error, ExactArithmetic,
    FunctionDescriptor, FunctionPieceDescriptor, KernelDescriptor, Result, RevisionDescriptor,
    RevisionImpossible, RevisionOutcomeDescriptor, RevisionReceiptDescriptor, SourceDescriptor,
    SourceDescriptorLimits, shape,
};

impl SourceDescriptor {
    /// Encode plain data. This checks shape/extent only; use `admit` or `import`
    /// before trusting any embedded mathematical object or result claim.
    /// # Errors
    /// Shape/extent errors, allocation or cancellation.
    pub fn to_bytes(
        &self,
        limits: SourceDescriptorLimits,
        control: &dyn Control,
    ) -> Result<Vec<u8>> {
        self.preflight(limits, control)?;
        let mut out = Writer {
            bytes: Vec::new(),
            limits: limits.descriptors,
            control,
        };
        out.put(b"BESC\x01")?;
        match self {
            Self::Function(f) => {
                out.put(&[0])?;
                out.function(f)?;
            }
            Self::Kernel(k) => {
                out.put(&[1])?;
                out.map(&k.parent)?;
                out.function(&k.density)?;
            }
            Self::Revision(r) => {
                out.put(&[2])?;
                out.blob(&r.prior)?;
                match &r.receipt {
                    RevisionReceiptDescriptor::Condition { evidence, mass } => {
                        out.put(&[0])?;
                        out.blob(evidence)?;
                        out.blob(mass)?;
                    }
                    RevisionReceiptDescriptor::Likelihood {
                        likelihood,
                        normalizer,
                    } => {
                        out.put(&[1])?;
                        out.function(likelihood)?;
                        out.blob(normalizer)?;
                    }
                    RevisionReceiptDescriptor::Jeffrey {
                        cells,
                        old_masses,
                        targets,
                    } => {
                        out.put(&[2])?;
                        out.number(cells.len())?;
                        for ((cell, mass), target) in cells.iter().zip(old_masses).zip(targets) {
                            out.blob(cell)?;
                            out.blob(mass)?;
                            out.blob(target)?;
                        }
                    }
                }
                match &r.outcome {
                    RevisionOutcomeDescriptor::Revised { posterior } => {
                        out.put(&[0])?;
                        out.blob(posterior)?;
                    }
                    RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroEvidence) => {
                        out.put(&[1])?;
                    }
                    RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroLikelihood) => {
                        out.put(&[2])?;
                    }
                    RevisionOutcomeDescriptor::Impossible(
                        RevisionImpossible::UnsupportedTargets { cells },
                    ) => {
                        out.put(&[3])?;
                        out.number(cells.len())?;
                        for &index in cells.iter() {
                            out.number(index)?;
                        }
                    }
                }
            }
        }
        control.checkpoint()?;
        Ok(out.bytes)
    }

    /// Parse untrusted BESC data, without admitting its BEVT/BERA payloads.
    /// # Errors
    /// Unknown version/tag, malformed lengths, trailing bytes, limits/cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: SourceDescriptorLimits,
        control: &dyn Control,
    ) -> Result<Self> {
        control.checkpoint()?;
        shape(
            bytes.len(),
            limits.descriptors.bytes,
            Capacity::DescriptorBytes,
        )?;
        let mut input = Reader {
            bytes,
            budget: Budget::new(limits.descriptors),
            control,
        };
        input.budget.item(0)?;
        if input.take(4)? != b"BESC" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let value = match input.take(1)?[0] {
            0 => Self::Function(input.function(limits)?),
            1 => {
                input.budget.item(0)?;
                Self::Kernel(KernelDescriptor {
                    parent: input.map()?,
                    density: input.function(limits)?,
                })
            }
            2 => {
                input.budget.item(0)?;
                let prior = input.blob()?;
                let receipt = match input.take(1)?[0] {
                    0 => RevisionReceiptDescriptor::Condition {
                        evidence: input.blob()?,
                        mass: input.blob()?,
                    },
                    1 => RevisionReceiptDescriptor::Likelihood {
                        likelihood: input.function(limits)?,
                        normalizer: input.blob()?,
                    },
                    2 => {
                        let count = input.count()?;
                        shape(count, limits.partitions.cells, Capacity::PartitionCells)?;
                        shape(count, limits.functions.cells, Capacity::FunctionCells)?;
                        input.roster(count, 3)?;
                        let (mut cells, mut old_masses, mut targets) =
                            (Vec::new(), Vec::new(), Vec::new());
                        cells.try_reserve_exact(count)?;
                        old_masses.try_reserve_exact(count)?;
                        targets.try_reserve_exact(count)?;
                        for _ in 0..count {
                            cells.push(input.blob()?);
                            old_masses.push(input.blob()?);
                            targets.push(input.blob()?);
                        }
                        RevisionReceiptDescriptor::Jeffrey {
                            cells,
                            old_masses,
                            targets,
                        }
                    }
                    _ => return Err(Error::InvalidEncoding),
                };
                let outcome = match input.take(1)?[0] {
                    0 => RevisionOutcomeDescriptor::Revised {
                        posterior: input.blob()?,
                    },
                    1 => RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroEvidence),
                    2 => RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroLikelihood),
                    3 => {
                        let count = input.count()?;
                        shape(count, limits.partitions.cells, Capacity::PartitionCells)?;
                        let mut cells = Vec::new();
                        cells.try_reserve_exact(count)?;
                        for _ in 0..count {
                            input.budget.item(0)?;
                            cells.push(input.number()?);
                        }
                        RevisionOutcomeDescriptor::Impossible(
                            RevisionImpossible::UnsupportedTargets {
                                cells: cells.into(),
                            },
                        )
                    }
                    _ => return Err(Error::InvalidEncoding),
                };
                Self::Revision(RevisionDescriptor {
                    prior,
                    receipt,
                    outcome,
                })
            }
            _ => return Err(Error::InvalidEncoding),
        };
        if !input.bytes.is_empty() {
            return Err(Error::InvalidEncoding);
        }
        control.checkpoint()?;
        Ok(value)
    }

    /// Parse then reconstruct all native checks; only this result is executable.
    /// # Errors
    /// The combined refusal contracts of `from_bytes` and `admit`.
    pub fn import(
        bytes: &[u8],
        limits: SourceDescriptorLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<AdmittedSourceDescriptor> {
        Self::from_bytes(bytes, limits, work.control())?.admit(limits, work)
    }
}

impl Writer<'_> {
    fn function(&mut self, value: &FunctionDescriptor) -> Result<()> {
        self.blob(&value.space)?;
        self.number(value.pieces.len())?;
        for piece in &value.pieces {
            self.blob(&piece.region)?;
            self.blob(&piece.value)?;
        }
        Ok(())
    }
}

impl Reader<'_> {
    fn roster(&self, count: usize, items: usize) -> Result<()> {
        shape(
            count,
            self.budget.limits.items.saturating_sub(self.budget.items) / items,
            Capacity::DescriptorItems,
        )?;
        if count > self.bytes.len() / (8 * items) {
            return Err(Error::InvalidEncoding);
        }
        Ok(())
    }
    fn function(&mut self, limits: SourceDescriptorLimits) -> Result<FunctionDescriptor> {
        self.budget.item(0)?;
        let space = self.blob()?;
        let count = self.count()?;
        shape(count, limits.functions.cells, Capacity::FunctionCells)?;
        self.roster(count, 2)?;
        let mut pieces = Vec::new();
        pieces.try_reserve_exact(count)?;
        for _ in 0..count {
            pieces.push(FunctionPieceDescriptor {
                region: self.blob()?,
                value: self.blob()?,
            });
        }
        Ok(FunctionDescriptor { space, pieces })
    }
}
