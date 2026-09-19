//! BESC v1 fixed-law and v2 family envelopes, with fixed-depth grammars. BEDC
//! maps and embedded Event/arithmetic formats retain their byte contracts.
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
        out.put(if matches!(self, Self::Family(_)) {
            b"BESC\x02"
        } else {
            b"BESC\x01"
        })?;
        match self {
            Self::Family(f) => f.write(&mut out)?,
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

    /// Parse untrusted BESC data, without admitting its mathematical payloads.
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
        if version == 2 {
            let value = Self::Family(Box::new(super::FamilyDescriptor::read(&mut input, limits)?));
            if !input.bytes.is_empty() {
                return Err(Error::InvalidEncoding);
            }
            control.checkpoint()?;
            return Ok(value);
        }
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let value = input.fixed_source(limits)?;
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
    fn fixed_source(&mut self, limits: SourceDescriptorLimits) -> Result<SourceDescriptor> {
        Ok(match self.take(1)?[0] {
            0 => SourceDescriptor::Function(self.function(limits)?),
            1 => {
                self.budget.item(0)?;
                SourceDescriptor::Kernel(KernelDescriptor {
                    parent: self.map()?,
                    density: self.function(limits)?,
                })
            }
            2 => {
                self.budget.item(0)?;
                let prior = self.blob()?;
                let receipt = match self.take(1)?[0] {
                    0 => RevisionReceiptDescriptor::Condition {
                        evidence: self.blob()?,
                        mass: self.blob()?,
                    },
                    1 => RevisionReceiptDescriptor::Likelihood {
                        likelihood: self.function(limits)?,
                        normalizer: self.blob()?,
                    },
                    2 => {
                        let count = self.count()?;
                        shape(count, limits.partitions.cells, Capacity::PartitionCells)?;
                        shape(count, limits.functions.cells, Capacity::FunctionCells)?;
                        self.roster(count, 3)?;
                        let (mut cells, mut old_masses, mut targets) =
                            (Vec::new(), Vec::new(), Vec::new());
                        cells.try_reserve_exact(count)?;
                        old_masses.try_reserve_exact(count)?;
                        targets.try_reserve_exact(count)?;
                        for _ in 0..count {
                            cells.push(self.blob()?);
                            old_masses.push(self.blob()?);
                            targets.push(self.blob()?);
                        }
                        RevisionReceiptDescriptor::Jeffrey {
                            cells,
                            old_masses,
                            targets,
                        }
                    }
                    _ => return Err(Error::InvalidEncoding),
                };
                let outcome = match self.take(1)?[0] {
                    0 => RevisionOutcomeDescriptor::Revised {
                        posterior: self.blob()?,
                    },
                    1 => RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroEvidence),
                    2 => RevisionOutcomeDescriptor::Impossible(RevisionImpossible::ZeroLikelihood),
                    3 => {
                        let count = self.count()?;
                        shape(count, limits.partitions.cells, Capacity::PartitionCells)?;
                        let mut cells = Vec::new();
                        cells.try_reserve_exact(count)?;
                        for _ in 0..count {
                            self.budget.item(0)?;
                            cells.push(self.number()?);
                        }
                        RevisionOutcomeDescriptor::Impossible(
                            RevisionImpossible::UnsupportedTargets {
                                cells: cells.into(),
                            },
                        )
                    }
                    _ => return Err(Error::InvalidEncoding),
                };
                SourceDescriptor::Revision(RevisionDescriptor {
                    prior,
                    receipt,
                    outcome,
                })
            }
            _ => return Err(Error::InvalidEncoding),
        })
    }

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
