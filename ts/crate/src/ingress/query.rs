//! The owned transport tree. Its leaves carry copied bytes, not executable
//! Events or certificates. The sole parser constructs this form; exhaustive
//! admission below builds the engine grammar under the worker's `WorkContext`.
use super::{Admit, ImportInput, ValueInput};
use crate::runtime::RuntimeError;
use bumbledb::event::{BoolOp4, MapOp, ModalOp};
use bumbledb::ir::SegmentOp;
use bumbledb::scalar::{NumericCast, Rounding};
use bumbledb::work::WorkContext;
use bumbledb::{AtomSource, CmpOp, FieldId, FoldOp, HeadTerm, NonEmpty, ParamId, VarId};
use bumbledb::{RelationProductOp, RelationViewOp};

#[derive(Debug)]
pub(crate) enum Term {
    Var(VarId),
    Param(ParamId),

    ParamSet(ParamId),
    Literal(ValueInput),
}

#[derive(Debug)]
pub(crate) struct Atom {
    pub source: AtomSource,

    pub bindings: Vec<(FieldId, Term)>,
}

#[derive(Debug)]
pub(crate) struct Comparison {
    pub op: CmpOp,
    pub lhs: Term,
    pub rhs: Term,
}

#[derive(Debug)]
pub(crate) enum ConditionTree {
    Leaf(Comparison),
    And(Vec<ConditionTree>),
    Or(Vec<ConditionTree>),
}

#[derive(Debug)]
pub(crate) struct Rule {
    pub finds: Vec<FindTerm>,

    pub atoms: Vec<Atom>,

    pub negated: Vec<Atom>,

    pub conditions: Vec<ConditionTree>,
}

#[derive(Debug)]
pub(crate) struct Interior {
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub(crate) struct RecRule {
    pub finds: Vec<VarId>,
    pub atoms: Vec<Atom>,
    pub conditions: Vec<ConditionTree>,
}

#[derive(Debug)]
pub(crate) struct RecStep {
    pub finds: Vec<VarId>,
    pub self_bindings: Vec<(FieldId, Term)>,
    pub atoms: Vec<Atom>,
    pub conditions: Vec<ConditionTree>,
}

#[derive(Debug)]
pub(crate) struct Rec {
    pub base: NonEmpty<RecRule>,
    pub rec: NonEmpty<RecStep>,
}

#[derive(Debug)]
pub(crate) struct Query {
    pub interiors: Vec<Interior>,

    pub head: Vec<HeadTerm>,

    pub rules: Vec<Rule>,

    pub rec: Option<Rec>,
}

mod guards;
mod numbers;
mod payoffs;
mod predicates;
pub(crate) use guards::{GuardExpr, GuardPlan};
pub(crate) use numbers::NumberExpr;
use payoffs::PayoffAdmission;
pub(crate) use predicates::PredicateExpr;

#[derive(Debug)]
pub(crate) enum Payoff {
    Integer(VarId),
    Ratio {
        numerator: VarId,
        denominator: VarId,
    },
    Imported(Vec<u8>),
}

#[derive(Debug)]
pub(crate) enum FindTerm {
    Guard(GuardExpr),
    Number(NumberExpr),
    Predicate(PredicateExpr),
    PredicateTest {
        predicate: PredicateExpr,
        quantifier: bumbledb::PredicateQuantifier,
    },
    Expectation {
        value: Payoff,
        when: VarId,
        given: VarId,
    },
    Probability {
        event: EventExpr,
        given: EventExpr,
    },
    Var(VarId),

    Compute(ScalarExpr),

    Event(EventExpr),
    Test(EventTest),

    Segments {
        op: SegmentOp,
        left: VarId,
        right: VarId,
    },

    Count,

    Aggregate {
        op: FoldOp,
        over: VarId,
    },

    Pack {
        over: VarId,
    },
}

#[derive(Debug)]
pub(crate) enum ScalarExpr {
    Var(VarId),
    Literal(ValueInput),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>),
    MulDiv {
        a: Box<Self>,
        b: Box<Self>,
        divisor: Box<Self>,
        rounding: Rounding,
    },
    Measure(Box<Self>),
    Cast {
        kind: NumericCast,
        expr: Box<Self>,
    },
    IsNaN(Box<Self>),
    IsFinite(Box<Self>),
}

#[derive(Debug)]
pub(crate) enum EventExpr {
    Bound(bumbledb::PredicateDepth),
    FixedPoint {
        kind: bumbledb::FixedPointKind,
        scope: Vec<u8>,
        body: Box<Self>,
    },
    Var(VarId),
    Empty(VarId),
    Full(VarId),
    Not(Box<Self>),
    Apply {
        op: BoolOp4,
        left: Box<Self>,
        right: Box<Self>,
    },
    Ite {
        condition: Box<Self>,
        high: Box<Self>,
        low: Box<Self>,
    },
    Cardinality {
        minimum: usize,
        maximum: usize,
        events: Vec<Self>,
    },
    Map {
        operation: MapOp,
        map: ImportInput,
        input: Box<Self>,
    },
    Relation {
        operation: RelationViewOp,
        relation: Box<RelationExpr>,
    },
    Modal {
        operation: ModalOp,
        relation: Box<RelationExpr>,
        input: Box<Self>,
    },
}

