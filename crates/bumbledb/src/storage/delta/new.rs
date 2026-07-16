use std::collections::BTreeMap;

use crate::arena::Arena;
use crate::schema::Schema;
use crate::storage::keys::DeterminantImage;

use super::WriteDelta;

impl<'s> WriteDelta<'s> {
    #[must_use]
    pub fn new(schema: &'s Schema) -> Self {
        Self {
            schema,
            arena: Arena::new(),
            facts: BTreeMap::new(),
            determinants: BTreeMap::new(),
            determinant_scratch: DeterminantImage::scratch(),
            #[cfg(test)]
            determinant_scratch_clones: 0,
            marks: BTreeMap::new(),
            row_count_delta: BTreeMap::new(),
            pending_interns: BTreeMap::new(),
            dict_next: None,
        }
    }
}
