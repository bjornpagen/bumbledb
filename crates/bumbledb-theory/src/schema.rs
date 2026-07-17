//! The schema as declared: descriptors, the shared value/type vocabulary,
//! and the ids the declaration order mints
//! (`docs/architecture/10-data-model.md`,
//! `docs/architecture/30-dependencies.md`).
//!
//! This is the theory half of the schema surface: plain data a host (or
//! the `schema!` macro's expansion) constructs, and the pure judgments
//! over it — [`SchemaDescriptor::materialized_statements`] and
//! [`value_matches`]. The admission boundary stays engine-side: the only
//! way to obtain the sealed `Schema` witness is the engine's
//! `SchemaDescriptor::validate`, and everything downstream trusts it.
//!
//! # What a descriptor can hold — the parity roster (normative)
//!
//! [`SchemaDescriptor`] is the one schema representation: the `schema!`
//! macro emits it directly, the runtime [`spec::SchemaSpec`] path
//! (`docs/architecture/70-api.md` § the `SchemaSpec` bindings contract)
//! lowers to it, and both are judged by the same engine-side
//! `SchemaDescriptor::validate`. Exhaustively, a descriptor holds:
//!
//! - **Relations** ([`RelationDescriptor`]): name plus ordered fields.
//!   Field types ([`ValueType`]): `bool`, `u64`, `i64`, `str`
//!   ([`ValueType::String`]), `bytes<N>` ([`ValueType::FixedBytes`],
//!   N ∈ 1..=64), and the interval family ([`ValueType::Interval`] —
//!   general `interval<i64|u64>`, or fixed-width `interval<T, w>` with
//!   `width: Some(w)`). Field generation ([`Generation`]): `fresh` marks
//!   on the mint fields.
//! - **Closed relations** — both tiers through one shape:
//!   `extension: Some(rows)` marks the relation closed (the option IS
//!   the kind); each [`Row`] is a ground axiom (handle + one [`Value`]
//!   per declared payload column, bare-handle vocabularies carrying zero
//!   columns). Validation prepends the synthetic (`id`, `u64`) handle
//!   field, so sealed statement ids address [`FieldId`] 0 as the handle.
//! - **Statements** ([`StatementDescriptor`]), the three forms:
//!   `Functionality` (the FD key form `R(X) -> R` — no selection, by
//!   representation), `Containment` (`A(X | φ) <= B(Y | ψ)`; `==` is the
//!   two adjacent containments, `A <= B` first — no bidirectional
//!   variant exists), and `Cardinality` (the window
//!   `B(Y | ψ) <={lo..hi} A(X | φ)`, `hi: None` the `*` spelling).
//!   [`Side`] selections σ bind fields to literals or literal SETS
//!   ([`LiteralSet`], read disjunctively) over the one [`Value`] sum —
//!   scalar literals, interval literals, and closed-relation handles
//!   (lowered to their declaration-order row-id words).
//!
//! Nothing else exists: no field-level constraint vocabulary, no order
//! statements, no relation-kind enum, no per-statement names.

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
/// anywhere (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    Bool,
    U64,
    I64,
    String,
    /// `bytes<N>`: exactly `len` raw bytes, identity-shaped — stored
    /// inline in the fact, word-padded, never interned (*intern what
    /// repeats; inline what identifies* —
    /// `docs/architecture/10-data-model.md`). The length is part of the
    /// type: `bytes<16>` and `bytes<32>` are different types, and the
    /// fingerprint feeds the length (a width change is a new theory).
    /// `len` is validated to `1..=64` at declaration.
    FixedBytes {
        len: u16,
    },
    /// A half-open `[start, end)` over the element domain, strictly
    /// `start < end` — a finite set of points, written as its bounds
    /// (`docs/architecture/10-data-model.md`). `width` selects within
    /// the interval FAMILY: `None` is the general type (16-byte
    /// `start ‖ end` encoding, rays representable); `Some(w)` is
    /// `interval<E, w>` — the width is the type (the `bytes<N>`
    /// precedent), the encoding stores ONLY the start (8 bytes; the
    /// end derives as `start + w`), wide values are unrepresentable,
    /// and the Q2 bound `start + w < MAX_END` bars ray-hood by
    /// construction (`lean/Bumbledb/Values.lean: FixedU64.not_ray`).
    /// Admitted under the admission rule: a type parameter is
    /// admitted iff it changes the encoding — `w` does; a parameter
    /// that merely checks is a CHECK constraint, refused
    /// (`docs/architecture/10-data-model.md` § the admission rule).
    /// The width is a fingerprint input — a width change is a new
    /// theory. `w ≥ 1`, validated at declaration.
    Interval {
        element: IntervalElement,
        width: Option<u64>,
    },
}

/// Field generation: a storage behavior, not a type
/// (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Generation {
    /// Ordinary field: the application supplies every value.
    None,
    /// The database mints values: monotonic per (relation, field), never
    /// re-issuing a value observable in a committed state. Must be `U64`.
    Fresh,
}

