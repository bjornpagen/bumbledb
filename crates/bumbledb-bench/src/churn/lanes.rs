//! The churn lane registry — pure data, mechanically banned from the
//! clock (timing belongs to [`super::run`] alone). A run is a
//! [`RunSpec`] row: every run structurally carries EXACTLY ONE ours
//! lane — the id-minter whose alloc stream names the fresh ids all
//! twins share — so "which store mints" is never a runtime question.
//! The five mandated lanes are three registry rows, not five code
//! paths: the driver ([`super::run`]) folds over [`all`] and nothing
//! else knows the roster.

use crate::storemode::StoreMode;

use super::engines;
use super::ops;

/// One `SQLite` twin kind — the mirror configurations the registry can
/// name:
///
/// - [`Bare`](Self::Bare) is the standard fairness session left alone
///   for the whole life — no operator, no maintenance, the store ages
///   as it ages.
/// - [`Maint`](Self::Maint) is the same session with the operator's
///   periodic VACUUM + ANALYZE, their wall time charged INTO the lane's
///   own throughput series (maintenance-included honesty — `SQLite`
///   gets its best realistic self, and pays for it on the record).
/// - [`Nosync`](Self::Nosync) is the `synchronous=OFF` twin matched to
///   LMDB `NOSYNC` — the ephemeral pairing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteLaneKind {
    /// The fairness session, untouched for the whole run.
    Bare,
    /// The fairness session plus the operator's periodic maintenance,
    /// paid on the record.
    Maint,
    /// The `synchronous=OFF` twin of the ephemeral store kind.
    Nosync,
}

impl SqliteLaneKind {
    /// The lane label, as reports print it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Bare => "sqlite-bare",
            Self::Maint => "sqlite-maint",
            Self::Nosync => "sqlite-nosync",
        }
    }

    /// The kind's sync twin: the maintained lane runs the SAME fairness
    /// session as the bare one — maintenance is a schedule, not a
    /// session change.
    #[must_use]
    pub fn sync(self) -> engines::SqliteSync {
        match self {
            Self::Bare | Self::Maint => engines::SqliteSync::Full,
            Self::Nosync => engines::SqliteSync::Nosync,
        }
    }

    /// Whether the operator's periodic VACUUM + ANALYZE runs on this
    /// lane (and gets charged into its own series).
    #[must_use]
    pub fn maintained(self) -> bool {
        self == Self::Maint
    }
}

/// The ours-side lane label for a store mode, as reports print it.
#[must_use]
pub fn ours_label(mode: StoreMode) -> &'static str {
    match mode {
        StoreMode::Durable => "ours-durable",
        StoreMode::Ephemeral => "ours-ephemeral",
    }
}

/// One run row: a name, its mix, the ONE ours lane (the id-minter), and
/// the `SQLite` twins riding the same logical operation stream. The
/// shape carries the law: a spec cannot hold two minters or zero, so
/// lockstep twinning is a property of the type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSpec {
    /// The run's name, as reports print it.
    pub name: &'static str,
    /// The per-cycle operation mix.
    pub mix: ops::Mix,
    /// The one ours lane — which store kind mints this run's ids.
    pub ours: StoreMode,
    /// The `SQLite` twins receiving the identical logical operations.
    pub sqlite: &'static [SqliteLaneKind],
}

/// The registry: exactly three rows covering the five mandated lanes.
///
/// - `steady`: the headline degradation story — the durable store vs
///   the bare fairness session vs the maintained session, all aging
///   under the same steady churn; whatever degrades, degrades on the
///   record, and the maintained lane shows what `SQLite`'s best
///   realistic self costs.
/// - `nosync`: the matched no-fsync pair — the ephemeral (LMDB
///   `NOSYNC`) store vs the `synchronous=OFF` session, both commits
///   stopping at the OS page cache, so the curve isolates engine work
///   from media waits.
/// - `delete-heavy`: half the working set churned per cycle — the
///   compact-on-delete plateau vs `SQLite`'s freelist growth, the
///   store-size story the counters were built for.
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
            ours: StoreMode::Ephemeral,
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
