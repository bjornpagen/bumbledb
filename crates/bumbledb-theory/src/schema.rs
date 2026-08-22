//! The schema as declared: descriptors, the shared value/type vocabulary,
//! .
//! This is the theory half of the schema surface: plain data a host (or
//! the `schema!` macro's expansion) constructs, and the pure judgments
//! over it — [`SchemaDescriptor::materialized_statements`] and
//! [`value_matches`]. The admission boundary stays engine-side: the only

pub mod spec;

use crate::value::Value;

/// Dense relation id: the relation's index in schema declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RelationId(pub u32);

/// Dense field id: the field's index in its relation's declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId(pub u16);

/// Dense statement id: the statement's index in the schema-global
/// materialized order — fresh auto-[`StatementDescriptor::Functionality`]
/// statements first, then closed auto-keys, then declared statements in
/// declaration order ([`SchemaDescriptor::materialized_statements`] owns
/// the rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatementId(pub u16);

/// The element domain of an Interval: closed to the two orderable scalars.
/// A flat enum, deliberately — no `Interval(Box<ValueType>)` recursion, so
/// illegal elements are unrepresentable rather than rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalElement {
    U64,
    I64,
}

/// A structural value type: the description *is* the identity — structural
/// equality of the description is type equality, and there is no name field
/// anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueType {
    Bool,
    U64,
    I64,
    String,

    FixedBytes {
        len: u16,
    },

    Interval {
        element: IntervalElement,
    },

    /// construction (`lean/Bumbledb/Values.lean: FixedU64.not_ray`).

    /// that merely checks is a CHECK constraint, refused
    FixedInterval {
        element: IntervalElement,
        width: u64,
    },
}

impl ValueType {
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool => 1,
            Self::U64 | Self::I64 | Self::String | Self::FixedInterval { .. } => 8,
            Self::FixedBytes { len } => (len as usize).div_ceil(8) * 8,
            Self::Interval { .. } => 16,
        }
    }

    #[must_use]
    pub const fn is_interval(self) -> bool {
        matches!(self, Self::Interval { .. } | Self::FixedInterval { .. })
    }

    #[must_use]
    pub const fn interval_element(self) -> Option<IntervalElement> {
        match self {
            Self::Interval { element } | Self::FixedInterval { element, .. } => Some(element),
            _ => None,
        }
    }
}

/// Field generation: a storage behavior, not a type
/// .
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generation {
    None,

    Fresh,
}

/// One field: name + structural type + generation attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: Box<str>,
    pub value_type: ValueType,
    pub generation: Generation,
}

/// vocabulary of the checking boundaries (query literals, bound params,
/// dynamic facts, statement selections). UTF-8 is [`Value::String`]'s
/// type, not a match failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueMismatch {
    Type,
}

/// The one `Value` ↔ `ValueType` compatibility check (kind, including
/// width) — IR validation, bind-time, the dynamic write path, and
/// selection validation all call this so the rules cannot drift apart.
/// Note the membership rule is *not* here: an element-typed value
/// against an `Interval` field is a kind mismatch to this check, and the
/// IR validation boundary owns that bivalence (the engine's
/// `ir::validate`, the bivalent-anchor resolution).
/// # Errors
/// [`ValueMismatch::Type`] on a wrong structural kind (including the
/// width rules).
pub fn value_matches(value: &Value, expected: &ValueType) -> Result<(), ValueMismatch> {
    match (value, expected) {
        (Value::Bool(_), ValueType::Bool)
        | (Value::U64(_), ValueType::U64)
        | (Value::I64(_), ValueType::I64)
        | (Value::String(_), ValueType::String)
        | (
            Value::IntervalU64(_),
            ValueType::Interval {
                element: IntervalElement::U64,
            },
        )
        | (
            Value::IntervalI64(_),
            ValueType::Interval {
                element: IntervalElement::I64,
            },
        ) => Ok(()),

        // `lean/Bumbledb/Values.lean: FixedU64.not_ray`). A wide or
        (
            Value::IntervalU64(interval),
            ValueType::FixedInterval {
                element: IntervalElement::U64,
                width,
            },
        ) if interval.end() - interval.start() == *width && !interval.is_ray() => Ok(()),
        (
            Value::IntervalI64(interval),
            ValueType::FixedInterval {
                element: IntervalElement::I64,
                width,
            },
        ) if interval.end().abs_diff(interval.start()) == *width && !interval.is_ray() => Ok(()),

        (Value::FixedBytes(raw), ValueType::FixedBytes { len }) => {
            if raw.len() == usize::from(*len) {
                Ok(())
            } else {
                Err(ValueMismatch::Type)
            }
        }
        _ => Err(ValueMismatch::Type),
    }
}

