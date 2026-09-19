//! BEPL v1 transports polynomial identity, not a law or a source domain.
use super::{ExactPolynomial, Operation, ParameterId, PolynomialLimits, PolynomialTerm};
use crate::{Capacity, Error, ExactArithmetic, ExactRational, Result};

impl ExactPolynomial {
    /// Canonical BEPL v1: named parameters, positive natural powers and exact
    /// BERA coefficients in the sparse normal-form order. No owner IDs occur.
    /// # Errors
    /// Polynomial/arithmetic/byte extents, allocation or cancellation.
    pub fn to_bytes(
        &self,
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Vec<u8>> {
        let mut op = Operation::new(limits, work)?;
        op.validate(self)?;
        let mut output = Vec::new();
        append(&mut output, b"BEPL\x01", &mut op)?;
        append(
            &mut output,
            &(self.terms.len() as u64).to_le_bytes(),
            &mut op,
        )?;
        for term in &self.terms {
            let coefficient = term.coefficient.to_bytes(op.work)?;
            append(
                &mut output,
                &(coefficient.len() as u64).to_le_bytes(),
                &mut op,
            )?;
            append(&mut output, &coefficient, &mut op)?;
            append(
                &mut output,
                &(term.powers.len() as u64).to_le_bytes(),
                &mut op,
            )?;
            for &(parameter, exponent) in &term.powers {
                append(&mut output, &parameter.0, &mut op)?;
                append(&mut output, &exponent.to_le_bytes(), &mut op)?;
            }
        }
        op.step()?;
        Ok(output)
    }

    /// Parse and admit only canonical BEPL v1. Unordered or repeated monomials,
    /// zero coefficients/powers, duplicate parameters and trailing bytes refuse.
    /// # Errors
    /// Malformed/noncanonical bytes, unknown version, limits or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: PolynomialLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let mut op = Operation::new(limits, work)?;
        if bytes.len() > limits.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        let mut input = Reader { bytes, offset: 0 };
        if input.take(4)? != b"BEPL" {
            return Err(Error::InvalidPolynomial);
        }
        let version = input.take(1)?[0];
        if version != 1 {
            return Err(Error::UnsupportedVersion(version));
        }
        let count = input.length()?;
        op.term_extent(count)?;
        // Each nonzero term needs two lengths and at least 24 BERA bytes.
        if count > input.remaining() / 40 {
            return Err(Error::InvalidPolynomial);
        }
        let mut terms = Vec::<PolynomialTerm>::new();
        terms.try_reserve_exact(count)?;
        for _ in 0..count {
            op.step()?;
            let size = input.length()?;
            let coefficient = ExactRational::from_bytes(input.take(size)?, op.work)?;
            if coefficient.is_zero() {
                return Err(Error::InvalidPolynomial);
            }
            let count = input.length()?;
            op.factor_extent(count)?;
            if count > input.remaining() / 36 {
                return Err(Error::InvalidPolynomial);
            }
            let mut powers = Vec::<(ParameterId, u32)>::new();
            powers.try_reserve_exact(count)?;
            for _ in 0..count {
                op.step()?;
                let parameter = ParameterId(
                    input
                        .take(32)?
                        .try_into()
                        .map_err(|_| Error::InvalidPolynomial)?,
                );
                let exponent = u32::from_le_bytes(
                    input
                        .take(4)?
                        .try_into()
                        .map_err(|_| Error::InvalidPolynomial)?,
                );
                if exponent == 0 || powers.last().is_some_and(|p| p.0 >= parameter) {
                    return Err(Error::InvalidPolynomial);
                }
                powers.push((parameter, exponent));
            }
            let term = PolynomialTerm {
                coefficient,
                powers: powers.into_boxed_slice(),
            };
            op.validate_term(&term)?;
            if terms.last().is_some_and(|t| t.powers >= term.powers) {
                return Err(Error::InvalidPolynomial);
            }
            terms.push(term);
        }
        if input.remaining() != 0 {
            return Err(Error::InvalidPolynomial);
        }
        op.step()?;
        Ok(Self {
            terms: terms.into_boxed_slice(),
        })
    }
}

fn append(bytes: &mut Vec<u8>, part: &[u8], op: &mut Operation<'_, '_>) -> Result<()> {
    op.step()?;
    if part.len() > op.limits.bytes.saturating_sub(bytes.len()) {
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
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(Error::InvalidPolynomial)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::InvalidPolynomial)?;
        self.offset = end;
        Ok(bytes)
    }

    fn length(&mut self) -> Result<usize> {
        usize::try_from(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| Error::InvalidPolynomial)?,
        ))
        .map_err(|_| Error::Capacity(Capacity::DescriptorBytes))
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}
