//! The wire-tag tables: one declarative table per mirrored core enum.
//! A new core variant breaks compile here; payload marshaling stays in
//! `marshal.rs`.
use bumbledb::schema::spec::{
    BoundSpec, CapacityWindowSpec, LiteralSetSpec, LiteralSpec, StatementSpec, WeightSpec,
};
use bumbledb::schema::{IntervalElement, ValueType};
use bumbledb::{
    AtomSource, CmpOp, ConditionTree, Direction, ErrorFamily, FindTerm, HeadOp, HeadTerm, Query,
    StatementKind, Term, Value,
};

use crate::marshal::OwnedParam;

macro_rules! wire_tags {
    ($(#[$doc:meta])* mod $mod_name:ident for unit $enum_ty:ty {
        $($const_name:ident : $variant:path => $tag:literal),+ $(,)?
    }) => {
        $(#[$doc])*
        pub(crate) mod $mod_name {
            #[allow(unused_imports)]
            use super::*;

            $(pub(crate) const $const_name: &str = $tag;)+

            /// The EXHAUSTIVE variant → wire-tag map. Deliberately no
            /// wildcard: a new core variant fails compile HERE, forcing
            /// the wire decision to land with the variant.
            #[allow(dead_code)]
            pub(crate) fn tag(value: &$enum_ty) -> &'static str {
                match value {
                    $($variant => $const_name,)+
                }
            }

            /// The IN direction, generated from the same rows — a unit
            /// variant's pattern IS its constructor, so the parsers stop
            /// hand-mirroring this table (the old admitted drift gap:
            /// a new variant satisfied `tag()` and still refused at
            /// runtime as an "unknown … kind").
            #[allow(dead_code)]
            pub(crate) fn parse(tag: &str) -> Option<$enum_ty> {
                match tag {
                    $($tag => Some($variant),)+
                    _ => None,
                }
            }

            /// Every wire tag, core declaration order — the `tags.json`
            /// golden reads this (test-only consumption, hence the allow).
            #[allow(dead_code)]
            pub(crate) const TAGS: &[&str] = &[$($const_name),+];
        }
    };
    ($(#[$doc:meta])* mod $mod_name:ident for $enum_ty:ty {
        $($const_name:ident : $pat:pat => $tag:literal),+ $(,)?
    }) => {
        $(#[$doc])*
        pub(crate) mod $mod_name {
            #[allow(unused_imports)]
            use super::*;

            $(pub(crate) const $const_name: &str = $tag;)+

            /// The EXHAUSTIVE variant → wire-tag map. Deliberately no
            /// wildcard: a new core variant fails compile HERE, forcing
            /// the wire decision to land with the variant.
            #[allow(dead_code)]
            pub(crate) fn tag(value: &$enum_ty) -> &'static str {
                match value {
                    $($pat => $const_name,)+
                }
            }

            /// Every wire tag, core declaration order — the `tags.json`
            /// golden reads this (test-only consumption, hence the allow).
            #[allow(dead_code)]
            pub(crate) const TAGS: &[&str] = &[$($const_name),+];
        }
    };
}

wire_tags! {
    /// `bumbledb::Value` — the tagged value lane (`tagged_value`).
    mod value for Value {
        BOOL: Value::Bool(_) => "bool",
        U64: Value::U64(_) => "u64",
        I64: Value::I64(_) => "i64",
        F64: Value::F64(_) => "f64",
        ID128: Value::Id128(_) => "id128",
        STRING: Value::String(_) => "string",
        FIXED_BYTES: Value::FixedBytes(_) => "fixedBytes",
        INTERVAL_U64: Value::IntervalU64(_) => "intervalU64",
        INTERVAL_I64: Value::IntervalI64(_) => "intervalI64",
        INTERVAL_F64: Value::IntervalF64(_) => "intervalF64",
    }
}

wire_tags! {
    /// `bumbledb::schema::ValueType` — one table for BOTH directions
    /// (`value_type_in` parses it, `value_type_out` renders it): the old
    /// in/out twin tables are one datum.
    mod value_type for ValueType {
        BOOL: ValueType::Bool => "bool",
        U64: ValueType::U64 => "u64",
        I64: ValueType::I64 => "i64",
        F64: ValueType::F64 => "f64",
        ID128: ValueType::Id128 => "id128",
        STRING: ValueType::String => "string",
        FIXED_BYTES: ValueType::FixedBytes { .. } => "fixedBytes",
        INTERVAL: ValueType::Interval { .. } | ValueType::FixedInterval { .. } => "interval",
    }
}

