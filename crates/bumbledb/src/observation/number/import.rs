//! BENO v1 replays an exact numerical derivation. No numerical answer or truth
//! claim is trusted from the wire. Full observation leaves retain indexed
//! payoffs, including empty cells and possible zero-mass outcomes.
#![allow(clippy::large_types_passed_by_value)]

use super::{
    ObservationComponent, ObservationNumber, ObservationNumberExpr, ObservationNumberLimits,
};
use crate::event::{
    AdmittedFamilyDescriptor, AdmittedSourceDescriptor, Capacity, Control, Error, EventPartition,
    ExactArithmetic, ExactRational, FamilyFunction, FiniteFunction, FunctionPatch, NumberOp,
    ParameterDomain, SourceDescriptor, SourceDescriptorLimits,
};
use crate::{Event, ExpectationAnswer, ExpectationPayoff, ProbabilityAnswer, Result};
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// Whole-envelope bytes/items, per-source bounds, and numerical expression
/// bounds. These are admission limits, not a total retained-memory quota.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObservationNumberCodecLimits {
    pub sources: SourceDescriptorLimits,
    pub numbers: ObservationNumberLimits,
}

#[derive(Debug)]
struct Import {
    value: ObservationNumber,
    bytes: Box<[u8]>,
}

/// Owned, replay-checked numerical query data. Identity is the complete encoded
/// derivation, independent of Arc sharing and allocation order. This is neither
/// numerical equivalence nor stochastic independence.
#[derive(Debug, Clone)]
pub struct ObservationNumberImport(Arc<Import>);

impl PartialEq for ObservationNumberImport {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.bytes == other.0.bytes
    }
}
impl Eq for ObservationNumberImport {}
impl std::hash::Hash for ObservationNumberImport {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.bytes.hash(state);
    }
}

