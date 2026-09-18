//! BEVT v2 is a canonical v1 structural Event plus a BELW v1 finite law.
//! Nested measured Events are forbidden. Decoding reestablishes every source
//! invariant and compares the reconstructed canonical law before publication.
use super::{Cell, DensityPiece, LawLimits};
use crate::{
    ArithmeticLimits, Capacity, Control, Error, Event, ExactArithmetic, ExactRational, Limits,
    Result, Space,
};

fn append(bytes: &mut Vec<u8>, value: &[u8], limit: usize) -> Result<()> {
    let length = bytes
        .len()
        .checked_add(8)
        .and_then(|n| n.checked_add(value.len()))
        .ok_or(Error::Capacity(Capacity::DescriptorBytes))?;
    if length > limit {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    bytes.try_reserve_exact(length - bytes.len())?;
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

pub(super) fn encode_law(
    space: &Space,
    cells: &[(Vec<u8>, Cell)],
    limits: LawLimits,
    control: &dyn Control,
) -> Result<Vec<u8>> {
    if limits.bytes < 13 {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(13)?;
    bytes.extend_from_slice(b"BELW\x01");
    bytes.extend_from_slice(&(cells.len() as u64).to_le_bytes());
    for (density, cell) in cells {
        control.checkpoint()?;
        append(&mut bytes, density, limits.bytes)?;
        let region = space.event(cell.root).structural_bytes(control)?;
        append(&mut bytes, &region, limits.bytes)?;
    }
    Ok(bytes)
}

pub(crate) fn encode(event: &[u8], law: &[u8], control: &dyn Control) -> Result<Vec<u8>> {
    control.checkpoint()?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(5)?;
    bytes.extend_from_slice(b"BEVT\x02");
    append(&mut bytes, event, usize::MAX)?;
    append(&mut bytes, law, usize::MAX)?;
    control.checkpoint()?;
    Ok(bytes)
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn count(&mut self) -> Result<usize> {
        let end = self.offset.checked_add(8).ok_or(Error::InvalidEncoding)?;
        let raw = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::InvalidEncoding)?;
        self.offset = end;
        usize::try_from(u64::from_le_bytes(
            raw.try_into().map_err(|_| Error::InvalidEncoding)?,
        ))
        .map_err(|_| Error::InvalidEncoding)
    }
    fn blob(&mut self) -> Result<&'a [u8]> {
        let count = self.count()?;
        let end = self
            .offset
            .checked_add(count)
            .ok_or(Error::InvalidEncoding)?;
        let part = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::InvalidEncoding)?;
        self.offset = end;
        Ok(part)
    }
    fn finish(&self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::InvalidEncoding)
        }
    }
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
    limits: LawLimits,
    arithmetic: ArithmeticLimits,
    control: &dyn Control,
) -> Result<Event> {
    control.checkpoint()?;
    let mut input = Reader { bytes, offset: 5 };
    let raw_event = input.blob()?;
    let raw_law = input.blob()?;
    input.finish()?;
    if raw_law.len() > limits.bytes {
        return Err(Error::Capacity(Capacity::DescriptorBytes));
    }
    if raw_law.get(..5) != Some(b"BELW\x01") {
        return Err(Error::InvalidEncoding);
    }
    let mut law = Reader {
        bytes: raw_law,
        offset: 5,
    };
    let count = law.count()?;
    if count > limits.cells {
        return Err(Error::Capacity(Capacity::LawCells));
    }
    // Each entry needs two lengths, a nonempty BERA and a nonempty BEVT.
    if count > raw_law.len().saturating_sub(13) / 16 {
        return Err(Error::InvalidEncoding);
    }
    let event = structural(raw_event, order, events, control)?;
    let space = event.space();
    let mut work = ExactArithmetic::new(arithmetic, control);
    let mut pieces = Vec::new();
    pieces.try_reserve_exact(count)?;
    for _ in 0..count {
        control.checkpoint()?;
        let density = ExactRational::from_bytes(law.blob()?, &mut work)?;
        let region = structural(law.blob()?, order, events, control)?.align_to(&space, control)?;
        pieces.push(DensityPiece { region, density });
    }
    law.finish()?;
    let measured = space.with_density(&pieces, limits, &mut work)?;
    if measured.measurement_bytes() != Some(raw_law) {
        return Err(Error::InvalidEncoding);
    }
    event.in_space(&measured, control)
}
