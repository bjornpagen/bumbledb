//! BEPR v1 is an exact named univariate set: canonical algebraic boundaries
//! and independent point/sector membership. No source/law is fabricated here.
use super::{Operation, ParameterDomain, ParameterLimits, ParameterRegion};
use crate::{
    AlgebraicLimits, AlgebraicRoot, Capacity, Error, ExactArithmetic, ParameterId, Result,
};

#[derive(Debug, Clone, Copy)]
pub struct ParameterCodecLimits {
    pub region: ParameterLimits,
    pub algebraic: AlgebraicLimits,
    pub bytes: usize,
}
impl Default for ParameterCodecLimits {
    fn default() -> Self {
        Self {
            region: ParameterLimits::default(),
            algebraic: AlgebraicLimits::default(),
            bytes: 16 * 1024 * 1024,
        }
    }
}

impl ParameterRegion {
    /// Canonical BEPR v1 set identity within the captured parameter name.
    /// Boundary descriptions use BEAR minimal-polynomial/root identities, so
    /// equivalent intervals/polynomial presentations and complement polarity
    /// cannot affect these bytes. This does not designate a measured source.
    /// # Errors
    /// Region/root/factor/arithmetic/byte limits, allocation or cancellation.
    pub fn to_bytes(
        &self,
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<u8>> {
        let mut op = Operation::new(limits.region, work)?;
        op.validate(self)?;
        let mut bytes = Vec::new();
        append(&mut bytes, b"BEPR\x01", limits.bytes, &mut op)?;
        append(&mut bytes, &self.parameter.0, limits.bytes, &mut op)?;
        append(
            &mut bytes,
            &(self.partition.roots.len() as u64).to_le_bytes(),
            limits.bytes,
            &mut op,
        )?;
        for root in &self.partition.roots {
            op.step()?;
            let root = root.to_bytes(limits.algebraic, op.work)?;
            append(
                &mut bytes,
                &(root.len() as u64).to_le_bytes(),
                limits.bytes,
                &mut op,
            )?;
            append(&mut bytes, &root, limits.bytes, &mut op)?;
        }
        for index in 0..self.partition.membership.len() {
            append(
                &mut bytes,
                &[u8::from(self.member(index))],
                limits.bytes,
                &mut op,
            )?;
        }
        op.step()?;
        Ok(bytes)
    }

    /// Reconstruct exact canonical boundaries and check strict numerical order,
    /// membership and boundary necessity. No solver certificates are trusted.
    /// # Errors
    /// Malformed/noncanonical bytes, unknown version, capacities or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits.region, work)?;
        if bytes.len() > limits.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        let mut input = Reader { bytes, offset: 0 };
        if input.take(4)? != b"BEPR" {
            return Err(Error::InvalidEncoding);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let parameter = ParameterId(
            input
                .take(32)?
                .try_into()
                .map_err(|_| Error::InvalidEncoding)?,
        );
        let count = input.length()?;
        let cells = count
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::Capacity(Capacity::ParameterCells))?;
        op.extent(cells)?;
        // Every root requires a blob length and at least the BEAR header, plus
        // the final 2n+1 membership bytes. Refuse impossible extents preallocation.
        if cells > input.remaining() || count > (input.remaining() - cells) / 13 {
            return Err(Error::InvalidEncoding);
        }
        let mut roots: Vec<AlgebraicRoot> = Vec::new();
        roots.try_reserve_exact(count)?;
        for _ in 0..count {
            op.step()?;
            let length = input.length()?;
            let root = AlgebraicRoot::from_bytes(input.take(length)?, limits.algebraic, op.work)?;
            if let Some(previous) = roots.last()
                && previous.compare(&root, limits.region.roots, op.work)?
                    != std::cmp::Ordering::Less
            {
                return Err(Error::InvalidEncoding);
            }
            roots.push(root);
        }
        let raw = input.take(cells)?;
        if input.remaining() != 0 || raw.iter().any(|&b| b > 1) {
            return Err(Error::InvalidEncoding);
        }
        for i in 0..count {
            op.step()?;
            if raw[2 * i] == raw[2 * i + 1] && raw[2 * i + 1] == raw[2 * i + 2] {
                return Err(Error::InvalidEncoding);
            }
        }
        let mut membership = Vec::new();
        membership.try_reserve_exact(cells)?;
        for &bit in raw {
            op.step()?;
            membership.push(bit == 1);
        }
        let region = op.finish(parameter, roots, &membership)?;
        op.validate(&region)?;
        Ok(region)
    }
}

impl ParameterDomain {
    /// Canonical BEPR v1 of the admitted nonempty region.
    /// # Errors
    /// As `ParameterRegion::to_bytes`.
    pub fn to_bytes(
        &self,
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<u8>> {
        self.region().to_bytes(limits, work)
    }

    /// Decode a region and additionally require an inhabited source domain.
    /// # Errors
    /// As `ParameterRegion::from_bytes`, plus `EmptyParameterDomain`.
    pub fn from_bytes(
        bytes: &[u8],
        limits: ParameterCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::new(ParameterRegion::from_bytes(bytes, limits, work)?)
    }
}

fn append(
    bytes: &mut Vec<u8>,
    part: &[u8],
    limit: usize,
    op: &mut Operation<'_, '_>,
) -> Result<()> {
    op.step()?;
    if part.len() > limit.saturating_sub(bytes.len()) {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    bytes.try_reserve(part.len())?;
    bytes.extend_from_slice(part);
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(Error::InvalidEncoding)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::InvalidEncoding)?;
        self.offset = end;
        Ok(bytes)
    }
    fn length(&mut self) -> Result<usize> {
        usize::try_from(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| Error::InvalidEncoding)?,
        ))
        .map_err(|_| Error::InvalidEncoding)
    }
    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}
