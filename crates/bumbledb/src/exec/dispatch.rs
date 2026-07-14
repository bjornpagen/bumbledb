//! KeyProbe-probe access path dispatch (docs/architecture/40-execution.md,
//! § access paths): the point-lookup fast path that routes qualifying
//! queries around the join machinery entirely (`50-storage.md`'s `U`/`M`
//! read-side readers).
//!
//! The dispatch is a **representation**, not a runtime mode: classification
//! happens once at prepare time into the prepared rule sum; the branch
//! exists exactly once. No images are touched on the key-probe path —
//! it works identically on a cold, just-committed database (the latency
//! property the decision exists for).

use crate::image::view::{Const, FilterPredicate};
use crate::ir::VarId;
use crate::schema::{FieldId, RelationId, StatementId};

mod classify;
mod execute_key_probe;
mod fact_word;
mod key_probe_fact;
#[cfg(test)]
mod tests;

pub use classify::classify;
pub use execute_key_probe::execute_key_probe;
pub(crate) use fact_word::{FactOperand, fact_operand, fact_word};
pub(crate) use key_probe_fact::key_probe_fact;

/// One variable a key-probe plan decodes from the fetched fact: the field it
/// reads and its binding-slot span (the `SlotWidth` layout — an interval
/// variable spans two consecutive word slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyProbeVar {
    pub field: FieldId,
    pub var: VarId,
    /// First binding slot.
    pub slot: usize,
    /// Width in words: 2 for an interval variable, 1 otherwise.
    pub width: usize,
}

/// The point-lookup plan: one `U` determinant (or `M`-membership) get, one `F`
/// fetch, a decode — no images, no COLT, no plan search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyProbePlan {
    pub relation: RelationId,
    /// The matched key (`Functionality`) statement, probed through its `U`
    /// determinant index; `None` means every field is bound by value and the probe is
    /// the full-fact `M` membership check.
    pub statement: Option<StatementId>,
    /// The key constants in determinant-byte order: the statement's projection
    /// order for a `U` probe, field declaration order for the `M` path.
    pub key: Vec<(FieldId, Const)>,
    /// Filters not consumed by the key, checked on the fetched fact
    /// (fields outside the key's projection may still be constrained).
    pub remaining_filters: Vec<FilterPredicate>,
    /// Variables decoded from the fetched fact; slot layout follows this
    /// order through each entry's `(slot, width)` span.
    pub vars: Vec<KeyProbeVar>,
}

impl KeyProbePlan {
    /// The first slot index of a variable.
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        self.vars
            .iter()
            .find(|binding| binding.var == var)
            .expect("key-probe plans bind every variable")
            .slot
    }

    /// A variable's slot width in words.
    #[must_use]
    pub fn width_of(&self, var: VarId) -> usize {
        self.vars
            .iter()
            .find(|binding| binding.var == var)
            .expect("key-probe plans bind every variable")
            .width
    }

    /// Total binding-slot words (the `SlotWidth` layout over `vars`).
    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.vars.last().map_or(0, |v| v.slot + v.width)
    }
}
