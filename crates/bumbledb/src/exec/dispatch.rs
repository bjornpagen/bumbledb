//! Guard-probe access path dispatch (docs/architecture/30-execution.md): the point-lookup fast path
//! that routes qualifying queries around the join machinery entirely
//! (`docs/architecture/30-execution.md` — access paths; `40-storage.md`'s
//! `U`/`M` read-side readers).
//!
//! The dispatch is a **representation**, not a runtime mode: classification
//! happens once at prepare time into the two-variant [`ExecPlan`]; the
//! branch exists exactly once. No images are touched on the guard path —
//! it works identically on a cold, just-committed database (the latency
//! property the decision exists for).

use crate::image::view::{Const, FilterPredicate};
use crate::ir::VarId;
use crate::plan::fj::ValidatedPlan;
use crate::schema::{ConstraintId, FieldId, RelationId};

mod classify;
mod exec_plan;
mod execute_guard;
mod fact_word;
mod guard_probe_fact;
#[cfg(test)]
mod tests;

pub use classify::classify;
pub use execute_guard::execute_guard;
pub(crate) use fact_word::fact_word;
pub(crate) use guard_probe_fact::guard_probe_fact;

/// The prepared execution plan: either the guard-probe fast path or the
/// Free Join engine.
#[derive(Debug)]
pub enum ExecPlan {
    GuardProbe(GuardPlan),
    FreeJoin(ValidatedPlan),
}

/// The point-lookup plan: one `U`-guard (or `M`-membership) get, one `F`
/// fetch, a decode — no images, no COLT, no plan search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardPlan {
    pub relation: RelationId,
    /// The probed unique constraint; `None` means every field is constant
    /// and the probe is a full-fact `M` membership check.
    pub constraint: Option<ConstraintId>,
    /// The key constants in guard-key field order.
    pub key: Vec<(FieldId, Const)>,
    /// Filters not consumed by the key, checked on the fetched fact
    /// (fields outside the unique key may still be constrained).
    pub remaining_filters: Vec<FilterPredicate>,
    /// Variables decoded from the fetched fact: `(field, var)`; slot order
    /// is this order.
    pub vars: Vec<(FieldId, VarId)>,
}
