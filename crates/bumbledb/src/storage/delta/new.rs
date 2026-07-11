use std::collections::BTreeMap;

use crate::arena::Arena;
use crate::schema::Schema;

use super::WriteDelta;

impl<'s> WriteDelta<'s> {
    #[must_use]
    pub fn new(schema: &'s Schema) -> Self {
        Self {
            schema,
            arena: Arena::new(),
            facts: BTreeMap::new(),
            guards: BTreeMap::new(),
            guard_scratch: Vec::new(),
            marks: BTreeMap::new(),
            row_count_delta: BTreeMap::new(),
            pending_interns: BTreeMap::new(),
            dict_next: None,
        }
    }
}