impl ObservationNumberImport {
    /// The query executor has already checked every node under its shared
    /// budget. Capture identity without contracting its observations again.
    pub(crate) fn checked(
        value: ObservationNumber,
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        let bytes = encode(&value, limits, work)?.into_boxed_slice();
        Ok(Self(Arc::new(Import { value, bytes })))
    }
    /// Capture through the same checked replay as untrusted input. Original
    /// source identities survive; reconstructed owners have fresh local keys.
    /// # Errors
    /// Encoding, source/numerical admission, bounds or cancellation.
    pub fn capture(
        value: &ObservationNumber,
        limits: ObservationNumberCodecLimits,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<Self> {
        Self::from_bytes(&encode(value, limits, work)?, limits, work)
    }

    /// Replay every leaf and operation with one shared arithmetic counter.
    /// Reject trailing bytes, unsupported tags and noncanonical presentations.
    /// # Errors
    /// Malformed data, invalid source/coverage/domain, bounds or cancellation.
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
        if input.take(4)? != b"BENO" {
            return Err(Error::InvalidEncoding.into());
        }
        let version = input.byte()?;
        if version != 1 {
            return Err(Error::UnsupportedVersion(version).into());
        }
        let value = input.node(1, work)?;
        if input.offset != bytes.len() {
            return Err(Error::InvalidEncoding.into());
        }
        // In particular this rejects scalar partition cells clipped by their
        // parent, noncanonical nested descriptors and presentation aliases.
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
    pub fn value(&self) -> &ObservationNumber {
        &self.0.value
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
}

pub(super) struct Budget {
    limits: ObservationNumberCodecLimits,
    items: usize,
    nodes: usize,
}
impl Budget {
    pub(super) fn new(limits: ObservationNumberCodecLimits) -> Self {
        Self {
            limits,
            items: 0,
            nodes: 0,
        }
    }
    fn items(&mut self, count: usize, control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        self.items = self
            .items
            .checked_add(count)
            .filter(|n| *n <= self.limits.sources.descriptors.items)
            .ok_or(Error::Capacity(Capacity::DescriptorItems))?;
        Ok(())
    }
    pub(super) fn node(&mut self, depth: usize, control: &dyn Control) -> Result<()> {
        self.items(1, control)?;
        self.nodes = self
            .nodes
            .checked_add(1)
            .filter(|n| *n <= self.limits.numbers.nodes)
            .ok_or(Error::Capacity(Capacity::ProgramNodes))?;
        if depth > self.limits.numbers.depth.min(256) {
            return Err(Error::Capacity(Capacity::ProgramNodes).into());
        }
        Ok(())
    }
}

pub(super) struct Writer {
    pub(super) bytes: Vec<u8>,
    pub(super) budget: Budget,
}
fn encode(
    value: &ObservationNumber,
    limits: ObservationNumberCodecLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    let mut output = Writer {
        bytes: Vec::new(),
        budget: Budget::new(limits),
    };
    output.put(b"BENO\x01", work.control())?;
    output.node(value, 1, work)?;
    Ok(output.bytes)
}
impl Writer {
    pub(super) fn put(&mut self, bytes: &[u8], control: &dyn Control) -> Result<()> {
        control.checkpoint()?;
        if bytes.len()
            > self
                .budget
                .limits
                .sources
                .descriptors
                .bytes
                .saturating_sub(self.bytes.len())
        {
            return Err(Error::Capacity(Capacity::DescriptorBytes).into());
        }
        self.bytes.try_reserve(bytes.len()).map_err(Error::from)?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn count(&mut self, count: usize, control: &dyn Control) -> Result<()> {
        let count = u32::try_from(count).map_err(|_| Error::Capacity(Capacity::DescriptorBytes))?;
        self.put(&count.to_le_bytes(), control)
    }
    pub(super) fn blob(&mut self, bytes: &[u8], control: &dyn Control) -> Result<()> {
        self.budget.items(1, control)?;
        self.count(bytes.len(), control)?;
        self.put(bytes, control)
    }
    fn event(&mut self, event: &Event, work: &mut ExactArithmetic<'_>) -> Result<()> {
        self.blob(&event.to_bytes(work.control())?, work.control())
    }
    fn function(
        &mut self,
        function: &AdmittedSourceDescriptor,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        let limits = self.budget.limits.sources;
        let bytes =
            SourceDescriptor::capture(function, limits, work)?.to_bytes(limits, work.control())?;
        self.blob(&bytes, work.control())
    }
    pub(super) fn node(
        &mut self,
        value: &ObservationNumber,
        depth: usize,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<()> {
        let mut pending = Vec::new();
        pending.try_reserve(1).map_err(Error::from)?;
        pending.push((value, depth));
        while let Some((value, depth)) = pending.pop() {
            self.budget.node(depth, work.control())?;
            pending.try_reserve(2).map_err(Error::from)?;
            match value.expression() {
                ObservationNumberExpr::Literal(value) => {
                    self.put(&[0], work.control())?;
                    self.blob(&value.to_bytes(work)?, work.control())?;
                }
                ObservationNumberExpr::Probability {
                    observation,
                    component,
                } => {
                    self.put(&[1, component_tag(*component)], work.control())?;
                    self.event(observation.event(), work)?;
                    self.event(observation.given(), work)?;
                }
                ObservationNumberExpr::Expectation {
                    observation,
                    component,
                } => {
                    self.put(&[2, component_tag(*component)], work.control())?;
                    self.payoff(observation.payoff(), work)?;
                }
                ObservationNumberExpr::Binary { op, left, right } => {
                    self.put(&[3, operation_tag(*op)], work.control())?;
                    pending.push((right, depth + 1));
                    pending.push((left, depth + 1));
                }
                ObservationNumberExpr::Negate(value) => {
                    self.put(&[4], work.control())?;
                    pending.push((value, depth + 1));
                }
                ObservationNumberExpr::Abs(value) => {
                    self.put(&[5], work.control())?;
                    pending.push((value, depth + 1));
                }
                ObservationNumberExpr::Pow { value, exponent } => {
                    self.put(&[6], work.control())?;
                    self.put(&exponent.to_le_bytes(), work.control())?;
                    pending.push((value, depth + 1));
                }
                ObservationNumberExpr::OnDomain { value, domain } => {
                    self.put(&[7], work.control())?;
                    let bytes =
                        domain.to_bytes(self.budget.limits.sources.parameters.parameters, work)?;
                    self.blob(&bytes, work.control())?;
                    pending.push((value, depth + 1));
                }
            }
        }
        Ok(())
    }
    fn payoff(&mut self, payoff: &ExpectationPayoff, work: &mut ExactArithmetic<'_>) -> Result<()> {
        self.event(payoff.given(), work)?;
        let (tag, count) = match payoff {
            ExpectationPayoff::Scalar { partition, .. } => (0, partition.cells().len()),
            ExpectationPayoff::Finite(cover) => (1, cover.patches().len()),
            ExpectationPayoff::Family(cover) => (2, cover.patches().len()),
        };
        self.budget.items(count, work.control())?;
        self.put(&[tag], work.control())?;
        self.count(count, work.control())?;
        match payoff {
            ExpectationPayoff::Scalar { partition, values } => {
                if partition.cells().len() != values.len() {
                    return Err(Error::PartitionArity.into());
                }
                for (region, value) in partition.cells().iter().zip(values) {
                    self.event(region, work)?;
                    self.blob(&value.to_bytes(work)?, work.control())?;
                }
            }
            ExpectationPayoff::Finite(cover) => {
                for patch in cover.patches() {
                    self.event(&patch.region, work)?;
                    self.function(
                        &AdmittedSourceDescriptor::Function(patch.function.clone()),
                        work,
                    )?;
                }
            }
            ExpectationPayoff::Family(cover) => {
                for patch in cover.patches() {
                    self.event(&patch.region, work)?;
                    self.function(
                        &AdmittedSourceDescriptor::Family(Box::new(
                            AdmittedFamilyDescriptor::Function(patch.function.clone()),
                        )),
                        work,
                    )?;
                }
            }
        }
        Ok(())
    }
}

pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) offset: usize,
    pub(super) budget: Budget,
}
impl<'a> Reader<'a> {
    pub(super) fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(Error::InvalidEncoding)?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::InvalidEncoding)?;
        self.offset = end;
        Ok(bytes)
    }
    pub(super) fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn word(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }
    fn count(&mut self) -> Result<usize> {
        usize::try_from(self.word()?).map_err(|_| Error::InvalidEncoding.into())
    }
    pub(super) fn blob(&mut self, control: &dyn Control) -> Result<&'a [u8]> {
        self.budget.items(1, control)?;
        let count = self.count()?;
        self.take(count)
    }
    fn event(&mut self, work: &mut ExactArithmetic<'_>) -> Result<Event> {
        let limits = self.budget.limits.sources;
        let bytes = self.blob(work.control())?;
        Ok(if bytes.get(..5) == Some(b"BEVT\x03") {
            Event::from_bytes_with_parameter_limits(
                bytes,
                None,
                limits.descriptors.events,
                limits.parameters,
                work,
            )?
        } else {
            Event::from_bytes_with_arithmetic(
                bytes,
                None,
                limits.descriptors.events,
                limits.laws,
                work,
            )?
        })
    }
    // Explicit work/value stacks: hostile depth never consumes the host stack.
    pub(super) fn node(
        &mut self,
        depth: usize,
        work: &mut ExactArithmetic<'_>,
    ) -> Result<ObservationNumber> {
        enum Pending {
            Read(usize),
            Binary(NumberOp),
            Negate,
            Abs,
            Pow(u32),
            OnDomain(ParameterDomain),
        }
        let limits = self.budget.limits;
        let mut pending = Vec::new();
        let mut values: Vec<ObservationNumber> = Vec::new();
        pending.try_reserve(1).map_err(Error::from)?;
        pending.push(Pending::Read(depth));
        while let Some(task) = pending.pop() {
            work.control().checkpoint()?;
            pending.try_reserve(3).map_err(Error::from)?;
            values.try_reserve(1).map_err(Error::from)?;
            let value =
                match task {
                    Pending::Read(depth) => {
                        self.budget.node(depth, work.control())?;
                        match self.byte()? {
                            tag @ 0..=2 => self.leaf(tag, work)?,
                            3 => {
                                pending.push(Pending::Binary(operation(self.byte()?)?));
                                pending.push(Pending::Read(depth + 1));
                                pending.push(Pending::Read(depth + 1));
                                continue;
                            }
                            tag @ (4 | 5) => {
                                pending.push(if tag == 4 {
                                    Pending::Negate
                                } else {
                                    Pending::Abs
                                });
                                pending.push(Pending::Read(depth + 1));
                                continue;
                            }
                            6 => {
                                pending.push(Pending::Pow(self.word()?));
                                pending.push(Pending::Read(depth + 1));
                                continue;
                            }
                            7 => {
                                let domain = ParameterDomain::from_bytes(
                                    self.blob(work.control())?,
                                    limits.sources.parameters.parameters,
                                    work,
                                )?;
                                pending.push(Pending::OnDomain(domain));
                                pending.push(Pending::Read(depth + 1));
                                continue;
                            }
                            _ => return Err(Error::InvalidEncoding.into()),
                        }
                    }
                    operation => {
                        let value = values.pop().ok_or(Error::InvalidEncoding)?;
                        match operation {
                            Pending::Binary(op) => values
                                .pop()
                                .ok_or(Error::InvalidEncoding)?
                                .apply(op, &value, limits.numbers, work)?,
                            Pending::Negate => value.negate(limits.numbers, work)?,
                            Pending::Abs => value.abs(limits.numbers, work)?,
                            Pending::Pow(exponent) => value.pow(exponent, limits.numbers, work)?,
                            Pending::OnDomain(domain) => {
                                value.on_domain(&domain, limits.numbers, work)?
                            }
                            Pending::Read(_) => unreachable!("handled above"),
                        }
                    }
                };
            values.push(value);
        }
        if values.len() != 1 {
            return Err(Error::InvalidEncoding.into());
        }
        Ok(values.pop().expect("one completed root"))
    }
    fn leaf(&mut self, tag: u8, work: &mut ExactArithmetic<'_>) -> Result<ObservationNumber> {
        let limits = self.budget.limits;
        match tag {
            0 => {
                let value = ExactRational::from_bytes(self.blob(work.control())?, work)?;
                ObservationNumber::literal(value, limits.numbers, work)
            }
            1 => {
                let component = component(self.byte()?)?;
                let event = self.event(work)?;
                let given = self.event(work)?.align_to(&event.space(), work.control())?;
                let observation =
                    ProbabilityAnswer::with_limits(event, given, limits.sources.parameters, work)?;
                ObservationNumber::probability(observation, component, limits.numbers, work)
            }
            2 => {
                let component = component(self.byte()?)?;
                let input = self.payoff(work)?;
                let observation = ExpectationAnswer::with_limits(input, &limits.sources, work)?;
                ObservationNumber::expectation(observation, component, limits.numbers, work)
            }
            _ => unreachable!("node dispatch checks leaf tags"),
        }
    }
    fn payoff(&mut self, work: &mut ExactArithmetic<'_>) -> Result<ExpectationPayoff> {
        let limits = self.budget.limits.sources;
        let given = self.event(work)?;
        let tag = self.byte()?;
        if tag > 2 {
            return Err(Error::InvalidEncoding.into());
        }
        let count = self.count()?;
        self.budget.items(count, work.control())?;
        let (bound, capacity) = match tag {
            0 => (limits.partitions.cells, Capacity::PartitionCells),
            1 => (limits.functions.cells, Capacity::FunctionCells),
            _ => (limits.parameters.functions.cells, Capacity::FunctionCells),
        };
        if count > bound {
            return Err(Error::Capacity(capacity).into());
        }
        match tag {
            0 => {
                let mut cells = Vec::new();
                let mut values = Vec::new();
                cells.try_reserve_exact(count).map_err(Error::from)?;
                values.try_reserve_exact(count).map_err(Error::from)?;
                for _ in 0..count {
                    cells.push(self.event(work)?);
                    values.push(ExactRational::from_bytes(self.blob(work.control())?, work)?);
                }
                Ok(ExpectationPayoff::Scalar {
                    partition: EventPartition::on(
                        &given,
                        &cells,
                        limits.partitions,
                        work.control(),
                    )?,
                    values,
                })
            }
            1 => {
                let mut patches = Vec::new();
                patches.try_reserve_exact(count).map_err(Error::from)?;
                for _ in 0..count {
                    let region = self.event(work)?;
                    let function =
                        SourceDescriptor::import(self.blob(work.control())?, limits, work)?;
                    let AdmittedSourceDescriptor::Function(function) = function else {
                        return Err(Error::InvalidEncoding.into());
                    };
                    patches.push(FunctionPatch { region, function });
                }
                Ok(ExpectationPayoff::Finite(FiniteFunction::glue(
                    &given,
                    &patches,
                    limits.functions,
                    work,
                )?))
            }
            _ => {
                let mut patches = Vec::new();
                patches.try_reserve_exact(count).map_err(Error::from)?;
                for _ in 0..count {
                    let region = self.event(work)?;
                    let function =
                        SourceDescriptor::import(self.blob(work.control())?, limits, work)?;
                    let AdmittedSourceDescriptor::Family(function) = function else {
                        return Err(Error::InvalidEncoding.into());
                    };
                    let AdmittedFamilyDescriptor::Function(function) = *function else {
                        return Err(Error::InvalidEncoding.into());
                    };
                    patches.push(FunctionPatch { region, function });
                }
                Ok(ExpectationPayoff::Family(FamilyFunction::glue(
                    &given,
                    &patches,
                    limits.parameters,
                    work,
                )?))
            }
        }
    }
}

fn component_tag(value: ObservationComponent) -> u8 {
    match value {
        ObservationComponent::Value => 0,
        ObservationComponent::Numerator => 1,
        ObservationComponent::EvidenceMass => 2,
    }
}
fn component(tag: u8) -> Result<ObservationComponent> {
    match tag {
        0 => Ok(ObservationComponent::Value),
        1 => Ok(ObservationComponent::Numerator),
        2 => Ok(ObservationComponent::EvidenceMass),
        _ => Err(Error::InvalidEncoding.into()),
    }
}
fn operation_tag(value: NumberOp) -> u8 {
    match value {
        NumberOp::Add => 0,
        NumberOp::Subtract => 1,
        NumberOp::Multiply => 2,
        NumberOp::Divide => 3,
        NumberOp::Min => 4,
        NumberOp::Max => 5,
    }
}
fn operation(tag: u8) -> Result<NumberOp> {
    match tag {
        0 => Ok(NumberOp::Add),
        1 => Ok(NumberOp::Subtract),
        2 => Ok(NumberOp::Multiply),
        3 => Ok(NumberOp::Divide),
        4 => Ok(NumberOp::Min),
        5 => Ok(NumberOp::Max),
        _ => Err(Error::InvalidEncoding.into()),
    }
}
