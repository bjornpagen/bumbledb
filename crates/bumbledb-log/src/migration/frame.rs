//! Bounded framing for the one migration format family (C11):
//! `family || layout(u16) || kind(u8) || fields`, big-endian, fixed widths,
//! length-prefixed spans, no trailing bytes — the same discipline as the
//! history frames, in the migration family so plan bytes can never alias a
//! command, decision or control frame. Framing establishes grammar only;
//! digests, admission and authority happen above it. Physical bytes remain
//! provisional until the F3 format freeze (C12).

use crate::history::FrameError;

/// The one migration format family. Recognizing the integer layout alone is
/// forbidden; every reader checks the full family string first.
pub const FAMILY: &[u8] = b"bumbledb.migration.v1\0";
pub const LAYOUT: u16 = 1;

/// Frame kinds within the family.
pub const KIND_PLAN: u8 = 1;
pub const KIND_MANIFEST_BASE: u8 = 2;
pub const KIND_MANIFEST_ENTRY: u8 = 3;
pub const KIND_PLAN_SET: u8 = 4;
#[allow(
    dead_code,
    reason = "reserved kind number in the one migration frame family; \
              renumbering a frozen grammar is forbidden (C11/C12)"
)]
pub const KIND_STATE: u8 = 5;
pub const KIND_APPLIED: u8 = 6;
pub const KIND_BASELINE: u8 = 7;
#[allow(
    dead_code,
    reason = "reserved kind number in the one migration frame family; \
              renumbering a frozen grammar is forbidden (C11/C12)"
)]
pub const KIND_TOMBSTONE: u8 = 8;

/// Hash domains (blake3 `derive_key` contexts). One role, one domain; no
/// digest is ever a truncation or re-tagging of another.
pub const PLAN_DIGEST_DOMAIN: &str = "bumbledb.migration.v1/plan-digest";
pub const PREFIX_DIGEST_DOMAIN: &str = "bumbledb.migration.v1/prefix-digest";
pub const PLAN_SET_DIGEST_DOMAIN: &str = "bumbledb.migration.v1/plan-set-digest";
pub const STATE_DIGEST_DOMAIN: &str = "bumbledb.migration.v1/state-digest";
pub const SYSTEM_DIGEST_DOMAIN: &str = "bumbledb.migration.v1/system-digest";

pub(crate) fn check_limit(length: usize, cap: usize) -> Result<(), FrameError> {
    if length > cap {
        Err(FrameError::LimitExceeded)
    } else {
        Ok(())
    }
}

pub(crate) fn begin(kind: u8, cap: usize) -> Result<Frame, FrameError> {
    check_limit(FAMILY.len() + 3, cap)?;
    let mut out = Vec::new();
    out.try_reserve(FAMILY.len() + 3)
        .map_err(|_| FrameError::Allocation)?;
    out.extend_from_slice(FAMILY);
    out.extend_from_slice(&LAYOUT.to_be_bytes());
    out.push(kind);
    Ok(Frame { out, cap })
}

/// A growing frame with its allocation cap enforced on every append. The
/// callers of this writer build variably-shaped plans; precomputing exact
/// lengths for every nested arm would triple the grammar, so the cap check
/// rides each append instead.
pub(crate) struct Frame {
    out: Vec<u8>,
    cap: usize,
}

impl Frame {
    pub(crate) fn bytes(&mut self, bytes: &[u8]) -> Result<(), FrameError> {
        let len = self
            .out
            .len()
            .checked_add(bytes.len())
            .ok_or(FrameError::LengthOverflow)?;
        check_limit(len, self.cap)?;
        self.out
            .try_reserve(bytes.len())
            .map_err(|_| FrameError::Allocation)?;
        self.out.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn tag(&mut self, tag: u8) -> Result<(), FrameError> {
        self.bytes(&[tag])
    }

    pub(crate) fn u64(&mut self, value: u64) -> Result<(), FrameError> {
        self.bytes(&value.to_be_bytes())
    }

    pub(crate) fn span(&mut self, bytes: &[u8]) -> Result<(), FrameError> {
        self.u64(u64::try_from(bytes.len()).map_err(|_| FrameError::LengthOverflow)?)?;
        self.bytes(bytes)
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.out
    }
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn begin(bytes: &'a [u8], kind: u8, cap: usize) -> Result<Self, FrameError> {
        check_limit(bytes.len(), cap)?;
        let mut input = Self { bytes, at: 0 };
        if input.take(FAMILY.len())? != FAMILY {
            return Err(FrameError::Family);
        }
        let layout = u16::from_be_bytes(input.array()?);
        if layout != LAYOUT {
            return Err(FrameError::Layout { got: layout });
        }
        let (_, got) = input.tag()?;
        if got != kind {
            return Err(FrameError::Kind { got });
        }
        Ok(input)
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], FrameError> {
        let end = self.at.checked_add(len).ok_or(FrameError::LengthOverflow)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(FrameError::Truncated { at: self.at })?;
        self.at = end;
        Ok(bytes)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], FrameError> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    pub(crate) fn u64(&mut self) -> Result<u64, FrameError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub(crate) fn tag(&mut self) -> Result<(usize, u8), FrameError> {
        let at = self.at;
        Ok((at, self.array::<1>()?[0]))
    }

    pub(crate) fn span(&mut self, cap: usize) -> Result<&'a [u8], FrameError> {
        let len = usize::try_from(self.u64()?).map_err(|_| FrameError::LengthOverflow)?;
        check_limit(len, cap)?;
        self.take(len)
    }

    /// A bounded count field: refuses counts that could not possibly fit the
    /// remaining bytes, before any allocation happens.
    pub(crate) fn count(&mut self, min_entry_bytes: usize) -> Result<usize, FrameError> {
        let count = usize::try_from(self.u64()?).map_err(|_| FrameError::LengthOverflow)?;
        let remaining = self.bytes.len() - self.at;
        if min_entry_bytes > 0 && count > remaining / min_entry_bytes {
            return Err(FrameError::InvalidCount);
        }
        Ok(count)
    }

    pub(crate) fn end(self) -> Result<(), FrameError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(FrameError::TrailingBytes { at: self.at })
        }
    }
}

/// The one keyed digest constructor for the migration domains above.
#[must_use]
pub fn keyed_digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    blake3::derive_key(domain, bytes)
}
