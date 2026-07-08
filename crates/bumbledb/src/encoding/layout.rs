//! Layout machinery: computing a relation's dense fact byte layout.

use super::{FactLayout, TypeDesc};

impl FactLayout {
    /// Computes the layout for the given field types in declaration order.
    #[must_use]
    pub fn new(field_types: &[TypeDesc]) -> Self {
        let mut offset = 0;
        let fields = field_types
            .iter()
            .map(|&desc| {
                let slot = (offset, desc);
                offset += desc.width();
                slot
            })
            .collect();
        Self {
            fields,
            fact_width: offset,
        }
    }
}