/// One field: name + structural type + generation attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: Box<str>,
    pub value_type: ValueType,
    pub generation: Generation,
}

/// How a [`Value`] failed to match an expected [`ValueType`] — the shared
/// vocabulary of the checking boundaries (query literals, bound params,
/// dynamic facts, statement selections).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueMismatch {
    /// Wrong structural kind.
    Type,
    /// `Value::String` bytes are not UTF-8 (the type's contract).
    Utf8,
}

/// The one `Value` ↔ `ValueType` compatibility check (kind and String UTF-8)
/// — IR validation, bind-time, the dynamic write
/// path, and selection validation all call this so the rules cannot drift
/// apart. Note the membership rule is *not* here: an element-typed value
/// against an `Interval` field is a kind mismatch to this check, and the
/// IR validation boundary owns that bivalence (the engine's
/// `ir::validate`, the bivalent-anchor resolution).
///
/// # Errors
///
/// The [`ValueMismatch`] arm that names the failure: `Type` on a wrong
/// structural kind (including the width rules), `Utf8` on non-UTF-8
/// `String` bytes.
pub fn value_matches(value: &Value, expected: &ValueType) -> Result<(), ValueMismatch> {
    match (value, expected) {
        (Value::Bool(_), ValueType::Bool)
        | (Value::U64(_), ValueType::U64)
        | (Value::I64(_), ValueType::I64) => Ok(()),
        // The interval family: the general type takes any checked
        // interval of its element; a fixed-width type takes exactly the
        // declared width, never a ray (Q2: `start + w < MAX_END` — the
        // ray end IS `MAX_END`, so `!is_ray()` is the bound;
        // `lean/Bumbledb/Values.lean: FixedU64.not_ray`). A wide or
        // narrow value is a kind mismatch — the width is the type.
        (
            Value::IntervalU64(interval),
            ValueType::Interval {
                element: IntervalElement::U64,
                width,
            },
        ) => match width {
            None => Ok(()),
            Some(w) if interval.end() - interval.start() == *w && !interval.is_ray() => Ok(()),
            Some(_) => Err(ValueMismatch::Type),
        },
        (
            Value::IntervalI64(interval),
            ValueType::Interval {
                element: IntervalElement::I64,
                width,
            },
        ) => match width {
            None => Ok(()),
            Some(w) if interval.end().abs_diff(interval.start()) == *w && !interval.is_ray() => {
                Ok(())
            }
            Some(_) => Err(ValueMismatch::Type),
        },
        // The length is the type: a bytes<N> literal of any other width
        // is a kind mismatch.
        (Value::FixedBytes(raw), ValueType::FixedBytes { len }) => {
            if raw.len() == usize::from(*len) {
                Ok(())
            } else {
                Err(ValueMismatch::Type)
            }
        }
        (Value::String(raw), ValueType::String) => {
            if std::str::from_utf8(raw).is_ok() {
                Ok(())
            } else {
                Err(ValueMismatch::Utf8)
            }
        }
        _ => Err(ValueMismatch::Type),
    }
}

/// One σ binding's literal set — the disjunctive selection fragment
/// (`lean/Bumbledb/Schema.lean: Selection`): the selected field's value is
/// a MEMBER of the spelled set, bindings read conjunctively. The singleton
/// arm is today's equality by representation
/// (`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`) and
/// stays zero-cost — no per-literal indirection on the one-literal path.
/// The `Many` arm's canonical form is sorted and duplicate-free with at
/// least two literals; validation canonicalizes the order and rejects the
/// degenerate spellings (`docs/architecture/30-dependencies.md`
/// § validation roster).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralSet {
    /// One literal: the equality binding — the whole accepted σ fragment
    /// before the disjunctive extension, unchanged in meaning.
    One(Value),
    /// Two or more literals, read disjunctively. The sets are first-class,
    /// not per-literal sugar: a window over a disjunctive selection is not
    /// any conjunction of per-literal windows
    /// (`lean/Bumbledb/Countermodels.lean:
    /// disjunctive_window_not_literal_conjunction`).
    Many(Box<[Value]>),
}

impl LiteralSet {
    /// The literals, one or more — the `One` arm borrows in place
    /// (`std::slice::from_ref`), so the singleton path allocates and
    /// indirects nothing.
    #[must_use]
    pub fn literals(&self) -> &[Value] {
        match self {
            Self::One(literal) => std::slice::from_ref(literal),
            Self::Many(literals) => literals,
        }
    }

    /// The singleton reading, when this binding is today's equality.
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
/// (`docs/architecture/30-dependencies.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    pub relation: RelationId,
    /// π — ordered, the statement's written order.
    pub projection: Box<[FieldId]>,
    /// σ — a set of (field, literal-set) bindings read conjunctively,
    /// each binding a disjunction over its spelled set; empty =
    /// unselected. Literals are the one shared [`Value`] sum
    /// (`docs/architecture/30-dependencies.md` — any type's literal binds
    /// in σ; dependencies and queries share one representation).
    pub selection: Box<[(FieldId, LiteralSet)]>,
}

