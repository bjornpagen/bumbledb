//! Golden example manifest for the set-native rewrite.

/// Deterministic golden example family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoldenFamily {
    /// Ledger/normalized accounting facts.
    Ledger,
    /// Sailors/boats/reserves many-to-many facts.
    Sailors,
    /// Join stress chains and cyclic triangles.
    Joinstress,
    /// TPC-H-inspired normalized commerce subset.
    TpchSubset,
    /// IMDb/JOB-inspired title/name/principal subset.
    ImdbJobSubset,
    /// Lahman-inspired compound sports facts.
    LahmanSubset,
    /// LDBC-inspired social graph subset.
    LdbcSubset,
}

/// Static golden family manifest. Every family listed here must have exact
/// correctness tests before rewrite PRDs may complete.
pub const GOLDEN_FAMILIES: &[GoldenFamily] = &[
    GoldenFamily::Ledger,
    GoldenFamily::Sailors,
    GoldenFamily::Joinstress,
    GoldenFamily::TpchSubset,
    GoldenFamily::ImdbJobSubset,
    GoldenFamily::LahmanSubset,
    GoldenFamily::LdbcSubset,
];

impl GoldenFamily {
    /// Stable family name used by test diagnostics and benchmark metadata.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ledger => "ledger",
            Self::Sailors => "sailors",
            Self::Joinstress => "joinstress",
            Self::TpchSubset => "tpch_subset",
            Self::ImdbJobSubset => "imdb_job_subset",
            Self::LahmanSubset => "lahman_subset",
            Self::LdbcSubset => "ldbc_subset",
        }
    }
}
