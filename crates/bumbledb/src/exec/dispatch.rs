//! The key-probe access path: a rule whose key is fully bound reads one row
//! through the source's key index or exact membership, with no images and no
//! join. Classification happens once at prepare into the prepared rule sum.
use crate::image::view::{Const, FilterPredicate};
use crate::ir::VarId;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

mod classify;
mod execute_key_probe;
mod fact_word;
mod key_probe_fact;
#[cfg(test)]
mod tests;

pub(crate) use classify::classify;
pub(crate) use execute_key_probe::execute_key_probe;
pub(crate) use fact_word::FactOperand;
pub(crate) use key_probe_fact::key_probe_row;

/// One variable a key-probe plan decodes from the fetched fact: the field it
/// reads and its binding-slot span (the `SlotWidth` layout — an interval
/// variable spans two consecutive word slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyProbeVar {
    pub field: FieldId,
    pub var: VarId,

    pub slot: usize,

    pub width: usize,
}

/// One schema-sealed key field and its fixed word range. The three u16s
/// occupy the alignment space beside Const; execution needs no span Vec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyProbePart {
    pub field: FieldId,
    pub start: u16,
    pub end: u16,
    pub value: Const,
}

const _: () = assert!(
    std::mem::size_of::<KeyProbePart>() <= std::mem::size_of::<(FieldId, Const)>(),
    "sealed word ranges must not grow the existing key allocation"
);

impl KeyProbePart {
    pub(crate) fn words(&self) -> std::ops::Range<usize> {
        usize::from(self.start)..usize::from(self.end)
    }
}

/// U vs M access path. Trusted layer: Option-as-tag is accidental.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyProbeKind {
    Uniqueness {
        statement: StatementId,
        projection: crate::schema::ProjectionId,
        key: Vec<KeyProbePart>,
    },
    Membership {
        key: Vec<KeyProbePart>,
    },
}

impl KeyProbeKind {
    pub(crate) fn key(&self) -> &[KeyProbePart] {
        match self {
            Self::Uniqueness { key, .. } | Self::Membership { key } => key,
        }
    }
}

/// The point-lookup plan: one `U` determinant (or `M`-membership) get, one `F`
/// fetch, a decode — no images, no COLT, no plan search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeyProbePlan {
    pub relation: RelationId,
    pub kind: KeyProbeKind,

    pub remaining_filters: Vec<FilterPredicate>,

    pub vars: Vec<KeyProbeVar>,
}

impl KeyProbePlan {
    #[must_use]
    pub(crate) fn slot_of(&self, var: VarId) -> usize {
        self.vars
            .iter()
            .find(|binding| binding.var == var)
            .expect("key-probe plans bind every variable")
            .slot
    }

    #[must_use]
    pub(crate) fn width_of(&self, var: VarId) -> usize {
        self.vars
            .iter()
            .find(|binding| binding.var == var)
            .expect("key-probe plans bind every variable")
            .width
    }

    #[must_use]
    pub(crate) fn slot_count(&self) -> usize {
        self.vars.last().map_or(0, |v| v.slot + v.width)
    }
}

/// What a key probe reads: the source, its schema, the text resolver and the
/// bound parameters.
#[derive(Clone, Copy)]
pub(crate) struct ProbeCtx<'a> {
    pub(crate) source: &'a crate::api::prepared::source::QuerySource<'a>,
    pub(crate) schema: &'a crate::schema::Schema,
    pub(crate) interner: &'a crate::image::intern::InternerHandle<'a>,
    pub(crate) params: &'a [Const],
}

/// One key-probe rule's buffers, reused across probes: the decoded candidate
/// row, the resolved key words, the key's values, and the reference walk's
/// key fields and first words. Every probe replaces their contents; nothing
/// is memoized.
pub(crate) struct ProbeBuffers {
    pub(crate) row: crate::image::canon::RowWords,
    key: crate::image::view::ResolvedWords,
    values: Vec<crate::ir::Value>,
    walk_fields: Vec<FieldId>,
    walk_words: Vec<u64>,
}

impl ProbeBuffers {
    pub(crate) fn new(field_types: &[bumbledb_theory::schema::ValueType]) -> Self {
        Self {
            row: crate::image::canon::RowWords::prepared(field_types),
            key: crate::image::view::ResolvedWords::default(),
            values: Vec::with_capacity(field_types.len()),
            walk_fields: Vec::new(),
            walk_words: Vec::new(),
        }
    }

    pub(crate) fn release_memory(&mut self) {
        self.row.release_memory();
        self.key = crate::image::view::ResolvedWords::default();
        self.values = Vec::new();
        self.walk_fields = Vec::new();
        self.walk_words = Vec::new();
    }
}