/// One dependency statement: a judgment about queries
/// (`docs/architecture/30-dependencies.md`). Statements are anonymous —
/// their identity is their materialized-order [`StatementId`]. There is no
/// bidirectional variant: `==` is lowered to two `Containment` statements
/// with the sides swapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementDescriptor {
    /// `R(X) -> R`: πX is injective on R. X is ordered (the order defines
    /// the determinant key), non-empty, duplicate-free.
    Functionality {
        relation: RelationId,
        projection: Box<[FieldId]>,
    },
    /// `A(X | φ) <= B(Y | ψ)`: πX(σφ(A)) ⊆ πY(σψ(B)) as sets of tuples.
    Containment { source: Side, target: Side },
    /// `B(Y | ψ) <={lo..hi} A(X | φ)` (B-family, target-left — the left
    /// side is `target`): the cardinality window — per selected target
    /// fact, the count of selected source facts sharing its projected
    /// tuple lies in the window
    /// (`lean/Bumbledb/Cardinality.lean: CardinalityWindow`;
    /// `lean/Bumbledb/Schema.lean: Statement.cardinality`). `hi = None`
    /// is the `*` spelling — the only spelling of "no upper bound";
    /// `lo = hi` is the `{n}` exact-count spelling (`{0}` the exclusion).
    Cardinality {
        source: Side,
        /// The inclusive lower count bound.
        lo: u64,
        /// The inclusive upper count bound; `None` is `*`.
        hi: Option<u64>,
        target: Side,
    },
}

/// The extension-row cap: a vocabulary larger than 256 is policy data
/// wearing a vocabulary costume, and the cap keeps every compiled word-set
/// a fixed 4×u64 bitset (`docs/architecture/10-data-model.md`, the refusal —
/// *trigger* for lifting it: a census sighting).
pub const MAX_EXTENSION_ROWS: usize = 256;

/// One ground axiom of a closed relation: the handle — the row's identity,
/// NOT a column — plus one value per declared intrinsic column, in
/// field-declaration order (`docs/architecture/10-data-model.md` § closed
/// relations). The row id is the declaration index, exactly the
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
/// option *is* the kind (`docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDescriptor {
    pub name: Box<str>,
    pub fields: Vec<FieldDescriptor>,
    pub extension: Option<Extension>,
}

/// The schema as declared: input to validation. Statements are
/// schema-level, between relations, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDescriptor {
    pub relations: Vec<RelationDescriptor>,
    pub statements: Vec<StatementDescriptor>,
}

impl SchemaDescriptor {
    /// The materialized statement list — the one owner of the ordering rule
    /// pinned by the fingerprint (`docs/architecture/10-data-model.md`,
    /// § fingerprint inputs): one auto-`Functionality` per `Fresh` field
    /// (relation declaration order, then field order; projection = the one
    /// fresh field), then one closed auto-key `R(id) -> R` per closed
    /// relation (declaration order; projection = the synthetic id field),
    /// then the declared statements in declaration order. Fresh before
    /// closed is a fingerprint input, pinned here and never revisited
    /// (`docs/architecture/30-dependencies.md`). [`StatementId`] = index
    /// into this list, schema-global.
    ///
    /// # Panics
    ///
    /// When a relation or field ordinal exceeds the id space (`u32`/`u16`)
    /// — impossible for a descriptor the acceptance gate admitted.
    #[must_use]
    pub fn materialized_statements(&self) -> Vec<StatementDescriptor> {
        let mut statements: Vec<StatementDescriptor> = Vec::new();
        for (rel_idx, relation) in self.relations.iter().enumerate() {
            for (field_idx, field) in relation.fields.iter().enumerate() {
                // A closed relation's sealed field list opens with the
                // synthetic id, so its declared fields sit at idx + 1.
                let sealed_idx = field_idx + usize::from(relation.extension.is_some());
                if field.generation == Generation::Fresh {
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
        // Closedness materializes `R(id) -> R` exactly as `fresh` does:
        // the handle is the identity, and the auto-key is the statement
        // containments target when a plain-u64 reference declares its
        // containment against a closed relation.
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
/// reads off a manifest entry or a rendered violation without matching
/// the payload-carrying enums ([`StatementDescriptor`] / the engine's
/// `Violation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatementKind {
    /// `R(X) -> R` — the FD key form.
    Functionality,
    /// `A(X | φ) <= B(Y | ψ)`.
    Containment,
    /// `B(Y | ψ) <={lo..hi} A(X | φ)` — the cardinality window.
    Cardinality,
}

impl StatementDescriptor {
    /// The form tag of this statement.
    #[must_use]
    pub const fn kind(&self) -> StatementKind {
        match self {
            Self::Functionality { .. } => StatementKind::Functionality,
            Self::Containment { .. } => StatementKind::Containment,
            Self::Cardinality { .. } => StatementKind::Cardinality,
        }
    }
}
