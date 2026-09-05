use std::path::Path;

use bumbledb::schema::Theory;
use bumbledb::{Admission, Db};

/// The engine store mode. Exactly one point remains: **durable** (LMDB
/// default fsync per commit).
///
/// ENG-008 decision, recorded (2026-09-04, P14): the successor deleted the
/// whole `*_nosync` constructor surface — no hidden no-sync capability
/// exists in production, and chapter 40/41 forbid a benchmark re-adding
/// one. A bench-side `MDB_NOSYNC` reimplementation would have to fork the
/// store's environment owner (a fake engine, not a measurement of this
/// one), so the ours-side NOSYNC baseline lane is **dropped**, not
/// re-homed. The type survives as a single-variant seam because lane
/// signatures across the crate (including P11-owned lanes) thread it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoreMode {
    #[default]
    Durable,
}

impl StoreMode {
    /// # Errors
    pub fn create<S: Theory>(self, path: &Path, schema: S) -> Result<Db<S>, String> {
        match Db::create(path, schema) {
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
        Db::open(path, schema).map_err(|error| format!("open ({}): {error:?}", self.label()))
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
        }
    }
}
