//! The one binary frame codec for every log artifact: a family tag naming
//! the artifact and its format, then big-endian fields. Decoding is total:
//! any byte string yields a value or a [`FrameError`], never a panic, and a
//! decoded frame re-encodes to the same bytes.

use bumbledb::ChangeError;

/// What a frame holds, named by its family tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Entry,
    Command,
    Receipt,
    Head,
}

impl Kind {
    const fn tag(self) -> &'static [u8] {
        match self {
            Self::Entry => b"bdb.entry.v1\0",
            Self::Command => b"bdb.command.v1\0",
            Self::Receipt => b"bdb.receipt.v1\0",
            Self::Head => b"bdb.head.v1\0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The bytes do not start with the expected family tag.
    Family,
    /// An unknown variant tag.
    Tag(u8),
    Truncated,
    Trailing,
    /// A length or count larger than the rest of the frame.
    Length,
    /// Text that is not UTF-8.
    Text,
    /// A field whose type forbids its value (an empty list, a zero count).
    Value,
    /// Embedded change bytes the engine refused.
    Changes(ChangeError),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "log frame: {self:?}")
    }
}

impl std::error::Error for FrameError {}

impl From<ChangeError> for FrameError {
    fn from(error: ChangeError) -> Self {
        Self::Changes(error)
    }
}

pub(crate) struct Writer(Vec<u8>);

impl Writer {
    pub(crate) fn new(kind: Kind) -> Self {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(kind.tag());
        Self(bytes)
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.0.push(value);
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn array(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }

    /// A length-prefixed byte string.
    pub(crate) fn blob(&mut self, bytes: &[u8]) {
        self.len(bytes.len());
        self.0.extend_from_slice(bytes);
    }

    pub(crate) fn text(&mut self, text: &str) {
        self.blob(text.as_bytes());
    }

    /// A list length; every frame list is far below `u32::MAX` items.
    pub(crate) fn len(&mut self, len: usize) {
        self.u32(u32::try_from(len).expect("frame lengths fit u32"));
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.0
    }
}

pub(crate) struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8], kind: Kind) -> Result<Self, FrameError> {
        let rest = bytes.strip_prefix(kind.tag()).ok_or(FrameError::Family)?;
        Ok(Self { rest })
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], FrameError> {
        let (head, tail) = self
            .rest
            .split_at_checked(len)
            .ok_or(FrameError::Truncated)?;
        self.rest = tail;
        Ok(head)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], FrameError> {
        let (head, tail) = self
            .rest
            .split_first_chunk::<N>()
            .ok_or(FrameError::Truncated)?;
        self.rest = tail;
        Ok(*head)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, FrameError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u32(&mut self) -> Result<u32, FrameError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, FrameError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    /// A list length, refused when `len * min_item` exceeds the rest, so a
    /// hostile count never drives an allocation.
    pub(crate) fn len(&mut self, min_item: usize) -> Result<usize, FrameError> {
        let len = usize::try_from(self.u32()?).map_err(|_| FrameError::Length)?;
        if len.saturating_mul(min_item.max(1)) > self.rest.len() {
            return Err(FrameError::Length);
        }
        Ok(len)
    }

    pub(crate) fn blob(&mut self) -> Result<&'a [u8], FrameError> {
        let len = self.len(1)?;
        self.take(len)
    }

    pub(crate) fn text(&mut self) -> Result<&'a str, FrameError> {
        std::str::from_utf8(self.blob()?).map_err(|_| FrameError::Text)
    }

    pub(crate) fn finish(self) -> Result<(), FrameError> {
        if self.rest.is_empty() {
            Ok(())
        } else {
            Err(FrameError::Trailing)
        }
    }
}