/// One σ binding's literal set — the disjunctive selection fragment
/// a MEMBER of the spelled set, bindings read conjunctively. The singleton
/// arm is today's equality by representation
/// stays zero-cost — no per-literal indirection on the one-literal path.
/// The `Many` arm's canonical form is sorted and duplicate-free with at
/// least two literals; validation canonicalizes the order and rejects the
/// degenerate spellings.
/// (`lean/Bumbledb/Schema.lean: Selection`): the selected field's value is
/// (`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`) and
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralSet {
    /// before the disjunctive extension, unchanged in meaning.
    One(Value),

    /// (`lean/Bumbledb/Countermodels.lean:
    Many(Box<[Value]>),
}

impl LiteralSet {
    #[must_use]
    pub fn literals(&self) -> &[Value] {
        match self {
            Self::One(literal) => std::slice::from_ref(literal),
            Self::Many(literals) => literals,
        }
    }

    #[must_use]
    pub fn as_equality(&self) -> Option<&Value> {
        match self {
            Self::One(literal) => Some(literal),
            Self::Many(_) => None,
        }
    }
}

impl From<Value> for LiteralSet {
    fn from(literal: Value) -> Self {
        Self::One(literal)
    }
}

/// One side of a containment: the single-atom query `R(X | φ)`
/// .
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    pub relation: RelationId,

    pub projection: Box<[FieldId]>,

    pub selection: Box<[(FieldId, LiteralSet)]>,
}

/// The measure of one source fact — a capacity statement's weight, the
/// TOTAL three-case sum (ruled 2026-07-24, C4: `Unit` is a case, not an
/// absence, so the wire, the descriptor encoding, and this type agree
/// that unit weight crosses explicitly;
/// (`<={lo..hi}` — the utterance survives character for character);
/// `Field` reads a u64-encoded SOURCE position (signed encodings are
/// gate-refused — polarity: a negative weight would let an insert lower
/// a sum); `DurationOf` reads a SOURCE interval position's measure (the
/// `lean/Bumbledb/Schema.lean: Weight`). `Unit` is the count instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weight {
    Unit,

    Field(FieldId),

    DurationOf(FieldId),
}

/// A capacity ceiling: a literal, or a DEPENDENT bound read from the
/// TARGET's row (per-group capacity — `{0..supply}`; ruled 2026-07-24,
/// C1: the ident resolves by NAME against the target's whole field
/// roster, never through the projection tuple;
/// interval measure of a target interval position — the calendar law's
/// `{0..Duration(span)}` ceiling. The floor is not a `Bound`: dependent
/// bounds are hi-slot only (ruled 2026-07-24, C6 — a dependent floor
/// has no use case, and inversion with idents is statically
/// `lean/Bumbledb/Schema.lean: Bound`). `TargetDuration` is the
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bound {
    Lit(u64),

    TargetField(FieldId),

    TargetDuration(FieldId),
}

/// One dependency statement: a judgment about queries
/// . Statements are anonymous —
/// their identity is their materialized-order [`StatementId`]. There is no
/// bidirectional variant: `==` is lowered to two `Containment` statements
/// with the sides swapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementDescriptor {
    Functionality {
        relation: RelationId,
        projection: Box<[FieldId]>,
    },

    Containment {
        source: Side,
        target: Side,
    },

    /// (`lean/Bumbledb/Capacity.lean: CapacityLaw`;
    /// `lean/Bumbledb/Schema.lean: Statement.capacity`). Field order is

    /// 2026-07-24, C2: the corpus JSON, the FFI marshal, the descriptor
    Capacity {
        target: Side,

        weight: Weight,

        lo: u64,

        hi: Option<Bound>,
        source: Side,
    },
}

