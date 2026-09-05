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
/// materialized order — closed relations' auto-handle
/// [`StatementDescriptor::Functionality`] statements first, then declared
/// statements in declaration order
/// ([`SchemaDescriptor::materialized_statements`] owns the rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StatementId(pub u16);

/// The element domain of a general Interval: closed to the three
/// orderable scalars. A flat enum, deliberately — no
/// `Interval(Box<ValueType>)` recursion, so illegal elements are
/// unrepresentable rather than rejected. `F64` is the dense numeric line
/// with canonical binary64 endpoints; the integers keep their discrete
/// point domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalElement {
    U64,
    I64,
    F64,
}

/// The element domain of a fixed-width interval: the discrete integers
/// only. `FixedInterval<F64>` is unrepresentable by this type — rounded
/// `start + width` is not an exact fixed-width representation on the
/// dense line, so the illegal state has no descriptor spelling at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FixedIntervalElement {
    U64,
    I64,
}

impl FixedIntervalElement {
    /// The general element domain this discrete element embeds into.
    #[must_use]
    pub const fn element(self) -> IntervalElement {
        match self {
            Self::U64 => IntervalElement::U64,
            Self::I64 => IntervalElement::I64,
        }
    }
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
    F64,

    /// The application-owned 128-bit identity scalar: sixteen exact bytes.
    /// Nominal host wrappers lower here; the name is not a second kernel,
    /// and no database issuance/reservation authority exists for it.
    Id128,

    FixedBytes {
        len: u16,
    },

    Interval {
        element: IntervalElement,
    },

    /// `interval<E, w>` — width is the type; encoding stores only the start.
    /// `lean/Bumbledb/Values.lean: FixedU64.not_ray`. A width that merely
    /// checks is refused, and the element domain is discrete by type:
    /// no `FixedInterval<F64>` can be declared.
    FixedInterval {
        element: FixedIntervalElement,
        width: u64,
    },
}

impl ValueType {
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool => 1,
            Self::U64 | Self::I64 | Self::F64 | Self::String | Self::FixedInterval { .. } => 8,
            Self::Id128 | Self::Interval { .. } => 16,
            Self::FixedBytes { len } => (len as usize).div_ceil(8) * 8,
        }
    }

    #[must_use]
    pub const fn is_interval(self) -> bool {
        matches!(self, Self::Interval { .. } | Self::FixedInterval { .. })
    }

    #[must_use]
    pub const fn interval_element(self) -> Option<IntervalElement> {
        match self {
            Self::Interval { element } => Some(element),
            Self::FixedInterval { element, .. } => Some(element.element()),
            _ => None,
        }
    }

    /// Whether this is an interval position with an exact integer duration
    /// — the only interval family a grouped duration measure or a
    /// duration-dimension bound may read. Dense float intervals have a
    /// numerical length, never an exact capacity weight.
    #[must_use]
    pub const fn is_discrete_interval(self) -> bool {
        matches!(
            self,
            Self::Interval {
                element: IntervalElement::U64 | IntervalElement::I64,
            } | Self::FixedInterval { .. }
        )
    }
}

