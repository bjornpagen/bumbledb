//! BEVT v3: a canonical finite logical presentation, checked parameter binding,
//! and optional declared rational-function law. No nested source is trusted.
use super::measure::Cell;
use super::{ParameterDensityPiece, ParameterGuard, ParameterSourceLimits};
use crate::{
    BoolOp4, Capacity, Control, Error, Event, ExactArithmetic, ExactPolynomial,
    GuardedRationalFunction, Limits, ParameterDomain, ParameterRegion, Result, Space,
};

struct Writer {
    bytes: Vec<u8>,
    limit: usize,
}
impl Writer {
    fn put(&mut self, part: &[u8]) -> Result<()> {
        if part.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        self.bytes.try_reserve(part.len())?;
        self.bytes.extend_from_slice(part);
        Ok(())
    }
    fn number(&mut self, value: usize) -> Result<()> {
        self.put(&(value as u64).to_le_bytes())
    }
    fn blob(&mut self, part: &[u8]) -> Result<()> {
        self.number(part.len())?;
        self.put(part)
    }
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
    fn number(&mut self) -> Result<usize> {
        usize::try_from(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| Error::InvalidEncoding)?,
        ))
        .map_err(|_| Error::InvalidEncoding)
    }
    fn blob(&mut self) -> Result<&'a [u8]> {
        let n = self.number()?;
        self.take(n)
    }
    fn finish(&self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::InvalidEncoding)
        }
    }
    fn header(&mut self, expected: &[u8]) -> Result<()> {
        if self.take(expected.len())? == expected {
            Ok(())
        } else {
            Err(Error::InvalidEncoding)
        }
    }
}

pub(super) fn encode_context(
    domain: &ParameterDomain,
    guards: &[ParameterGuard],
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    let mut out = Writer {
        bytes: Vec::new(),
        limit: limits.parameters.bytes,
    };
    out.put(b"BEPX\x01")?;
    out.blob(&domain.to_bytes(limits.parameters, work)?)?;
    out.number(guards.len())?;
    for guard in guards {
        work.control().checkpoint()?;
        out.put(&[guard.coordinate])?;
        let region = guard.region.apply(
            BoolOp4::AND,
            domain.region(),
            limits.parameters.region,
            work,
        )?;
        out.blob(&region.to_bytes(limits.parameters, work)?)?;
    }
    Ok(out.bytes)
}