/// The extension-row cap: a vocabulary larger than 256 is policy data
/// wearing a vocabulary costume, and the cap keeps every compiled word-set
/// a fixed 4×u64 bitset.
pub const MAX_EXTENSION_ROWS: usize = 256;

/// One ground axiom of a closed relation: the handle — the row's identity,
/// NOT a column — plus one value per declared intrinsic column, in
/// field-declaration order. The row id is the declaration index, exactly the
/// declaration-order rule relations, fields, and statements already obey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub handle: Box<str>,
    pub values: Box<[Value]>,
}

/// A closed relation's extension: its ground axioms in declaration order.
pub type Extension = Box<[Row]>;

/// One declared relation. `Some(extension)` declares it **closed** — its
/// rows are ground axioms, frozen by the fingerprint, virtual in storage,
/// write-refused; `None` is ordinary. No relation-kind enum exists: the
/// option *is* the kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: Box<str>,
    pub fields: Vec<FieldDescriptor>,
    pub extension: Option<Extension>,
}

/// One slot of a relation's SEALED shape — the numbering every dynamic
/// row, statement projection, and manifest field id addresses: a closed
/// relation's synthetic (`id`, u64) handle field at sealed ordinal 0
/// (no [`FieldDescriptor`] exists), then the declared fields in
/// declaration order. Synthetic vs declared is a sum, not a missing
/// descriptor.
#[derive(Debug, Clone, Copy)]
pub enum SealedField<'a> {
    SyntheticId,

    Declared(&'a FieldDescriptor),
}

impl<'a> SealedField<'a> {
    #[must_use]
    pub fn name(self) -> &'a str {
        match self {
            Self::SyntheticId => "id",
            Self::Declared(field) => &field.name,
        }
    }

    #[must_use]
    pub fn value_type(self) -> &'a ValueType {
        const SYNTHETIC_ID_TYPE: &ValueType = &ValueType::U64;
        match self {
            Self::SyntheticId => SYNTHETIC_ID_TYPE,
            Self::Declared(field) => &field.value_type,
        }
    }
}

impl RelationDescriptor {
    pub fn sealed_fields(&self) -> impl Iterator<Item = SealedField<'_>> {
        self.extension
            .is_some()
            .then_some(SealedField::SyntheticId)
            .into_iter()
            .chain(self.fields.iter().map(SealedField::Declared))
    }
}

/// The schema as declared: input to validation. Statements are
/// schema-level, between relations, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDescriptor {
    pub relations: Vec<RelationDescriptor>,
    pub statements: Vec<StatementDescriptor>,
}

impl SchemaDescriptor {
    /// # Panics

    #[must_use]
    pub fn materialized_statements(&self) -> Vec<StatementDescriptor> {
        let mut statements: Vec<StatementDescriptor> = Vec::new();
        for (rel_idx, relation) in self.relations.iter().enumerate() {
            for (sealed_idx, slot) in relation.sealed_fields().enumerate() {
                if let SealedField::Declared(field) = slot
                    && field.generation == Generation::Fresh
                {
                    statements.push(StatementDescriptor::Functionality {
                        relation: RelationId(
                            u32::try_from(rel_idx).expect("relation count fits u32"),
                        ),
                        projection: Box::new([FieldId(
                            u16::try_from(sealed_idx).expect("field count fits u16"),
                        )]),
                    });
                }
            }
        }

        for (rel_idx, relation) in self.relations.iter().enumerate() {
            if relation.extension.is_some() {
                statements.push(StatementDescriptor::Functionality {
                    relation: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
                    projection: Box::new([FieldId(0)]),
                });
            }
        }
        statements.extend(self.statements.iter().cloned());
        statements
    }
}

/// The statement-form tag, as plain data — the kind a bindings layer
/// the payload-carrying enums ([`StatementDescriptor`] / the engine's
/// `Violation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatementKind {
    Functionality,

    Containment,

    Capacity,
}

impl StatementDescriptor {
    #[must_use]
    pub const fn kind(&self) -> StatementKind {
        match self {
            Self::Functionality { .. } => StatementKind::Functionality,
            Self::Containment { .. } => StatementKind::Containment,
            Self::Capacity { .. } => StatementKind::Capacity,
        }
    }
}
