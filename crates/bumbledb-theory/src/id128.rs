//! Application-owned identity bytes, independent of database/history authority.

use core::fmt;
use core::str::FromStr;

/// An application-owned 128-bit value with no reserved bit patterns.
///
/// This is exactly sixteen bytes, not a database allocation or a history stamp.
/// All values, including all-zero bytes, are valid. Applications choose the
/// bytes once and preserve them across retries; this type promises no uniqueness.
/// Its canonical text is exactly 32 lowercase hexadecimal ASCII characters.
///
/// ```compile_fail
/// use bumbledb_theory::Id128;
/// let id = Id128([0; 16]); // Use the fixed-width public constructor.
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Id128([u8; 16]);

impl Id128 {
    /// Own an application's sixteen identifier bytes without reinterpretation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Return the original sixteen bytes, with no added history information.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }

    /// Borrow the immutable sixteen bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Decode canonical lowercase hexadecimal text, without aliases or prefixes.
    ///
    /// # Errors
    /// Returns [`Id128ParseError::InvalidHexLength`] unless the input is exactly
    /// 32 bytes, or [`Id128ParseError::InvalidHexDigit`] for anything outside
    /// ASCII `0`–`9` and `a`–`f`. Uppercase and hyphenated UUID text are not this
    /// canonical wire form; their already-decoded bytes can use [`Self::from_bytes`].
    pub fn from_hex(text: &str) -> Result<Self, Id128ParseError> {
        let input = text.as_bytes();
        if input.len() != 32 {
            return Err(Id128ParseError::InvalidHexLength {
                actual: input.len(),
            });
        }
        let mut bytes = [0; 16];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = 2 * index;
            *byte = (hex_digit(input[offset], offset)? << 4)
                | hex_digit(input[offset + 1], offset + 1)?;
        }
        Ok(Self(bytes))
    }
}

fn hex_digit(byte: u8, index: usize) -> Result<u8, Id128ParseError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(Id128ParseError::InvalidHexDigit { index }),
    }
}

impl From<[u8; 16]> for Id128 {
    fn from(bytes: [u8; 16]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<Id128> for [u8; 16] {
    fn from(value: Id128) -> Self {
        value.to_bytes()
    }
}

impl TryFrom<&[u8]> for Id128 {
    type Error = Id128ParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let payload = bytes
            .try_into()
            .map_err(|_| Id128ParseError::InvalidByteLength {
                actual: bytes.len(),
            })?;
        Ok(Self::from_bytes(payload))
    }
}

impl FromStr for Id128 {
    type Err = Id128ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_hex(text)
    }
}

impl fmt::Display for Id128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Id128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id128({self})")
    }
}

/// Failure to decode the fixed-width bytes or canonical text of an [`Id128`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id128ParseError {
    /// A binary identifier must contain exactly sixteen bytes.
    InvalidByteLength { actual: usize },
    /// Canonical hexadecimal text must contain exactly 32 bytes.
    InvalidHexLength { actual: usize },
    /// A byte at this zero-based offset is not a lowercase hexadecimal digit.
    InvalidHexDigit { index: usize },
}

impl fmt::Display for Id128ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidByteLength { actual } => {
                write!(f, "Id128 requires 16 bytes, got {actual}")
            }
            Self::InvalidHexLength { actual } => {
                write!(f, "Id128 requires 32 hexadecimal bytes, got {actual}")
            }
            Self::InvalidHexDigit { index } => {
                write!(
                    f,
                    "invalid lowercase Id128 hexadecimal digit at byte {index}"
                )
            }
        }
    }
}

impl std::error::Error for Id128ParseError {}