#[derive(Debug)]
pub(crate) enum EventTest {
    IsEmpty(EventExpr),
    IsFull(EventExpr),
    Subset(EventExpr, EventExpr),
    Equal(EventExpr, EventExpr),
    Disjoint(EventExpr, EventExpr),
    Covers(EventExpr, EventExpr),
}

#[derive(Debug)]
pub(crate) enum RelationExpr {
    Bind {
        faces: ImportInput,
        region: Box<EventExpr>,
    },
    Identity {
        faces: ImportInput,
    },
    Test {
        faces: ImportInput,
        predicate: Box<EventExpr>,
    },
    Not(Box<Self>),
    Apply {
        op: BoolOp4,
        left: Box<Self>,
        right: Box<Self>,
    },
    Converse(Box<Self>),
    Product {
        operation: RelationProductOp,
        plan: ImportInput,
        left: Box<Self>,
        right: Box<Self>,
    },
    Star {
        plan: ImportInput,
        relation: Box<Self>,
    },
}

impl Admit for Atom {
    type Output = bumbledb::Atom;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::Atom {
            source: self.source,
            bindings: self.bindings.admit(work)?,
        })
    }
}

impl Admit for Comparison {
    type Output = bumbledb::Comparison;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::Comparison {
            op: self.op,
            lhs: self.lhs.admit(work)?,
            rhs: self.rhs.admit(work)?,
        })
    }
}

impl Admit for Rule {
    type Output = bumbledb::Rule;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::Rule {
            finds: self.finds.admit(work)?,
            atoms: self.atoms.admit(work)?,
            negated: self.negated.admit(work)?,
            conditions: self.conditions.admit(work)?,
        })
    }
}

impl Admit for Interior {
    type Output = bumbledb::Interior;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::Interior {
            rules: self.rules.admit(work)?,
        })
    }
}

impl Admit for RecRule {
    type Output = bumbledb::RecRule;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::RecRule {
            finds: self.finds,
            atoms: self.atoms.admit(work)?,
            conditions: self.conditions.admit(work)?,
        })
    }
}

impl Admit for RecStep {
    type Output = bumbledb::RecStep;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::RecStep {
            finds: self.finds,
            self_bindings: self.self_bindings.admit(work)?,
            atoms: self.atoms.admit(work)?,
            conditions: self.conditions.admit(work)?,
        })
    }
}

impl Admit for Rec {
    type Output = bumbledb::Rec;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        work.checkpoint()?;
        Ok(bumbledb::Rec {
            base: self.base.admit(work)?,
            rec: self.rec.admit(work)?,
        })
    }
}

impl Admit for Term {
    type Output = bumbledb::Term;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::Term as O;
        work.checkpoint()?;
        Ok(match self {
            Self::Var(v) => O::Var(v),
            Self::Param(p) => O::Param(p),
            Self::ParamSet(p) => O::ParamSet(p),
            Self::Literal(v) => O::Literal(v.admit(work)?),
        })
    }
}

impl Admit for ConditionTree {
    type Output = bumbledb::ConditionTree;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::ConditionTree as O;
        work.checkpoint()?;
        Ok(match self {
            Self::Leaf(v) => O::Leaf(v.admit(work)?),
            Self::And(v) => O::And(v.admit(work)?),
            Self::Or(v) => O::Or(v.admit(work)?),
        })
    }
}

impl Admit for FindTerm {
    type Output = bumbledb::FindTerm;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::FindTerm as O;
        work.checkpoint()?;
        Ok(match self {
            Self::Guard(v) => O::Guard(PayoffAdmission::new(work).guard(v)?),
            Self::Predicate(v) => O::Predicate(PayoffAdmission::new(work).predicate(v, 1)?),
            Self::PredicateTest {
                predicate,
                quantifier,
            } => O::PredicateTest {
                predicate: PayoffAdmission::new(work).predicate(predicate, 1)?,
                quantifier,
            },
            Self::Number(v) => O::Number(PayoffAdmission::new(work).number(v, 1)?),
            Self::Var(v) => O::Var(v),
            Self::Compute(v) => O::Compute(v.admit(work)?),
            Self::Event(v) => O::Event(v.admit(work)?),
            Self::Test(v) => O::Test(v.admit(work)?),
            Self::Expectation { value, when, given } => O::Expectation {
                value: PayoffAdmission::new(work).admit(value)?,
                when,
                given,
            },
            Self::Probability { event, given } => O::Probability {
                event: event.admit(work)?,
                given: given.admit(work)?,
            },
            Self::Segments { op, left, right } => O::Segments { op, left, right },
            Self::Count => O::Count,
            Self::Aggregate { op, over } => O::Aggregate { op, over },
            Self::Pack { over } => O::Pack { over },
        })
    }
}

