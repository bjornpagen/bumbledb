//! Borrowed sequential decode of one stored canonical row.
//!
//! Canonical rows hold text inline, so a typed fact's `&str` and `&[u8]`
//! fields borrow the stored bytes (a snapshot's mapped pages or a
//! transaction's pending row) without a copy. Fields are read in declaration
//! order, the order `schema!`-generated `Fact::decode` consumes them. Every
//! step bounds-checks and refuses malformed bytes as corruption.

use crate::error::{CorruptionError, Error, Result};
use bumbledb_theory::{F64, Interval, Uuid};

use crate::canonical::tag;

/// Sequential field reader over one canonical row's bytes.
#[derive(Debug, Clone, Copy)]
pub struct RowReader<'a> {
    rest: &'a [u8],
    remaining: u16,
}

fn malformed(what: &'static str) -> Error {
    Error::Corruption(CorruptionError::MalformedValue(what))
}

impl<'a> RowReader<'a> {
    /// Begin reading one canonical row.
    /// # Errors
    /// Corruption when the arity header is missing.
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        let (header, rest) = bytes
            .split_first_chunk::<2>()
            .ok_or(malformed("canonical row arity header"))?;
        let remaining = u16::from_be_bytes(*header);
        Ok(Self { rest, remaining })
    }

    /// Fields not yet consumed.
    #[must_use]
    pub fn remaining(&self) -> u16 {
        self.remaining
    }

    fn take(&mut self, len: usize, what: &'static str) -> Result<&'a [u8]> {
        let (head, rest) = self.rest.split_at_checked(len).ok_or(malformed(what))?;
        self.rest = rest;
        Ok(head)
    }

    fn word<const N: usize>(&mut self, what: &'static str) -> Result<[u8; N]> {
        Ok(self.take(N, what)?.try_into().expect("split width"))
    }

    fn expect_tag(&mut self, expected: u8, what: &'static str) -> Result<()> {
        if self.remaining == 0 {
            return Err(malformed("canonical row read past its arity"));
        }
        self.remaining -= 1;
        let [found] = self.word::<1>(what)?;
        if found == expected {
            Ok(())
        } else {
            Err(malformed(what))
        }
    }

    /// # Errors
    /// Corruption on a wrong tag or malformed payload.
    pub fn next_bool(&mut self) -> Result<bool> {
        self.expect_tag(tag::BOOL, "canonical bool field")?;
        match self.word::<1>("canonical bool payload")? {
            [0] => Ok(false),
            [1] => Ok(true),
            _ => Err(malformed("canonical bool payload")),
        }
    }

    /// # Errors
    /// Corruption on a wrong tag or malformed payload.
    pub fn next_u64(&mut self) -> Result<u64> {
        self.expect_tag(tag::U64, "canonical u64 field")?;
        Ok(u64::from_be_bytes(self.word("canonical u64 payload")?))
    }

    /// # Errors
    /// Corruption on a wrong tag or malformed payload.
    pub fn next_i64(&mut self) -> Result<i64> {
        self.expect_tag(tag::I64, "canonical i64 field")?;
        Ok(i64::from_be_bytes(self.word("canonical i64 payload")?))
    }

    /// # Errors
    /// Corruption on a wrong tag, malformed payload, or noncanonical bits.
    pub fn next_f64(&mut self) -> Result<F64> {
        self.expect_tag(tag::F64, "canonical f64 field")?;
        F64::from_canonical_be_bytes(self.word("canonical f64 payload")?)
            .map_err(|_| malformed("canonical f64 payload"))
    }

    /// # Errors
    /// Corruption on a wrong tag or malformed payload.
    pub fn next_uuid(&mut self) -> Result<Uuid> {
        self.expect_tag(tag::UUID, "canonical uuid field")?;
        Ok(Uuid::from_bytes(self.word("canonical uuid payload")?))
    }

    /// # Errors
    /// Corruption on a wrong tag, malformed length, or non-UTF-8 bytes.
    pub fn next_str(&mut self) -> Result<&'a str> {
        self.expect_tag(tag::STRING, "canonical text field")?;
        let blob = self.blob("canonical text payload")?;
        std::str::from_utf8(blob).map_err(|_| malformed("canonical text payload"))
    }

    /// # Errors
    /// Corruption on a wrong tag or malformed length.
    pub fn next_bytes(&mut self) -> Result<&'a [u8]> {
        self.expect_tag(tag::FIXED_BYTES, "canonical bytes field")?;
        self.blob("canonical bytes payload")
    }

    /// A `bytes<N>` field.
    /// # Errors
    /// Corruption on a wrong tag or a payload that is not `N` bytes.
    pub fn next_fixed_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.expect_tag(tag::FIXED_BYTES, "canonical bytes field")?;
        <[u8; N]>::try_from(self.blob("canonical bytes payload")?)
            .map_err(|_| malformed("canonical bytes payload width"))
    }

    /// # Errors
    /// Corruption on a wrong tag or an empty/inverted span.
    pub fn next_interval_u64(&mut self) -> Result<Interval<u64>> {
        self.expect_tag(tag::INTERVAL_U64, "canonical interval field")?;
        let start = u64::from_be_bytes(self.word("canonical interval payload")?);
        let end = u64::from_be_bytes(self.word("canonical interval payload")?);
        Interval::new(start, end).ok_or(malformed("canonical interval payload"))
    }

    /// # Errors
    /// Corruption on a wrong tag or an empty/inverted span.
    pub fn next_interval_i64(&mut self) -> Result<Interval<i64>> {
        self.expect_tag(tag::INTERVAL_I64, "canonical interval field")?;
        let start = i64::from_be_bytes(self.word("canonical interval payload")?);
        let end = i64::from_be_bytes(self.word("canonical interval payload")?);
        Interval::new(start, end).ok_or(malformed("canonical interval payload"))
    }

    /// # Errors
    /// Corruption on a wrong tag, noncanonical endpoint bits, NaN
    /// endpoints, or an empty/inverted span.
    pub fn next_interval_f64(&mut self) -> Result<Interval<F64>> {
        self.expect_tag(tag::INTERVAL_F64, "canonical dense interval field")?;
        let start = F64::from_canonical_be_bytes(self.word("canonical dense interval payload")?)
            .map_err(|_| malformed("canonical dense interval payload"))?;
        let end = F64::from_canonical_be_bytes(self.word("canonical dense interval payload")?)
            .map_err(|_| malformed("canonical dense interval payload"))?;
        Interval::new(start, end).ok_or(malformed("canonical dense interval payload"))
    }

    fn blob(&mut self, what: &'static str) -> Result<&'a [u8]> {
        let len = u64::from_be_bytes(self.word(what)?);
        let len = usize::try_from(len).map_err(|_| malformed(what))?;
        self.take(len, what)
    }

    /// Assert the row is fully consumed with no trailing bytes.
    /// # Errors
    /// Corruption when fields or bytes remain.
    pub fn finish(self) -> Result<()> {
        if self.remaining != 0 || !self.rest.is_empty() {
            return Err(malformed("canonical row trailing bytes"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::RowReader;
    use crate::Value;
    use crate::canonical::CanonicalRow;
    use crate::error::{CorruptionError, Error};
    use crate::schema::{FieldDescriptor, ValueType};

    fn row(len: u16, bytes: &[u8]) -> CanonicalRow {
        let fields = [FieldDescriptor {
            name: "b".into(),
            value_type: ValueType::FixedBytes { len },
        }];
        CanonicalRow::encode(
            &fields,
            &[Value::FixedBytes(bytes.into())],
            &crate::WorkContext::new(),
        )
        .expect("encode")
    }

    #[test]
    fn fixed_bytes_read_at_their_declared_width() {
        let row = row(3, &[1, 2, 3]);
        let mut reader = RowReader::new(row.as_bytes()).expect("header");
        assert_eq!(reader.next_fixed_bytes::<3>().expect("read"), [1, 2, 3]);
        reader.finish().expect("consumed");

        let mut reader = RowReader::new(row.as_bytes()).expect("header");
        assert!(matches!(
            reader.next_fixed_bytes::<4>(),
            Err(Error::Corruption(CorruptionError::MalformedValue(_)))
        ));
    }
}
