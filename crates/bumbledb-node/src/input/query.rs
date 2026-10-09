//! The query IR, mirroring `bumbledb::Query` node for node: relations,
//! fields, variables and params are ordinals, exactly as the engine's
//! validator reads them.
use bumbledb::{
    AllenMask, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FieldId, FindTerm, FoldOp,
    HeadOp, HeadTerm, Interior, InteriorId, NonEmpty, NumericCast, ParamId, Query, Rec, RecRule,
    RecStep, RelationId, Rounding, Rule, ScalarExpr, SegmentOp, Term, VarId,
};
use napi_derive::napi;
use serde::Deserialize;

use super::value::Literal;

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryIn {
    pub interiors: Vec<InteriorIn>,
    pub head: Vec<HeadTermIn>,
    pub rules: Vec<RuleIn>,
    pub rec: Option<RecIn>,
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteriorIn {
    pub rules: Vec<RuleIn>,
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuleIn {
    pub finds: Vec<FindTermIn>,
    pub atoms: Vec<AtomIn>,
    pub negated: Vec<AtomIn>,
    pub conditions: Vec<ConditionIn>,
}

/// A recursive stage: nonempty base and step arms.
#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecIn {
    #[napi(ts_type = "Array<RecRuleIn>")]
    pub base: Arms<RecRuleIn>,
    #[napi(ts_type = "Array<RecStepIn>")]
    pub rec: Arms<RecStepIn>,
}

/// A nonempty list of recursive arms.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Vec<T>")]
pub struct Arms<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> TryFrom<Vec<T>> for Arms<T> {
    type Error = &'static str;
    fn try_from(mut arms: Vec<T>) -> Result<Self, &'static str> {
        if arms.is_empty() {
            return Err("expected at least one arm");
        }
        let first = arms.remove(0);
        Ok(Self { first, rest: arms })
    }
}

