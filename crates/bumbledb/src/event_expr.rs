//! Pure-data Event outputs. Variables name body bindings, never head aliases.
//! Shape admission precedes cloning/lowering; runtime scope admission precedes
//! every algebraic shortcut, including constant truth functions and ITE.
use crate::{VarId, event::BoolOp4};

/// Stable logical identity of one participating context refusal. Canonical
/// bytes contain source/support/value identity, never resident arena keys.
/// `stage = None` denotes the main head; `Some(i)` denotes interior i.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventOperandFault {
    pub stage: Option<usize>,
    pub rule: u16,
    pub find: usize,
    pub operand: usize,
    pub variable: VarId,
    pub category: EventFaultCategory,
    pub expected_space: Box<[u8]>,
    pub offending_value: Box<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventFaultCategory {
    SpaceMismatch,
}

/// One region of a shared, explicitly admitted world space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventExpr {
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
    /// Inclusive number of true roster positions. Equal values in different
    /// positions still count separately. The roster must contain an anchor.
    Cardinality {
        minimum: usize,
        maximum: usize,
        events: Vec<Self>,
    },
}

/// A Boolean statement about whole regions, suitable for a later-stage filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventTest {
    IsEmpty(EventExpr),
    IsFull(EventExpr),
    Subset(EventExpr, EventExpr),
    Equal(EventExpr, EventExpr),
    Disjoint(EventExpr, EventExpr),
    Covers(EventExpr, EventExpr),
}

/// Static shape/type refusals; dynamic context faults retain canonical values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventExprError {
    TooDeep,
    TooLarge,
    EmptyRoster,
    NotEvent(VarId),
}

impl std::fmt::Display for EventExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event expression: {self:?}")
    }
}

impl EventExpr {
    /// Syntactic leaf occurrences, in written order, including anchors and
    /// repeated variables. This is also the diagnostic operand numbering.
    pub fn variables(&self) -> impl Iterator<Item = VarId> + '_ {
        variables(vec![self])
    }

    /// Bound recursion and roster extent before normalization clones the IR.
    /// # Errors
    /// Excessive nesting/nodes or a roster without a scope anchor.
    pub fn validate_shape(&self) -> Result<(), EventExprError> {
        validate_shape(vec![self])
    }
}

impl EventTest {
    pub(crate) fn roots(&self) -> Vec<&EventExpr> {
        match self {
            Self::IsEmpty(a) | Self::IsFull(a) => vec![a],
            Self::Subset(a, b) | Self::Equal(a, b) | Self::Disjoint(a, b) | Self::Covers(a, b) => {
                vec![a, b]
            }
        }
    }

    pub fn variables(&self) -> impl Iterator<Item = VarId> + '_ {
        let mut roots = self.roots();
        roots.reverse();
        variables(roots)
    }

    /// # Errors
    /// As [`EventExpr::validate_shape`], bounding the whole test.
    pub fn validate_shape(&self) -> Result<(), EventExprError> {
        validate_shape(self.roots())
    }
}

fn children(expr: &EventExpr) -> Vec<&EventExpr> {
    match expr {
        EventExpr::Var(_) | EventExpr::Empty(_) | EventExpr::Full(_) => vec![],
        EventExpr::Not(value) => vec![value],
        EventExpr::Apply { left, right, .. } => vec![left, right],
        EventExpr::Ite {
            condition,
            high,
            low,
        } => vec![condition, high, low],
        EventExpr::Cardinality { events, .. } => events.iter().collect(),
    }
}

fn variables(mut pending: Vec<&EventExpr>) -> impl Iterator<Item = VarId> {
    std::iter::from_fn(move || {
        while let Some(expr) = pending.pop() {
            match expr {
                EventExpr::Var(var) | EventExpr::Empty(var) | EventExpr::Full(var) => {
                    return Some(*var);
                }
                _ => pending.extend(children(expr).into_iter().rev()),
            }
        }
        None
    })
}

fn validate_shape(roots: Vec<&EventExpr>) -> Result<(), EventExprError> {
    let mut pending: Vec<_> = roots.into_iter().map(|root| (root, 1)).collect();
    let mut nodes = 0;
    while let Some((expr, depth)) = pending.pop() {
        nodes += 1;
        if depth > 128 {
            return Err(EventExprError::TooDeep);
        }
        if nodes + pending.len() > 4096 {
            return Err(EventExprError::TooLarge);
        }
        if let EventExpr::Cardinality { events, .. } = expr {
            if events.is_empty() {
                return Err(EventExprError::EmptyRoster);
            }
            if events.len() > 4096 {
                return Err(EventExprError::TooLarge);
            }
        }
        pending.extend(children(expr).into_iter().map(|child| (child, depth + 1)));
    }
    Ok(())
}
