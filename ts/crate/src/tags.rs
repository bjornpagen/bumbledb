//! The wire-tag tables — ONE declarative table per mirrored core enum
//! (cleanup-0.5.0 U3 kill 10, the `wire_tags!` macro).
//!
//! Every enum the bridge mirrors as `{ kind: "…" }` tagged objects is
//! maintained in THREE places: the core Rust enum, this crate's string
//! matches, and the TS type unions (`ts/src/native.ts` / `ts/src/spec.ts`).
//! The engine deliberately refuses serde ("a downstream binding serializes
//! it however it likes" — `bumbledb/src/schema/manifest.rs`), so the single
//! source lives HERE, bridge-side, as one table per enum emitting:
//!
//! - `tag(&E) -> &'static str` — the EXHAUSTIVE variant → tag map. No
//!   wildcard arm exists, so a NEW core variant breaks THIS crate's compile
//!   and the author lands the wire decision with the variant, instead of
//!   the old silent `other =>` drift (~90 unprotected arms across 16 match
//!   sites).
//! - one named `const` per tag — the `*_in` parsers match against these
//!   (const str patterns), so the IN direction reads the same table.
//! - `TAGS` — every tag in core declaration order; the `tags.json` golden
//!   (`ts/test/fixtures/tags.json`, verified by `golden::tags_json_matches`
//!   below) is rendered from these, and a TS test
//!   (`ts/test/wire-tags.test.ts`) pins `native.ts`/`spec.ts`'s unions
//!   against the same file — closing the TS direction too.
//!
//! Payload field marshaling stays by hand in `marshal.rs` (the
//! u64-as-bigint law rejects a full serde-JSON crossing); only the TAG
//! vocabulary is generated.

use bumbledb::schema::spec::{LiteralSetSpec, LiteralSpec, StatementSpec, WindowSpec};
use bumbledb::schema::{IntervalElement, ValueType};
use bumbledb::{
    AggOp, AtomSource, CmpOp, ConditionTree, Direction, FindTerm, HeadOp, HeadTerm, MaskTerm,
    StatementKind, Term, Value,
};

use crate::marshal::OwnedParam;

/// One wire-tag table: named tag consts (the parsers' patterns), the
/// exhaustive `tag()` map (the compile tripwire + the OUT direction), and
/// the declaration-order `TAGS` roster (the golden's source).
macro_rules! wire_tags {
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
        STRING: Value::String(_) => "string",
        FIXED_BYTES: Value::FixedBytes(_) => "fixedBytes",
        INTERVAL_U64: Value::IntervalU64(_) => "intervalU64",
        INTERVAL_I64: Value::IntervalI64(_) => "intervalI64",
        ALLEN_MASK: Value::AllenMask(_) => "allenMask",
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
        STRING: ValueType::String => "string",
        FIXED_BYTES: ValueType::FixedBytes { .. } => "fixedBytes",
        INTERVAL: ValueType::Interval { .. } => "interval",
    }
}

