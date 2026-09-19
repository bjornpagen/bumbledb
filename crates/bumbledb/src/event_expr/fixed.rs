//! Lexically bound predicates are separate from row variables. The sealed
//! carrier and conservative variance analysis make iteration a typed operation.
use super::{EventExpr, EventExprError, RelationExpr, RelationProductOp, import::Context};
use crate::event::{
    BoolOp4, Capacity, Control, Error, Event, FiniteCarrier, ModalOp, Result, Space, Variance,
};
use std::sync::Arc;

#[derive(Debug)]
struct Scope {
    carrier: FiniteCarrier,
    bytes: Box<[u8]>,
}

/// Captured original full support, including designated law and parameter
/// guards. A fixed-point iteration cannot extend this sealed presentation.
/// Equality is canonical context equality, independent of manager allocation.
#[derive(Debug, Clone)]
pub struct EventScope(Arc<Scope>);

impl PartialEq for EventScope {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.bytes == other.0.bytes
    }
}
impl Eq for EventScope {}

impl EventScope {
    /// Capture the full context, never the support of a supplied predicate.
    /// # Errors
    /// Refuses encoding/counting limits, unavailable resources or cancellation.
    pub fn capture(space: &Space, control: &dyn Control) -> Result<Self> {
        let bytes = space.full().to_bytes(control)?.into_boxed_slice();
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        Ok(Self(Arc::new(Scope {
            carrier: FiniteCarrier::new(space, control)?,
            bytes,
        })))
    }

    /// Admit an already decoded full-space Event. A partial Event refuses;
    /// its use as an iteration scope would silently change complement meaning.
    /// # Errors
    /// Refuses non-full input or `capture`'s resource failures.
    pub fn from_event(event: &Event, control: &dyn Control) -> Result<Self> {
        control.checkpoint()?;
        if !event.is_full() {
            return Err(Error::PartialCarrier);
        }
        Self::capture(&event.space(), control)
    }

    /// Import canonical full-space BEVT, checking the law/guards and atom bound.
    /// # Errors
    /// Refuses malformed/partial input, resources or cancellation.
    pub fn from_bytes(bytes: &[u8], control: &dyn Control) -> Result<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Capacity(Capacity::DescriptorBytes));
        }
        Self::from_event(&Event::from_bytes(bytes, control)?, control)
    }

    #[must_use]
    pub fn carrier(&self) -> &FiniteCarrier {
        &self.0.carrier
    }
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0.bytes
    }
    pub(super) fn context(&self) -> Context<'_> {
        Context {
            marker: self.bytes(),
            space: self.carrier().space(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedPointKind {
    Least,
    Greatest,
}

/// De Bruijn depth: zero refers to the closest enclosing fixed-point binder.
/// It is never a database variable, parameter, or computed output alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PredicateDepth(pub u16);

fn combine(left: Variance, right: Variance) -> Variance {
    left.apply(BoolOp4::OR, right)
}

pub(super) fn require_monotone(body: &EventExpr) -> std::result::Result<(), EventExprError> {
    if matches!(
        variance(body, 0),
        Variance::Independent | Variance::Increasing
    ) {
        Ok(())
    } else {
        Err(EventExprError::NonMonotoneFixedPoint)
    }
}

fn variance(expr: &EventExpr, depth: usize) -> Variance {
    match expr {
        EventExpr::Var(_) | EventExpr::Empty(_) | EventExpr::Full(_) => Variance::Independent,
        EventExpr::Bound(bound) => {
            if usize::from(bound.0) == depth {
                Variance::Increasing
            } else {
                Variance::Independent
            }
        }
        EventExpr::FixedPoint { body, .. } => variance(body, depth + 1),
        EventExpr::Not(value) => variance(value, depth).complement(),
        EventExpr::Apply { op, left, right } => {
            variance(left, depth).apply(*op, variance(right, depth))
        }
        EventExpr::Ite {
            condition,
            high,
            low,
        } => variance(condition, depth).ite(variance(high, depth), variance(low, depth)),
        EventExpr::Map { input, .. } => variance(input, depth),
        EventExpr::Relation { relation, .. } => relation_variance(relation, depth),
        EventExpr::Modal {
            operation,
            relation,
            input,
        } => {
            let r = relation_variance(relation, depth);
            let e = variance(input, depth);
            match operation {
                ModalOp::May | ModalOp::Post => combine(r, e),
                ModalOp::All => combine(r, e.complement()).complement(),
                ModalOp::Must => combine(r, combine(r, e.complement()).complement()),
            }
        }
        EventExpr::Cardinality {
            minimum,
            maximum,
            events,
        } => {
            let v = events
                .iter()
                .fold(Variance::Independent, |v, e| combine(v, variance(e, depth)));
            if v == Variance::Independent
                || minimum > maximum
                || *minimum > events.len()
                || (*minimum == 0 && *maximum >= events.len())
            {
                Variance::Independent
            } else if *maximum >= events.len() {
                v
            } else if *minimum == 0 {
                v.complement()
            } else {
                Variance::Mixed
            }
        }
    }
}

fn relation_variance(expr: &RelationExpr, depth: usize) -> Variance {
    match expr {
        RelationExpr::Bind { region, .. } => variance(region, depth),
        RelationExpr::Test { predicate, .. } => variance(predicate, depth),
        RelationExpr::Identity { .. } => Variance::Independent,
        RelationExpr::Not(value) => relation_variance(value, depth).complement(),
        RelationExpr::Converse(value)
        | RelationExpr::Star {
            relation: value, ..
        } => relation_variance(value, depth),
        RelationExpr::Apply { op, left, right } => {
            relation_variance(left, depth).apply(*op, relation_variance(right, depth))
        }
        RelationExpr::Product {
            operation,
            left,
            right,
            ..
        } => {
            let left = relation_variance(left, depth);
            let right = relation_variance(right, depth);
            match operation {
                RelationProductOp::Compose => combine(left, right),
                RelationProductOp::LeftResidual => combine(left, right.complement()).complement(),
                RelationProductOp::RightResidual => combine(left.complement(), right).complement(),
            }
        }
    }
}
