//! BENP v1/v2: replay the entire predicate and its embedded BENO node grammar.
//! Version 2 adds exact region membership; trees without it retain v1 bytes.
//! One byte/item/node budget spans both trees. No truth partition is trusted.
#![allow(clippy::large_types_passed_by_value)]
use super::super::{
    ObservationNumberCodecLimits,
    import::{Budget, Reader, Writer},
};
use super::{ObservationPredicate, ObservationPredicateExpr};
use crate::Result;
use crate::event::{
    BoolOp4, Capacity, Error, ExactArithmetic, ParameterDomain, ParameterRegion, PolynomialSigns,
};
use std::sync::Arc;

#[derive(Debug)]
struct Import {
    value: ObservationPredicate,
    bytes: Box<[u8]>,
}

/// An owned replay-checked predicate. Identity includes complete numerical
/// derivations, domains and Boolean syntax, independently of allocation order.
#[derive(Debug, Clone)]
pub struct ObservationPredicateImport(Arc<Import>);
impl PartialEq for ObservationPredicateImport {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.bytes == other.0.bytes
    }
}
impl Eq for ObservationPredicateImport {}
impl std::hash::Hash for ObservationPredicateImport {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.bytes.hash(state);
    }
}
impl ObservationPredicateImport {
    /// Executor-produced predicates have already admitted every operand under
    /// the shared budget. Encode identity without recontracting their leaves.
    pub(crate) fn checked(
        value: ObservationPredicate,
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let bytes = encode(&value, limits, work)?.into_boxed_slice();
        Ok(Self(Arc::new(Import { value, bytes })))
    }
    /// Reconstruct independently owned leaves and replay every operation.
    /// # Errors
    /// Source/numerical/domain admission, encoding limits or cancellation.
    pub fn capture(
        value: &ObservationPredicate,
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::from_bytes(&encode(value, limits, work)?, limits, work)
    }

    /// Admit the complete expression under one shared exact-arithmetic counter.
    /// # Errors
    /// Malformed/noncanonical input, source/domain/coverage errors, limits or cancellation.
    pub fn from_bytes(
        bytes: &[u8],
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        work.control().checkpoint()?;
        if bytes.len() > limits.sources.descriptors.bytes {
            return Err(Error::Capacity(Capacity::DescriptorBytes).into());
        }
        let mut input = Reader {
            bytes,
            offset: 0,
            budget: Budget::new(limits),
        };
        if input.take(4)? != b"BENP" {
            return Err(Error::InvalidEncoding.into());
        }
        let version = input.byte()?;
        if !matches!(version, 1 | 2) {
            return Err(Error::UnsupportedVersion(version).into());
        }
        let value = read(&mut input, version, limits, work)?;
        if input.offset != bytes.len() {
            return Err(Error::InvalidEncoding.into());
        }
        let canonical = encode(&value, limits, work)?;
        if canonical != bytes {
            return Err(Error::InvalidEncoding.into());
        }
        work.control().checkpoint()?;
        Ok(Self(Arc::new(Import {
            value,
            bytes: canonical.into_boxed_slice(),
        })))
    }