impl Admit for ScalarExpr {
    type Output = bumbledb::ScalarExpr;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::ScalarExpr as O;
        work.checkpoint()?;
        Ok(match self {
            Self::Var(v) => O::Var(v),
            Self::Literal(v) => O::Literal(v.admit(work)?),
            Self::Negate(x) => O::Negate(x.admit(work)?),
            Self::Measure(x) => O::Measure(x.admit(work)?),
            Self::IsNaN(x) => O::IsNaN(x.admit(work)?),
            Self::IsFinite(x) => O::IsFinite(x.admit(work)?),
            Self::Add(a, b) => O::Add(a.admit(work)?, b.admit(work)?),
            Self::Subtract(a, b) => O::Subtract(a.admit(work)?, b.admit(work)?),
            Self::Multiply(a, b) => O::Multiply(a.admit(work)?, b.admit(work)?),
            Self::Divide(a, b) => O::Divide(a.admit(work)?, b.admit(work)?),
            Self::MulDiv {
                a,
                b,
                divisor,
                rounding,
            } => O::MulDiv {
                a: a.admit(work)?,
                b: b.admit(work)?,
                divisor: divisor.admit(work)?,
                rounding,
            },
            Self::Cast { kind, expr } => O::Cast {
                kind,
                expr: expr.admit(work)?,
            },
        })
    }
}

impl Admit for EventExpr {
    type Output = bumbledb::EventExpr;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::EventExpr as O;
        work.checkpoint()?;
        Ok(match self {
            Self::Bound(depth) => O::Bound(depth),
            Self::FixedPoint { kind, scope, body } => O::FixedPoint {
                kind,
                scope: bumbledb::EventScope::from_bytes(&scope, work)
                    .map_err(super::event_error)?,
                body: body.admit(work)?,
            },
            Self::Var(id) => O::Var(id),
            Self::Empty(id) => O::Empty(id),
            Self::Full(id) => O::Full(id),
            Self::Not(v) => O::Not(v.admit(work)?),
            Self::Apply { op, left, right } => O::Apply {
                op,
                left: left.admit(work)?,
                right: right.admit(work)?,
            },
            Self::Ite {
                condition,
                high,
                low,
            } => O::Ite {
                condition: condition.admit(work)?,
                high: high.admit(work)?,
                low: low.admit(work)?,
            },
            Self::Cardinality {
                minimum,
                maximum,
                events,
            } => O::Cardinality {
                minimum,
                maximum,
                events: events.admit(work)?,
            },
            Self::Map {
                operation,
                map,
                input,
            } => O::Map {
                operation,
                map: map.admit(work)?,
                input: input.admit(work)?,
            },
            Self::Relation {
                operation,
                relation,
            } => O::Relation {
                operation,
                relation: relation.admit(work)?,
            },
            Self::Modal {
                operation,
                relation,
                input,
            } => O::Modal {
                operation,
                relation: relation.admit(work)?,
                input: input.admit(work)?,
            },
        })
    }
}

impl Admit for EventTest {
    type Output = bumbledb::EventTest;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::EventTest as O;
        work.checkpoint()?;
        Ok(match self {
            Self::IsEmpty(e) => O::IsEmpty(e.admit(work)?),
            Self::IsFull(e) => O::IsFull(e.admit(work)?),
            Self::Subset(a, b) => O::Subset(a.admit(work)?, b.admit(work)?),
            Self::Equal(a, b) => O::Equal(a.admit(work)?, b.admit(work)?),
            Self::Disjoint(a, b) => O::Disjoint(a.admit(work)?, b.admit(work)?),
            Self::Covers(a, b) => O::Covers(a.admit(work)?, b.admit(work)?),
        })
    }
}

impl Admit for RelationExpr {
    type Output = bumbledb::RelationExpr;
    fn admit(self, work: &WorkContext) -> Result<Self::Output, RuntimeError> {
        use bumbledb::RelationExpr as O;
        work.checkpoint()?;
        Ok(match self {
            Self::Bind { faces, region } => O::Bind {
                faces: faces.admit(work)?,
                region: region.admit(work)?,
            },
            Self::Identity { faces } => O::Identity {
                faces: faces.admit(work)?,
            },
            Self::Test { faces, predicate } => O::Test {
                faces: faces.admit(work)?,
                predicate: predicate.admit(work)?,
            },
            Self::Not(v) => O::Not(v.admit(work)?),
            Self::Converse(v) => O::Converse(v.admit(work)?),
            Self::Apply { op, left, right } => O::Apply {
                op,
                left: left.admit(work)?,
                right: right.admit(work)?,
            },
            Self::Product {
                operation,
                plan,
                left,
                right,
            } => O::Product {
                operation,
                plan: plan.admit(work)?,
                left: left.admit(work)?,
                right: right.admit(work)?,
            },
            Self::Star { plan, relation } => O::Star {
                plan: plan.admit(work)?,
                relation: relation.admit(work)?,
            },
        })
    }
}