wire_tags! {
    /// `bumbledb::schema::IntervalElement` — the interval family's element
    /// domain (nested in `value_type` both directions; `parse` is the IN
    /// direction).
    mod interval_element for unit IntervalElement {
        U64: IntervalElement::U64 => "u64",
        I64: IntervalElement::I64 => "i64",
        F64: IntervalElement::F64 => "f64",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::LiteralSpec` (`literal_in`).
    mod literal for LiteralSpec {
        HANDLE: LiteralSpec::Handle(_) => "handle",
        VALUE: LiteralSpec::Value(_) => "value",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::LiteralSetSpec` (`literal_set_in`).
    mod literal_set for LiteralSetSpec {
        ONE: LiteralSetSpec::One(_) => "one",
        MANY: LiteralSetSpec::Many(_) => "many",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::CapacityWindowSpec` (`capacity_window_in`).
    mod capacity_window for CapacityWindowSpec {
        EXACT: CapacityWindowSpec::Exact(_) => "exact",
        RANGE: CapacityWindowSpec::Range { .. } => "range",
        FLOOR: CapacityWindowSpec::Floor(_) => "floor",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::BoundSpec` (`capacity_bound_in`) — the
    /// capacity window's bound vocabulary: a literal, a TARGET-row field
    /// by name (the dependent bound), or a TARGET interval's measure.
    mod capacity_bound for BoundSpec {
        LIT: BoundSpec::Lit(_) => "lit",
        FIELD: BoundSpec::Field(_) => "field",
        DURATION_FIELD: BoundSpec::Duration(_) => "durationField",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::WeightSpec` (`weight_in`) — the total
    /// weight sum (C4: `unit` is a case, not an absence; the wire always
    /// carries it).
    mod weight for WeightSpec {
        UNIT: WeightSpec::Unit => "unit",
        FIELD: WeightSpec::Field(_) => "field",
        DURATION_FIELD: WeightSpec::Duration(_) => "durationField",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::StatementSpec` (`statement_in`).
    mod statement for StatementSpec {
        FD: StatementSpec::Fd { .. } => "fd",
        CONTAINMENT: StatementSpec::Containment { .. } => "containment",
        CAPACITY: StatementSpec::Capacity { .. } => "capacity",
    }
}

wire_tags! {
    /// `bumbledb::StatementKind` — the manifest/violation form tag (OUT
    /// today; `parse` generated so an IN lane never re-opens the gap).
    mod statement_kind for unit StatementKind {
        FUNCTIONALITY: StatementKind::Functionality => "functionality",
        CONTAINMENT: StatementKind::Containment => "containment",
        CAPACITY: StatementKind::Capacity => "capacity",
    }
}

wire_tags! {
    /// `bumbledb::Term` — the IR term lane (`term_in`).
    mod term for Term {
        VAR: Term::Var(_) => "var",
        PARAM: Term::Param(_) => "param",
        PARAM_SET: Term::ParamSet(_) => "paramSet",
        LITERAL: Term::Literal(_) => "literal",
    }
}

wire_tags! {
    /// `bumbledb::HeadOp` — the var-free aggregate-op vocabulary. ONE
    /// table serves both op parsers: `fold_op_in` lifts `parse`'s `HeadOp`
    /// into `FoldOp`, `head_term_in` takes `parse` bare. `FoldOp::head_op` is
    /// the engine's exhaustive `FoldOp` ↔ `HeadOp` twin for Sum/Min/Max;
    /// Count and Pack are find-term kinds, not fold ops.
    mod head_op for unit HeadOp {
        SUM: HeadOp::Sum => "sum",
        MEAN: HeadOp::Mean => "mean",
        MIN: HeadOp::Min => "min",
        MAX: HeadOp::Max => "max",
        COUNT: HeadOp::Count => "count",
        PACK: HeadOp::Pack => "pack",
    }
}

wire_tags! {
    /// `bumbledb::HeadTerm` (`head_term_in`).
    mod head_term for HeadTerm {
        VAR: HeadTerm::Var => "var",
        COMPUTE: HeadTerm::Compute => "compute",
        AGGREGATE: HeadTerm::Aggregate(_) => "aggregate",
    }
}

wire_tags! {
    /// `bumbledb::FindTerm` (`find_term_in`). The `compute` arm carries the
    /// core `ScalarExpr` payload (`scalar_expr_in` in marshal.rs) — the P03R
    /// C05 roster (`FindTerm::Compute(ScalarExpr)`) is exhaustive here.
    mod find_term for FindTerm {
        VAR: FindTerm::Var(_) => "var",
        COMPUTE: FindTerm::Compute(_) => "compute",
        COUNT: FindTerm::Count => "count",
        AGGREGATE: FindTerm::Aggregate { .. } => "aggregate",
        PACK: FindTerm::Pack { .. } => "pack",
    }
}

wire_tags! {
    /// `bumbledb::ScalarExpr` — the computed-find expression lane
    /// (`scalar_expr_in`), spelled exactly as the plan JSON grammar spells
    /// the same roster (C01/C11: one spelling, no second evaluator).
    mod scalar_expr for bumbledb::ScalarExpr {
        VAR: bumbledb::ScalarExpr::Var(_) => "var",
        LITERAL: bumbledb::ScalarExpr::Literal(_) => "literal",
        NEGATE: bumbledb::ScalarExpr::Negate(_) => "negate",
        ADD: bumbledb::ScalarExpr::Add(_, _) => "add",
        SUBTRACT: bumbledb::ScalarExpr::Subtract(_, _) => "subtract",
        MULTIPLY: bumbledb::ScalarExpr::Multiply(_, _) => "multiply",
        DIVIDE: bumbledb::ScalarExpr::Divide(_, _) => "divide",
        CAST: bumbledb::ScalarExpr::Cast { .. } => "cast",
        IS_NAN: bumbledb::ScalarExpr::IsNaN(_) => "isNaN",
        IS_FINITE: bumbledb::ScalarExpr::IsFinite(_) => "isFinite",
    }
}

wire_tags! {
    /// `bumbledb::NumericCast` — the explicit-cast vocabulary nested in the
    /// scalar-expression lane (the same four spellings as the migration
    /// plan grammar).
    mod numeric_cast for unit bumbledb::NumericCast {
        TO_F64: bumbledb::NumericCast::ToF64 => "toF64",
        TO_F64_EXACT: bumbledb::NumericCast::ToF64Exact => "toF64Exact",
        TO_I64_EXACT: bumbledb::NumericCast::ToI64Exact => "toI64Exact",
        TO_U64_EXACT: bumbledb::NumericCast::ToU64Exact => "toU64Exact",
    }
}

wire_tags! {
    /// `bumbledb::AtomSource` (`atom_in`).
    mod atom_source for AtomSource {
        EDB: AtomSource::Edb(_) => "edb",
        INTERIOR: AtomSource::Interior(_) => "interior",
    }
}

wire_tags! {
    /// `bumbledb::CmpOp` (`comparison_in`).
    mod cmp_op for CmpOp {
        EQ: CmpOp::Eq => "eq",
        NE: CmpOp::Ne => "ne",
        LT: CmpOp::Lt => "lt",
        LE: CmpOp::Le => "le",
        GT: CmpOp::Gt => "gt",
        GE: CmpOp::Ge => "ge",
        ALLEN: CmpOp::Allen { .. } => "allen",
        POINT_IN: CmpOp::PointIn => "pointIn",
    }
}

wire_tags! {
    /// `bumbledb::ConditionTree` (`condition_in`).
    mod condition for ConditionTree {
        LEAF: ConditionTree::Leaf(_) => "leaf",
        AND: ConditionTree::And(_) => "and",
        OR: ConditionTree::Or(_) => "or",
    }
}

wire_tags! {
    /// Query IR kind (`query_in`): CQ carries no rec; Reach carries rec
    /// by value. Exhaustive over engine `Query`.
    mod query for Query {
        CQ: Query { rec: None, .. } => "cq",
        REACH: Query { .. } => "reach",
    }
}

wire_tags! {
    /// `bumbledb::Direction` — the containment violation's direction (OUT
    /// today; `parse` generated so an IN lane never re-opens the gap).
    mod direction for unit Direction {
        SOURCE_UNSATISFIED: Direction::SourceUnsatisfied => "sourceUnsatisfied",
        TARGET_REQUIRED: Direction::TargetRequired => "targetRequired",
    }
}

wire_tags! {
    /// The execute-param fork (`params_in`): a scalar param IS a tagged
    /// value (its tag is the value's own), the set arm is the one extra
    /// spelling — mirroring `bumbledb::ParamArg` structurally.
    mod param for OwnedParam {
        SET: OwnedParam::Set(_) => "set",
        SCALAR: OwnedParam::Scalar(_) => "scalar",
    }
}

wire_tags! {
    /// `bumbledb::ErrorFamily` — the forced napi kind table. Engine errors
    /// cross as `{ kind, message }`; a new family arm breaks this crate.
    mod error_family for unit ErrorFamily {
        FORMAT_MISMATCH: ErrorFamily::FormatMismatch => "formatMismatch",
        SCHEMA_MISMATCH: ErrorFamily::SchemaMismatch => "schemaMismatch",
        ALREADY_INITIALIZED: ErrorFamily::AlreadyInitialized => "alreadyInitialized",
        DESTINATION_EXISTS: ErrorFamily::DestinationExists => "destinationExists",
        PUBLISHED_BUT_UNSYNCED: ErrorFamily::PublishedButUnsynced => "publishedButUnsynced",
        ENVIRONMENT_LOCKED: ErrorFamily::EnvironmentLocked => "environmentLocked",
        IO: ErrorFamily::Io => "io",
        LMDB: ErrorFamily::Lmdb => "lmdb",
        READERS_FULL: ErrorFamily::ReadersFull => "readersFull",
        SCHEMA: ErrorFamily::Schema => "schema",
        VALIDATION: ErrorFamily::Validation => "validation",
        FACT_SHAPE: ErrorFamily::FactShape => "factShape",
        CLOSED_RELATION_WRITE: ErrorFamily::ClosedRelationWrite => "closedRelationWrite",
        COMMIT_SYNC: ErrorFamily::CommitSync => "commitSync",
        TRANSACTION_POISONED: ErrorFamily::TransactionPoisoned => "transactionPoisoned",
        FOREIGN_PREPARED: ErrorFamily::ForeignPreparedQuery => "foreignPrepared",
        FOREIGN_WITNESS: ErrorFamily::ForeignWitness => "foreignWitness",
        PARAM: ErrorFamily::Param => "param",
        CAPACITY_RAY_MEASURE: ErrorFamily::CapacityRayMeasure => "capacityRayMeasure",
        DERIVED_BUDGET_EXCEEDED: ErrorFamily::DerivedBudgetExceeded => "derivedBudgetExceeded",
        OVERFLOW: ErrorFamily::Overflow => "overflow",
        SCALAR: ErrorFamily::Scalar => "scalar",
        RESULT_BYTES_OVERFLOW: ErrorFamily::ResultBytesOverflow => "resultBytesOverflow",
        CORRUPTION: ErrorFamily::Corruption => "corruption",
        STORE: ErrorFamily::Store => "store",
    }
}

// The 0.x braided log codec tag tables (`log_op`, `log_encode_refusal`)
// are deleted with the braids/codec/manifest/sidecar protocol. Successor
// history refusal identities are spelled by `bumbledb_log::identities`
// (one speller) and cross through `log.rs` lanes directly.

pub(crate) mod admission_tag {
    pub(crate) const ACCEPTED: &str = "accepted";
    pub(crate) const REJECTED: &str = "rejected";
    #[allow(dead_code)]
    pub(crate) const TAGS: &[&str] = &[ACCEPTED, REJECTED];
}

pub(crate) mod write_tag {
    pub(crate) const ACCEPTED: &str = "accepted";
    pub(crate) const REJECTED: &str = "rejected";
    pub(crate) const ABANDONED: &str = "abandoned";
    pub(crate) const MOVED: &str = "moved";
    #[allow(dead_code)]
    pub(crate) const TAGS: &[&str] = &[ACCEPTED, REJECTED, ABANDONED, MOVED];
}

pub(crate) mod open_kind {
    pub(crate) const SCHEMA_ERROR: &str = "schemaError";
    pub(crate) const NEWTYPE_MISMATCH: &str = "newtypeMismatch";
    pub(crate) const FINGERPRINT_MISMATCH: &str = "fingerprintMismatch";
    /// The managed create/publish refusal for an already-populated child
    /// path — adopted into the one table (P06R seam resolution: `OpenKind`
    /// in ts/src/native.ts carries it; the bridge speaks the same roster).
    pub(crate) const DESTINATION_EXISTS: &str = "destinationExists";
    #[allow(dead_code)]
    pub(crate) const TAGS: &[&str] = &[
        SCHEMA_ERROR,
        NEWTYPE_MISMATCH,
        FINGERPRINT_MISMATCH,
        DESTINATION_EXISTS,
    ];
}

pub(crate) mod prepare_kind {
    pub(crate) const IR_ERROR: &str = "irError";
    #[allow(dead_code)]
    pub(crate) const TAGS: &[&str] = &[IR_ERROR];
}

#[cfg(test)]
mod golden {
    use serde_json::Value as Json;

    /// Every table, key → roster, as the golden spells it. `param` lists
    /// only the wire-visible extra spelling: a scalar param crosses as its
    /// value's own tag, so `"scalar"` never appears on the wire.
    fn tables() -> Vec<(&'static str, Vec<&'static str>)> {
        let wire_param: Vec<&'static str> = super::param::TAGS
            .iter()
            .copied()
            .filter(|tag| *tag != super::param::SCALAR)
            .collect();
        vec![
            ("value", super::value::TAGS.to_vec()),
            ("valueType", super::value_type::TAGS.to_vec()),
            ("intervalElement", super::interval_element::TAGS.to_vec()),
            ("literal", super::literal::TAGS.to_vec()),
            ("literalSet", super::literal_set::TAGS.to_vec()),
            ("capacityWindow", super::capacity_window::TAGS.to_vec()),
            ("capacityBound", super::capacity_bound::TAGS.to_vec()),
            ("weight", super::weight::TAGS.to_vec()),
            ("statement", super::statement::TAGS.to_vec()),
            ("statementKind", super::statement_kind::TAGS.to_vec()),
            ("term", super::term::TAGS.to_vec()),
            ("scalarExpr", super::scalar_expr::TAGS.to_vec()),
            ("numericCast", super::numeric_cast::TAGS.to_vec()),
            ("aggregateOp", super::head_op::TAGS.to_vec()),
            ("headTerm", super::head_term::TAGS.to_vec()),
            ("findTerm", super::find_term::TAGS.to_vec()),
            ("atomSource", super::atom_source::TAGS.to_vec()),
            ("cmpOp", super::cmp_op::TAGS.to_vec()),
            ("condition", super::condition::TAGS.to_vec()),
            ("query", super::query::TAGS.to_vec()),
            ("direction", super::direction::TAGS.to_vec()),
            ("param", wire_param),
            ("errorFamily", super::error_family::TAGS.to_vec()),
            ("admissionTag", super::admission_tag::TAGS.to_vec()),
            ("writeTag", super::write_tag::TAGS.to_vec()),
            ("openKind", super::open_kind::TAGS.to_vec()),
            ("prepareKind", super::prepare_kind::TAGS.to_vec()),
        ]
    }

    #[test]
    fn tags_json_matches() {
        let committed: Json = serde_json::from_str(include_str!("../../test/fixtures/tags.json"))
            .expect("ts/test/fixtures/tags.json parses");
        let expected: Json = serde_json::Value::Object(
            tables()
                .into_iter()
                .map(|(key, tags)| {
                    (
                        key.to_string(),
                        Json::Array(tags.into_iter().map(Into::into).collect()),
                    )
                })
                .collect(),
        );
        assert_eq!(
            committed, expected,
            "ts/test/fixtures/tags.json drifted from the wire_tags! tables — \
             update the golden to match the tables (never the reverse without \
             a core-enum change)"
        );
    }
}