    #[must_use]
    pub fn value(&self) -> &ObservationPredicate {
        &self.0.value
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
}

fn encode(
    value: &ObservationPredicate,
    limits: ObservationNumberCodecLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    let mut output = Writer {
        bytes: Vec::new(),
        budget: Budget::new(limits),
    };
    output.put(b"BENP\x01", work.control())?;
    let mut pending = Vec::new();
    pending.try_reserve(1).map_err(Error::from)?;
    pending.push((value, 1));
    while let Some((value, depth)) = pending.pop() {
        output.budget.node(depth, work.control())?;
        pending.try_reserve(2).map_err(Error::from)?;
        match value.expression() {
            ObservationPredicateExpr::Region { domain, region } => {
                output.bytes[4] = 2;
                output.put(&[4], work.control())?;
                output.blob(
                    &domain.to_bytes(limits.sources.parameters.parameters, work)?,
                    work.control(),
                )?;
                output.blob(
                    &region.to_bytes(limits.sources.parameters.parameters, work)?,
                    work.control(),
                )?;
            }
            ObservationPredicateExpr::Sign { number, signs } => {
                output.put(&[0, signs.bits()], work.control())?;
                output.node(number, depth + 1, work)?;
            }
            ObservationPredicateExpr::Negate(value) => {
                output.put(&[1], work.control())?;
                pending.push((value, depth + 1));
            }
            ObservationPredicateExpr::Binary { op, left, right } => {
                output.put(&[2, op.bits()], work.control())?;
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
            ObservationPredicateExpr::OnDomain { value, domain } => {
                output.put(&[3], work.control())?;
                output.blob(
                    &domain.to_bytes(limits.sources.parameters.parameters, work)?,
                    work.control(),
                )?;
                pending.push((value, depth + 1));
            }
        }
    }
    Ok(output.bytes)
}

fn read(
    input: &mut Reader<'_>,
    version: u8,
    limits: ObservationNumberCodecLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<ObservationPredicate> {
    enum Pending {
        Read(usize),
        Negate,
        Binary(BoolOp4),
        OnDomain(ParameterDomain),
    }
    let mut pending = Vec::new();
    let mut values = Vec::<ObservationPredicate>::new();
    pending.try_reserve(1).map_err(Error::from)?;
    pending.push(Pending::Read(1));
    while let Some(task) = pending.pop() {
        work.control().checkpoint()?;
        pending.try_reserve(3).map_err(Error::from)?;
        values.try_reserve(1).map_err(Error::from)?;
        let value = match task {
            Pending::Read(depth) => {
                input.budget.node(depth, work.control())?;
                match input.byte()? {
                    0 => {
                        let signs =
                            PolynomialSigns::new(input.byte()?).ok_or(Error::InvalidEncoding)?;
                        input
                            .node(depth + 1, work)?
                            .where_sign(signs, limits.numbers, work)?
                    }
                    1 => {
                        pending.push(Pending::Negate);
                        pending.push(Pending::Read(depth + 1));
                        continue;
                    }
                    2 => {
                        let op = BoolOp4::new(input.byte()?).ok_or(Error::InvalidEncoding)?;
                        pending.push(Pending::Binary(op));
                        pending.push(Pending::Read(depth + 1));
                        pending.push(Pending::Read(depth + 1));
                        continue;
                    }
                    3 => {
                        let domain = ParameterDomain::from_bytes(
                            input.blob(work.control())?,
                            limits.sources.parameters.parameters,
                            work,
                        )?;
                        pending.push(Pending::OnDomain(domain));
                        pending.push(Pending::Read(depth + 1));
                        continue;
                    }
                    4 if version == 2 => {
                        let domain = ParameterDomain::from_bytes(
                            input.blob(work.control())?,
                            limits.sources.parameters.parameters,
                            work,
                        )?;
                        let region = ParameterRegion::from_bytes(
                            input.blob(work.control())?,
                            limits.sources.parameters.parameters,
                            work,
                        )?;
                        ObservationPredicate::region(&domain, &region, limits.numbers, work)?
                    }
                    _ => return Err(Error::InvalidEncoding.into()),
                }
            }
            operation => {
                let value = values.pop().ok_or(Error::InvalidEncoding)?;
                match operation {
                    Pending::Negate => value.negate(limits.numbers, work)?,
                    Pending::Binary(op) => values.pop().ok_or(Error::InvalidEncoding)?.apply(
                        op,
                        &value,
                        limits.numbers,
                        work,
                    )?,
                    Pending::OnDomain(domain) => value.on_domain(&domain, limits.numbers, work)?,
                    Pending::Read(_) => unreachable!("handled above"),
                }
            }
        };
        values.push(value);
    }
    let value = values.pop().ok_or(Error::InvalidEncoding)?;
    if !values.is_empty() {
        return Err(Error::InvalidEncoding.into());
    }
    Ok(value)
}
