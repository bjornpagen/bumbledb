use std::path::Path;

use crate::error::Result;
use crate::schema::Schema;

use super::Environment;

impl Environment {
    /// # Errors
    /// exists (including as an empty directory), `Lmdb` on any LMDB
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn create(path: &Path, schema: &Schema) -> Result<Self> {
        let created = Self::publish_empty(path, schema)?;
        crate::obs::event(
            crate::obs::names::CREATE_DURABLE,
            crate::obs::TraceArgs::Count(2),
        );
        Ok(created)
    }
}
