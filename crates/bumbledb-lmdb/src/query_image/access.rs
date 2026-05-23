use std::ops::Range;

use crate::AccessId;
use crate::query_image::FieldId;

/// Immutable durable sorted index bytes for one relation image.
#[derive(Clone, Debug)]
pub struct RelationIndexImage {
    /// Dense storage access ID.
    #[cfg_attr(not(test), expect(dead_code, reason = "access id is diagnostic"))]
    pub access: AccessId,
    /// Leading fields in access-path order.
    pub fields: Vec<FieldId>,
    /// Encoded key components in access order.
    pub components: Vec<RelationAccessComponent>,
    /// Bytes per encoded index entry.
    pub encoded_len: usize,
    /// Namespace/relation/access prefix bytes before components.
    pub prefix_len: usize,
    /// Concatenated encoded index entries.
    pub bytes: Vec<u8>,
}

/// One field component inside a durable relation index image.
#[derive(Clone, Debug)]
pub struct RelationAccessComponent {
    /// Field ID in relation declaration order.
    pub field: FieldId,
    /// Offset of this component inside an encoded index entry.
    pub offset: usize,
    /// Encoded component width.
    pub width: usize,
}

impl RelationIndexImage {
    /// Returns true when this encoded index entry layout contains `field`.
    pub fn contains_field(&self, field: FieldId) -> bool {
        self.components
            .iter()
            .any(|component| component.field == field)
    }

    /// Returns the encoded field bytes from one encoded index entry.
    pub fn component_bytes<'a>(&self, entry: &'a [u8], field: FieldId) -> Option<&'a [u8]> {
        let component = self
            .components
            .iter()
            .find(|component| component.field == field)?;
        entry.get(component.offset..component.offset + component.width)
    }

    /// Returns encoded entries matching a leading component prefix.
    #[cfg(test)]
    pub fn entries_with_prefix<'a>(&'a self, prefix: &'a [u8]) -> RelationIndexPrefixIter<'a> {
        let range = self.prefix_range(prefix);
        RelationIndexPrefixIter {
            index: self,
            prefix,
            position: range.start,
            end: range.end,
        }
    }

    /// Returns the half-open entry-position range matching a leading component prefix.
    pub fn prefix_range(&self, prefix: &[u8]) -> Range<usize> {
        debug_assert!(prefix.len() <= self.encoded_len.saturating_sub(self.prefix_len));
        let start = self.lower_bound_prefix(prefix);
        let end = self.upper_bound_prefix(prefix);
        start..end
    }

    /// Returns the number of encoded index entries matching a leading component prefix.
    #[cfg(test)]
    pub fn prefix_count(&self, prefix: &[u8]) -> usize {
        debug_assert!(prefix.len() <= self.encoded_len.saturating_sub(self.prefix_len));
        let range = self.prefix_range(prefix);
        range.end.saturating_sub(range.start)
    }

    /// Returns true when any encoded index entry matches a leading component prefix.
    #[cfg(test)]
    pub fn prefix_exists(&self, prefix: &[u8]) -> bool {
        let range = self.prefix_range(prefix);
        range.start < range.end
    }

    /// Returns an encoded entry by entry position.
    pub fn entry_at(&self, position: usize) -> Option<&[u8]> {
        self.entry(position)
    }

    fn lower_bound_prefix(&self, prefix: &[u8]) -> usize {
        let entry_count = self.bytes.len() / self.encoded_len;
        let mut low = 0usize;
        let mut high = entry_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let entry = self.entry(mid).unwrap_or(&[]);
            let key = self.entry_prefix(entry, prefix.len()).unwrap_or(&[]);
            if key < prefix {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn upper_bound_prefix(&self, prefix: &[u8]) -> usize {
        let entry_count = self.bytes.len() / self.encoded_len;
        let mut low = 0usize;
        let mut high = entry_count;
        while low < high {
            let mid = low + (high - low) / 2;
            let entry = self.entry(mid).unwrap_or(&[]);
            let key = self.entry_prefix(entry, prefix.len()).unwrap_or(&[]);
            if key <= prefix {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn entry(&self, position: usize) -> Option<&[u8]> {
        let start = position.checked_mul(self.encoded_len)?;
        self.bytes.get(start..start + self.encoded_len)
    }

    fn entry_prefix<'a>(&self, entry: &'a [u8], len: usize) -> Option<&'a [u8]> {
        entry.get(self.prefix_len..self.prefix_len + len)
    }
}

/// Iterator over durable index entries matching an encoded prefix.
#[cfg(test)]
pub struct RelationIndexPrefixIter<'a> {
    index: &'a RelationIndexImage,
    prefix: &'a [u8],
    position: usize,
    end: usize,
}

#[cfg(test)]
impl<'a> Iterator for RelationIndexPrefixIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.end {
            return None;
        }
        let entry = self.index.entry(self.position)?;
        let key = self.index.entry_prefix(entry, self.prefix.len())?;
        if key != self.prefix {
            self.position = self.end;
            return None;
        }
        self.position += 1;
        Some(entry)
    }
}