/// One field: name + structural type. There is no generation attribute:
/// the database issues no identity; application-owned [`crate::Id128`]
/// values (or any declared key domain) arrive as ordinary input, and key
/// laws are declared statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: Box<str>,
    pub value_type: ValueType,
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
        | (Value::F64(_), ValueType::F64)
        | (Value::Id128(_), ValueType::Id128)
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
        )
        | (
            Value::IntervalF64(_),
            ValueType::Interval {
                element: IntervalElement::F64,
            },
        ) => Ok(()),

        // `lean/Bumbledb/Values.lean: FixedU64.not_ray`). A wide or
        (
            Value::IntervalU64(interval),
            ValueType::FixedInterval {
                element: FixedIntervalElement::U64,
                width,
            },
        ) if interval.end() - interval.start() == *width && !interval.is_ray() => Ok(()),
        (
            Value::IntervalI64(interval),
            ValueType::FixedInterval {
                element: FixedIntervalElement::I64,
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

/// One σ binding's literal set. The selected field's value is a member of the spelled set.
/// `lean/Bumbledb/Schema.lean: Selection`, `Selection.singleton_satisfies_iff`.
/// `Many` is sorted, duplicate-free, at least two literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralSet {
    /// One literal: the equality binding.
    One(Value),

    /// Two or more literals, read disjunctively.
    /// `lean/Bumbledb/Countermodels.lean: disjunctive_window_not_literal_conjunction`.
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

    /// `lean/Bumbledb/Capacity.lean: CapacityLaw`; `lean/Bumbledb/Schema.lean: Statement.capacity`.
    /// Field order is target, weight, window, source.
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
    ///
    /// When a relation or field ordinal exceeds the id space (`u32`/`u16`)
    /// — impossible for a descriptor the acceptance gate admitted.
    #[must_use]
    pub fn materialized_statements(&self) -> Vec<StatementDescriptor> {
        let mut statements: Vec<StatementDescriptor> = Vec::new();
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

#[cfg(test)]
mod tests {
    use super::{
        FieldDescriptor, FieldId, FixedIntervalElement, IntervalElement, RelationDescriptor,
        RelationId, Row, SchemaDescriptor, StatementDescriptor, ValueMismatch, ValueType,
        value_matches,
    };
    use crate::{F64, Id128, Interval, Value};

    fn field(name: &str, value_type: ValueType) -> FieldDescriptor {
        FieldDescriptor {
            name: name.into(),
            value_type,
        }
    }

    /// The materialized order holds only closed auto-handle keys and
    /// declared statements: no fresh auto-key exists anywhere — E-NO-RESERVE
    /// at the descriptor layer.
    #[test]
    fn materialized_statements_have_no_generated_identity_keys() {
        let descriptor = SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Student".into(),
                    fields: vec![field("id", ValueType::Id128)],
                    extension: None,
                },
                RelationDescriptor {
                    name: "Kind".into(),
                    fields: vec![],
                    extension: Some(Box::new([Row {
                        handle: "Basic".into(),
                        values: Box::new([]),
                    }])),
                },
            ],
            statements: vec![StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            }],
        };
        let materialized = descriptor.materialized_statements();
        // Closed auto-handle key first, then the one declared key. The
        // ordinary Id128 relation receives no automatic statement.
        assert_eq!(materialized.len(), 2);
        assert!(matches!(
            &materialized[0],
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection,
            } if **projection == [FieldId(0)]
        ));
        assert_eq!(materialized[1], descriptor.statements[0]);
    }

    #[test]
    fn value_matches_covers_the_new_scalars_and_refuses_cross_kinds() {
        let id = Value::Id128(Id128::from_bytes([7; 16]));
        assert_eq!(value_matches(&id, &ValueType::Id128), Ok(()));
        // Id128 is nominal: not raw bytes<16>, not an integer pair.
        assert_eq!(
            value_matches(&id, &ValueType::FixedBytes { len: 16 }),
            Err(ValueMismatch::Type)
        );
        assert_eq!(
            value_matches(&Value::FixedBytes(Box::from([0u8; 16])), &ValueType::Id128),
            Err(ValueMismatch::Type)
        );

        let dense =
            Value::IntervalF64(Interval::<F64>::new(F64::ZERO, F64::from(1.0)).expect("checked"));
        assert_eq!(
            value_matches(
                &dense,
                &ValueType::Interval {
                    element: IntervalElement::F64
                }
            ),
            Ok(())
        );
        // No implicit integer/float interval coercion in either direction.
        assert_eq!(
            value_matches(
                &dense,
                &ValueType::Interval {
                    element: IntervalElement::U64
                }
            ),
            Err(ValueMismatch::Type)
        );
        assert_eq!(
            value_matches(
                &Value::IntervalU64(Interval::<u64>::new(0, 1).expect("checked")),
                &ValueType::Interval {
                    element: IntervalElement::F64
                }
            ),
            Err(ValueMismatch::Type)
        );
    }

    /// A fixed-width float interval has no descriptor spelling: the element
    /// enum is discrete by type, and the width/element vocabulary stays
    /// exact. The refusal is structural, not a validation branch.
    #[test]
    fn fixed_interval_elements_are_discrete_by_construction() {
        for element in [FixedIntervalElement::U64, FixedIntervalElement::I64] {
            let ty = ValueType::FixedInterval { element, width: 8 };
            assert!(ty.is_interval());
            assert!(ty.is_discrete_interval());
            assert_ne!(ty.interval_element(), Some(IntervalElement::F64));
        }
        let dense = ValueType::Interval {
            element: IntervalElement::F64,
        };
        assert!(dense.is_interval());
        assert!(!dense.is_discrete_interval());
        assert_eq!(dense.width(), 16);
        assert_eq!(ValueType::Id128.width(), 16);
    }
}
