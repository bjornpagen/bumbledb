use crate::storemode::StoreMode;

use super::engines;
use super::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteLaneKind {

    Bare,

    Maint,

    Nosync,
}

impl SqliteLaneKind {

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Bare => "sqlite-bare",
            Self::Maint => "sqlite-maint",
            Self::Nosync => "sqlite-nosync",
        }
    }

    #[must_use]
    pub fn sync(self) -> engines::SqliteSync {
        match self {
            Self::Bare | Self::Maint => engines::SqliteSync::Full,
            Self::Nosync => engines::SqliteSync::Nosync,
        }
    }

    #[must_use]
    pub fn maintained(self) -> bool {
        self == Self::Maint
    }
}

#[must_use]
pub fn ours_label(mode: StoreMode) -> &'static str {
    match mode {
        StoreMode::Durable => "ours-durable",
        StoreMode::Nosync => "ours-ephemeral",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSpec {

    pub name: &'static str,

    pub mix: ops::Mix,

    pub ours: StoreMode,

    pub sqlite: &'static [SqliteLaneKind],
}

#[must_use]
pub fn all() -> &'static [RunSpec] {
    &[
        RunSpec {
            name: "steady",
            mix: ops::STEADY,
            ours: StoreMode::Durable,
            sqlite: &[SqliteLaneKind::Bare, SqliteLaneKind::Maint],
        },
        RunSpec {
            name: "nosync",
            mix: ops::STEADY,
            ours: StoreMode::Nosync,
            sqlite: &[SqliteLaneKind::Nosync],
        },
        RunSpec {
            name: "delete-heavy",
            mix: ops::DELETE_HEAVY,
            ours: StoreMode::Durable,
            sqlite: &[SqliteLaneKind::Bare],
        },
    ]
}