wire_tags! {
    /// `bumbledb::schema::IntervalElement` — the interval family's element
    /// domain (nested in `value_type` both directions).
    mod interval_element for IntervalElement {
        U64: IntervalElement::U64 => "u64",
        I64: IntervalElement::I64 => "i64",
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
    /// `bumbledb::schema::spec::WindowSpec` (`window_in`).
    mod window for WindowSpec {
        EXACT: WindowSpec::Exact(_) => "exact",
        RANGE: WindowSpec::Range { .. } => "range",
        FLOOR: WindowSpec::Floor(_) => "floor",
    }
}

wire_tags! {
    /// `bumbledb::schema::spec::StatementSpec` (`statement_in`).
    mod statement for StatementSpec {
        FD: StatementSpec::Fd { .. } => "fd",
        CONTAINMENT: StatementSpec::Containment { .. } => "containment",
        CARDINALITY: StatementSpec::Cardinality { .. } => "cardinality",
    }
}

wire_tags! {
    /// `bumbledb::StatementKind` — the manifest/violation form tag (OUT).
    mod statement_kind for StatementKind {
        FUNCTIONALITY: StatementKind::Functionality => "functionality",
        CONTAINMENT: StatementKind::Containment => "containment",
        CARDINALITY: StatementKind::Cardinality => "cardinality",
    }
}

wire_tags! {
    /// `bumbledb::Term` — the IR term lane (`term_in`).
    mod term for Term {
        VAR: Term::Var(_) => "var",
        PARAM: Term::Param(_) => "param",
        PARAM_SET: Term::ParamSet(_) => "paramSet",
        LITERAL: Term::Literal(_) => "literal",
        MEASURE: Term::Measure(_) => "measure",
    }
}

wire_tags! {
    /// `bumbledb::HeadOp` — the var-free aggregate-op vocabulary. ONE
    /// table serves both op parsers: `agg_op_in` matches these consts and
    /// attaches the Arg keys, `head_term_in` matches them bare — the old
    /// verbatim-duplicate table is dead. `AggOp::head_op` is the engine's
    /// own exhaustive `AggOp` ↔ `HeadOp` twin, so this table covers both enums.
    mod head_op for HeadOp {
        SUM: HeadOp::Sum => "sum",
        MIN: HeadOp::Min => "min",
        MAX: HeadOp::Max => "max",
        COUNT: HeadOp::Count => "count",
        COUNT_DISTINCT: HeadOp::CountDistinct => "countDistinct",
        ARG_MAX: HeadOp::ArgMax => "argMax",
        ARG_MIN: HeadOp::ArgMin => "argMin",
        PACK: HeadOp::Pack => "pack",
    }
}

/// The compile-side proof that `head_op`'s table covers `AggOp` too: the
/// engine's own `AggOp::head_op` is exhaustive over `AggOp`, so a new
/// `AggOp` variant breaks THAT compile; this map ties each `AggOp` to the
/// shared table's tag through it.
#[allow(dead_code)]
pub(crate) fn agg_op_tag(op: AggOp) -> &'static str {
    head_op::tag(&op.head_op())
}

wire_tags! {
    /// `bumbledb::HeadTerm` (`head_term_in`).
    mod head_term for HeadTerm {
        VAR: HeadTerm::Var => "var",
        AGGREGATE: HeadTerm::Aggregate(_) => "aggregate",
    }
}

wire_tags! {
    /// `bumbledb::FindTerm` (`find_term_in`).
    mod find_term for FindTerm {
        VAR: FindTerm::Var(_) => "var",
        AGGREGATE: FindTerm::Aggregate { .. } => "aggregate",
        MEASURE: FindTerm::Measure(_) => "measure",
        AGGREGATE_MEASURE: FindTerm::AggregateMeasure { .. } => "aggregateMeasure",
    }
}

wire_tags! {
    /// `bumbledb::AtomSource` (`atom_in`).
    mod atom_source for AtomSource {
        EDB: AtomSource::Edb(_) => "edb",
        IDB: AtomSource::Idb(_) => "idb",
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
    /// `bumbledb::MaskTerm` — the `allen` mask position (`comparison_in`).
    mod mask_term for MaskTerm {
        LITERAL: MaskTerm::Literal(_) => "literal",
        PARAM: MaskTerm::Param(_) => "param",
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
    /// `bumbledb::Direction` — the containment violation's direction (OUT).
    mod direction for Direction {
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

/// The golden: `ts/test/fixtures/tags.json` renders every table above, and
/// this test verifies the committed file matches — structure-compared (the
/// TS repo's formatter owns the bytes). The TS half
/// (`ts/test/wire-tags.test.ts`) pins the type unions against the same
/// file, closing the core-enum → bridge → TS chain in both directions.
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
            ("window", super::window::TAGS.to_vec()),
            ("statement", super::statement::TAGS.to_vec()),
            ("statementKind", super::statement_kind::TAGS.to_vec()),
            ("term", super::term::TAGS.to_vec()),
            ("aggregateOp", super::head_op::TAGS.to_vec()),
            ("headTerm", super::head_term::TAGS.to_vec()),
            ("findTerm", super::find_term::TAGS.to_vec()),
            ("atomSource", super::atom_source::TAGS.to_vec()),
            ("cmpOp", super::cmp_op::TAGS.to_vec()),
            ("maskTerm", super::mask_term::TAGS.to_vec()),
            ("condition", super::condition::TAGS.to_vec()),
            ("direction", super::direction::TAGS.to_vec()),
            ("param", wire_param),
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
