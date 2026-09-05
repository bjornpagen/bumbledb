//! KeyProbe-probe access path dispatch: the point-lookup fast path that routes qualifying
//! read-side readers).
//! The dispatch is a **representation**, not a runtime mode: classification
//! happens once at prepare time into the prepared rule sum; the branch
//! exists exactly once. No images are touched on the key-probe path —
//! it works identically on a cold, just-committed database (the latency
use crate::image::view::{Const, FilterPredicate};
use crate::ir::VarId;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

mod classify;
mod execute_key_probe;
mod fact_word;
mod key_probe_fact;
#[cfg(test)]
mod tests;

pub use classify::classify;
pub use execute_key_probe::execute_key_probe;
pub(crate) use fact_word::FactOperand;
pub(crate) use key_probe_fact::key_probe_row;

/// One variable a key-probe plan decodes from the fetched fact: the field it
/// reads and its binding-slot span (the `SlotWidth` layout — an interval
/// variable spans two consecutive word slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyProbeVar {
    pub field: FieldId,
    pub var: VarId,

    pub slot: usize,

    pub width: usize,
}

/// U vs M access path. Trusted layer: Option-as-tag is accidental.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyProbeKind {
    Uniqueness {
        statement: StatementId,
        key: Vec<(FieldId, Const)>,
    },
    Membership {
        key: Vec<(FieldId, Const)>,
    },
}

impl KeyProbeKind {
    pub fn key(&self) -> &[(FieldId, Const)] {
        match self {
            Self::Uniqueness { key, .. } | Self::Membership { key } => key,
        }
    }
}

/// The point-lookup plan: one `U` determinant (or `M`-membership) get, one `F`
/// fetch, a decode — no images, no COLT, no plan search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyProbePlan {
    pub relation: RelationId,
    pub kind: KeyProbeKind,

    pub remaining_filters: Vec<FilterPredicate>,

    pub vars: Vec<KeyProbeVar>,
}

impl KeyProbePlan {
    #[must_use]
    pub fn slot_of(&self, var: VarId) -> usize {
        self.vars
            .iter()
            .find(|binding| binding.var == var)
            .expect("key-probe plans bind every variable")
            .slot
    }

    #[must_use]
    pub fn width_of(&self, var: VarId) -> usize {
        self.vars
            .iter()
            .find(|binding| binding.var == var)
            .expect("key-probe plans bind every variable")
            .width
    }

    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.vars.last().map_or(0, |v| v.slot + v.width)
    }
}