impl<T> Arms<T> {
    fn lower<U: From<T>>(self) -> NonEmpty<U> {
        NonEmpty::new(self.first.into(), lower(self.rest))
    }
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecRuleIn {
    pub finds: Vec<u16>,
    pub atoms: Vec<AtomIn>,
    pub conditions: Vec<ConditionIn>,
}

/// A recursive step: `selfBindings` binds the stage's own previous output.
#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecStepIn {
    pub finds: Vec<u16>,
    pub self_bindings: Vec<BindingIn>,
    pub atoms: Vec<AtomIn>,
    pub conditions: Vec<ConditionIn>,
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum AtomSourceIn {
    Edb { relation: u32 },
    Interior { interior: u32 },
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingIn {
    pub field: u16,
    pub term: TermIn,
}

#[napi(object, object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AtomIn {
    pub source: AtomSourceIn,
    pub bindings: Vec<BindingIn>,
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum TermIn {
    Var {
        var: u16,
    },
    Param {
        param: u16,
    },
    ParamSet {
        param: u16,
    },
    Literal {
        #[napi(ts_type = "ValueIn")]
        value: Literal,
    },
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum FoldOpIn {
    Sum,
    Mean,
    Min,
    Max,
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum HeadOpIn {
    Sum,
    Mean,
    Min,
    Max,
    Count,
    Pack,
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SegmentOpIn {
    Intersection,
    Difference,
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum FindTermIn {
    Var {
        var: u16,
    },
    Compute {
        expr: ScalarExprIn,
    },
    Segments {
        op: SegmentOpIn,
        left: u16,
        right: u16,
    },
    Count,
    Aggregate {
        op: FoldOpIn,
        over: u16,
    },
    Pack {
        over: u16,
    },
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum HeadTermIn {
    Var,
    Compute,
    Aggregate { op: HeadOpIn },
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum NumericCastIn {
    ToF64,
    ToF64Exact,
    ToI64Exact,
    ToU64Exact,
}

#[napi(string_enum)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum RoundingIn {
    TowardZero,
    NearestTiesAwayFromZero,
    NearestTiesToEven,
}

/// A scalar expression over rule-local variables.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ScalarExprIn {
    Var {
        var: u16,
    },
    Literal {
        #[napi(ts_type = "ValueIn")]
        value: Literal,
    },
    Negate {
        #[napi(ts_type = "ScalarExprIn")]
        expr: Box<ScalarExprIn>,
    },
    Add {
        #[napi(ts_type = "ScalarExprIn")]
        left: Box<ScalarExprIn>,
        #[napi(ts_type = "ScalarExprIn")]
        right: Box<ScalarExprIn>,
    },
    Subtract {
        #[napi(ts_type = "ScalarExprIn")]
        left: Box<ScalarExprIn>,
        #[napi(ts_type = "ScalarExprIn")]
        right: Box<ScalarExprIn>,
    },
    Multiply {
        #[napi(ts_type = "ScalarExprIn")]
        left: Box<ScalarExprIn>,
        #[napi(ts_type = "ScalarExprIn")]
        right: Box<ScalarExprIn>,
    },
    Divide {
        #[napi(ts_type = "ScalarExprIn")]
        left: Box<ScalarExprIn>,
        #[napi(ts_type = "ScalarExprIn")]
        right: Box<ScalarExprIn>,
    },
    MulDiv {
        #[napi(ts_type = "ScalarExprIn")]
        a: Box<ScalarExprIn>,
        #[napi(ts_type = "ScalarExprIn")]
        b: Box<ScalarExprIn>,
        #[napi(ts_type = "ScalarExprIn")]
        divisor: Box<ScalarExprIn>,
        rounding: RoundingIn,
    },
    Measure {
        #[napi(ts_type = "ScalarExprIn")]
        expr: Box<ScalarExprIn>,
    },
    Cast {
        cast: NumericCastIn,
        #[napi(ts_type = "ScalarExprIn")]
        expr: Box<ScalarExprIn>,
    },
    IsNaN {
        #[napi(ts_type = "ScalarExprIn")]
        expr: Box<ScalarExprIn>,
    },
    IsFinite {
        #[napi(ts_type = "ScalarExprIn")]
        expr: Box<ScalarExprIn>,
    },
}

/// A comparison operator; `Allen.mask` is a nonzero 13-bit relation set.
#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum CmpOpIn {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    PointIn,
    Allen {
        #[napi(ts_type = "number")]
        mask: Mask,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "u16")]
pub struct Mask(AllenMask);

impl TryFrom<u16> for Mask {
    type Error = &'static str;
    fn try_from(bits: u16) -> Result<Self, &'static str> {
        AllenMask::new(bits)
            .map(Self)
            .ok_or("expected a valid Allen relation mask")
    }
}

#[napi(discriminant = "kind", object_to_js = false, object_from_js = false)]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ConditionIn {
    Leaf {
        op: CmpOpIn,
        lhs: TermIn,
        rhs: TermIn,
    },
    And {
        children: Vec<ConditionIn>,
    },
    Or {
        children: Vec<ConditionIn>,
    },
}

impl From<TermIn> for Term {
    fn from(value: TermIn) -> Self {
        match value {
            TermIn::Var { var } => Self::Var(VarId(var)),
            TermIn::Param { param } => Self::Param(ParamId(param)),
            TermIn::ParamSet { param } => Self::ParamSet(ParamId(param)),
            TermIn::Literal { value } => Self::Literal(value.0),
        }
    }
}

fn bindings(bindings: Vec<BindingIn>) -> Vec<(FieldId, Term)> {
    bindings
        .into_iter()
        .map(|binding| (FieldId(binding.field), binding.term.into()))
        .collect()
}

fn vars(vars: Vec<u16>) -> Vec<VarId> {
    vars.into_iter().map(VarId).collect()
}

fn lower<T, U: From<T>>(items: Vec<T>) -> Vec<U> {
    items.into_iter().map(Into::into).collect()
}

impl From<AtomIn> for Atom {
    fn from(value: AtomIn) -> Self {
        Self {
            source: match value.source {
                AtomSourceIn::Edb { relation } => AtomSource::Edb(RelationId(relation)),
                AtomSourceIn::Interior { interior } => AtomSource::Interior(InteriorId(interior)),
            },
            bindings: bindings(value.bindings),
        }
    }
}

impl From<ScalarExprIn> for ScalarExpr {
    fn from(value: ScalarExprIn) -> Self {
        let child = |expr: Box<ScalarExprIn>| Box::new(Self::from(*expr));
        match value {
            ScalarExprIn::Var { var } => Self::Var(VarId(var)),
            ScalarExprIn::Literal { value } => Self::Literal(value.0),
            ScalarExprIn::Negate { expr } => Self::Negate(child(expr)),
            ScalarExprIn::Add { left, right } => Self::Add(child(left), child(right)),
            ScalarExprIn::Subtract { left, right } => Self::Subtract(child(left), child(right)),
            ScalarExprIn::Multiply { left, right } => Self::Multiply(child(left), child(right)),
            ScalarExprIn::Divide { left, right } => Self::Divide(child(left), child(right)),
            ScalarExprIn::MulDiv {
                a,
                b,
                divisor,
                rounding,
            } => Self::MulDiv {
                a: child(a),
                b: child(b),
                divisor: child(divisor),
                rounding: match rounding {
                    RoundingIn::TowardZero => Rounding::TowardZero,
                    RoundingIn::NearestTiesAwayFromZero => Rounding::NearestTiesAwayFromZero,
                    RoundingIn::NearestTiesToEven => Rounding::NearestTiesToEven,
                },
            },
            ScalarExprIn::Measure { expr } => Self::Measure(child(expr)),
            ScalarExprIn::Cast { cast, expr } => Self::Cast {
                kind: match cast {
                    NumericCastIn::ToF64 => NumericCast::ToF64,
                    NumericCastIn::ToF64Exact => NumericCast::ToF64Exact,
                    NumericCastIn::ToI64Exact => NumericCast::ToI64Exact,
                    NumericCastIn::ToU64Exact => NumericCast::ToU64Exact,
                },
                expr: child(expr),
            },
            ScalarExprIn::IsNaN { expr } => Self::IsNaN(child(expr)),
            ScalarExprIn::IsFinite { expr } => Self::IsFinite(child(expr)),
        }
    }
}

impl From<FindTermIn> for FindTerm {
    fn from(value: FindTermIn) -> Self {
        match value {
            FindTermIn::Var { var } => Self::Var(VarId(var)),
            FindTermIn::Compute { expr } => Self::Compute(expr.into()),
            FindTermIn::Segments { op, left, right } => Self::Segments {
                op: match op {
                    SegmentOpIn::Intersection => SegmentOp::Intersection,
                    SegmentOpIn::Difference => SegmentOp::Difference,
                },
                left: VarId(left),
                right: VarId(right),
            },
            FindTermIn::Count => Self::Count,
            FindTermIn::Aggregate { op, over } => Self::Aggregate {
                op: match op {
                    FoldOpIn::Sum => FoldOp::Sum,
                    FoldOpIn::Mean => FoldOp::Mean,
                    FoldOpIn::Min => FoldOp::Min,
                    FoldOpIn::Max => FoldOp::Max,
                },
                over: VarId(over),
            },
            FindTermIn::Pack { over } => Self::Pack { over: VarId(over) },
        }
    }
}

impl From<HeadTermIn> for HeadTerm {
    fn from(value: HeadTermIn) -> Self {
        match value {
            HeadTermIn::Var => Self::Var,
            HeadTermIn::Compute => Self::Compute,
            HeadTermIn::Aggregate { op } => Self::Aggregate(match op {
                HeadOpIn::Sum => HeadOp::Sum,
                HeadOpIn::Mean => HeadOp::Mean,
                HeadOpIn::Min => HeadOp::Min,
                HeadOpIn::Max => HeadOp::Max,
                HeadOpIn::Count => HeadOp::Count,
                HeadOpIn::Pack => HeadOp::Pack,
            }),
        }
    }
}

impl From<ConditionIn> for ConditionTree {
    fn from(value: ConditionIn) -> Self {
        match value {
            ConditionIn::Leaf { op, lhs, rhs } => Self::Leaf(Comparison {
                op: match op {
                    CmpOpIn::Eq => CmpOp::Eq,
                    CmpOpIn::Ne => CmpOp::Ne,
                    CmpOpIn::Lt => CmpOp::Lt,
                    CmpOpIn::Le => CmpOp::Le,
                    CmpOpIn::Gt => CmpOp::Gt,
                    CmpOpIn::Ge => CmpOp::Ge,
                    CmpOpIn::PointIn => CmpOp::PointIn,
                    CmpOpIn::Allen { mask } => CmpOp::Allen { mask: mask.0 },
                },
                lhs: lhs.into(),
                rhs: rhs.into(),
            }),
            ConditionIn::And { children } => Self::And(lower(children)),
            ConditionIn::Or { children } => Self::Or(lower(children)),
        }
    }
}

impl From<RuleIn> for Rule {
    fn from(value: RuleIn) -> Self {
        Self {
            finds: lower(value.finds),
            atoms: lower(value.atoms),
            negated: lower(value.negated),
            conditions: lower(value.conditions),
        }
    }
}

impl From<RecRuleIn> for RecRule {
    fn from(value: RecRuleIn) -> Self {
        Self {
            finds: vars(value.finds),
            atoms: lower(value.atoms),
            conditions: lower(value.conditions),
        }
    }
}

impl From<RecStepIn> for RecStep {
    fn from(value: RecStepIn) -> Self {
        Self {
            finds: vars(value.finds),
            self_bindings: bindings(value.self_bindings),
            atoms: lower(value.atoms),
            conditions: lower(value.conditions),
        }
    }
}

impl From<QueryIn> for Query {
    fn from(value: QueryIn) -> Self {
        Self {
            interiors: value
                .interiors
                .into_iter()
                .map(|interior| Interior {
                    rules: lower(interior.rules),
                })
                .collect(),
            head: lower(value.head),
            rules: lower(value.rules),
            rec: value.rec.map(|rec| Rec {
                base: rec.base.lower(),
                rec: rec.rec.lower(),
            }),
        }
    }
}