/// This is canonical *definition* data. Equivalent rational expressions on a
/// constrained domain need not be the same declared source definition.
pub(super) fn encode_function(
    value: &GuardedRationalFunction,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Vec<u8>> {
    value.restrict(value.ambient().region(), limits.parameters.region, work)?;
    let mut out = Writer {
        bytes: Vec::new(),
        limit: limits.laws.bytes,
    };
    out.put(b"BEGF\x01")?;
    out.blob(
        &value
            .numerator()
            .to_bytes(limits.parameters.region.polynomial, work)?,
    )?;
    out.blob(
        &value
            .denominator()
            .to_bytes(limits.parameters.region.polynomial, work)?,
    )?;
    out.blob(&value.defined_on().to_bytes(limits.parameters, work)?)?;
    Ok(out.bytes)
}
fn decode_function(
    bytes: &[u8],
    domain: &ParameterDomain,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<GuardedRationalFunction> {
    if bytes.len() > limits.laws.bytes {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    let mut input = Reader { bytes, offset: 0 };
    input.header(b"BEGF\x01")?;
    let numerator =
        ExactPolynomial::from_bytes(input.blob()?, limits.parameters.region.polynomial, work)?;
    let denominator =
        ExactPolynomial::from_bytes(input.blob()?, limits.parameters.region.polynomial, work)?;
    let defined = ParameterRegion::from_bytes(input.blob()?, limits.parameters, work)?;
    input.finish()?;
    let value = GuardedRationalFunction::new(
        domain.clone(),
        numerator,
        denominator,
        limits.parameters.region,
        work,
    )?
    .restrict(&defined, limits.parameters.region, work)?;
    if encode_function(&value, limits, work)? != bytes {
        return Err(Error::InvalidEncoding);
    }
    Ok(value)
}
pub(super) fn encode_law(
    space: &Space,
    cells: &[(Vec<u8>, Cell)],
    limits: ParameterSourceLimits,
    control: &dyn Control,
) -> Result<Vec<u8>> {
    let mut out = Writer {
        bytes: Vec::new(),
        limit: limits.laws.bytes,
    };
    out.put(b"BEPM\x01")?;
    out.number(cells.len())?;
    for (function, cell) in cells {
        control.checkpoint()?;
        out.blob(function)?;
        out.blob(&space.event(cell.root).structural_bytes(control)?)?;
    }
    Ok(out.bytes)
}

pub(crate) fn encode(event: &Event, control: &dyn Control) -> Result<Vec<u8>> {
    control.checkpoint()?;
    let space = event.space();
    let context = space.0.parameter.as_ref().ok_or(Error::MissingParameter)?;
    let mut out = Writer {
        bytes: Vec::new(),
        limit: usize::MAX,
    };
    out.put(b"BEVT\x03")?;
    out.blob(&event.structural_bytes(control)?)?;
    out.blob(&context.canonical)?;
    out.blob(
        context
            .law
            .as_ref()
            .map_or(&[], |law| law.canonical.as_ref()),
    )?;
    control.checkpoint()?;
    Ok(out.bytes)
}

fn structural(
    bytes: &[u8],
    order: Option<&[u8]>,
    limits: Limits,
    control: &dyn Control,
) -> Result<Event> {
    if bytes.get(..5) != Some(b"BEVT\x01") {
        return Err(Error::InvalidEncoding);
    }
    Event::from_bytes_with_order(bytes, order, limits, control)
}

pub(crate) fn decode(
    bytes: &[u8],
    order: Option<&[u8]>,
    events: Limits,
    limits: ParameterSourceLimits,
    work: &mut ExactArithmetic<'_>,
) -> Result<Event> {
    let control = work.control();
    control.checkpoint()?;
    let mut input = Reader { bytes, offset: 0 };
    input.header(b"BEVT\x03")?;
    let raw_event = input.blob()?;
    let raw_context = input.blob()?;
    let raw_law = input.blob()?;
    input.finish()?;
    if raw_context.len() > limits.parameters.bytes || raw_law.len() > limits.laws.bytes {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    let event = structural(raw_event, order, events, control)?;
    let raw_space = event.space();
    let mut binding = Reader {
        bytes: raw_context,
        offset: 0,
    };
    binding.header(b"BEPX\x01")?;
    let domain = ParameterDomain::from_bytes(binding.blob()?, limits.parameters, work)?;
    let count = binding.number()?;
    if count > usize::from(raw_space.dimensions()) {
        return Err(Error::InvalidOrder);
    }
    let mut guards = Vec::new();
    guards.try_reserve_exact(count)?;
    for _ in 0..count {
        control.checkpoint()?;
        guards.push(ParameterGuard {
            coordinate: binding.take(1)?[0],
            region: ParameterRegion::from_bytes(binding.blob()?, limits.parameters, work)?,
        });
    }
    binding.finish()?;
    let space = raw_space.with_parameters(domain, &guards, limits, work)?;
    if space.parameter_bytes() != Some(raw_context)
        || space.full().structural_bytes(control)? != raw_space.full().structural_bytes(control)?
    {
        return Err(Error::InvalidEncoding);
    }
    if raw_law.is_empty() {
        return event.in_space(&space, control);
    }
    let mut law = Reader {
        bytes: raw_law,
        offset: 0,
    };
    law.header(b"BEPM\x01")?;
    let count = law.number()?;
    if count > limits.laws.cells {
        return Err(Error::Capacity(Capacity::LawCells));
    }
    if count > raw_law.len().saturating_sub(13) / 16 {
        return Err(Error::InvalidEncoding);
    }
    let mut pieces = Vec::new();
    pieces.try_reserve_exact(count)?;
    for _ in 0..count {
        control.checkpoint()?;
        let density = decode_function(
            law.blob()?,
            space.parameter_domain().ok_or(Error::MissingParameter)?,
            limits,
            work,
        )?;
        let region = structural(law.blob()?, order, events, control)?
            .align_to(&raw_space, control)?
            .in_space(&space, control)?;
        pieces.push(ParameterDensityPiece { region, density });
    }
    law.finish()?;
    let measured = space.with_parameter_density(&pieces, limits, work)?;
    if measured.measurement_bytes() != Some(raw_law) {
        return Err(Error::InvalidEncoding);
    }
    event.in_space(&measured, control)
}
