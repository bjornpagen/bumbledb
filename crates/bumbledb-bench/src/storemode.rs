use std::path::Path;

use bumbledb::schema::Theory;
use bumbledb::{Admission, Db};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreMode {
    #[default]
    Durable,

    Nosync,
}

impl StoreMode {

    /// # Errors

    pub fn create<S: Theory>(self, path: &Path, schema: S) -> Result<Db<S>, String> {
        match match self {
            Self::Durable => Db::create(path, schema),
            Self::Nosync => Db::create_nosync(path, schema),
        } {
            Err(error) => Err(format!("create ({}): {error:?}", self.label())),
            Ok(Admission::Accepted(db)) => Ok(db),
            Ok(Admission::Rejected(violations)) => Err(format!(
                "create ({}): empty rejected: {violations}",
                self.label()
            )),
        }
    }

    /// # Errors

    pub fn open<S: Theory>(self, path: &Path, schema: S) -> Result<Db<S>, String> {
        match self {
            Self::Durable => Db::open(path, schema),
            Self::Nosync => Db::open_nosync(path, schema),
        }
        .map_err(|error| format!("open ({}): {error:?}", self.label()))
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Nosync => "nosync",
        }
    }
}
