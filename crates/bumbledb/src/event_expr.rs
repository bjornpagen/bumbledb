//! Pure-data Event outputs. Variables name body bindings, never head aliases.
//! Shape admission precedes cloning/lowering; runtime scope admission precedes
//! every algebraic shortcut, including constant truth functions and ITE.
use crate::{
    VarId,
    event::{BoolOp4, MapOp, ModalOp, Space},
};

mod import;
use import::Context;
pub use import::EventImport;
mod relations;
pub use relations::{RelationExpr, RelationProductOp, RelationViewOp};

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
    /// A captured total readout determines both input and output contexts.
    Map {
        operation: MapOp,
        map: EventImport,
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
    ImportKind,
    ImportBudget,
    IncompatibleContexts,
    IncompatibleRoles,
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
        variables(vec![Node::Event(self)])
    }

    /// Bound recursion and roster extent before normalization clones the IR.
    /// # Errors
    /// Excessive nesting/nodes or a roster without a scope anchor.
    pub fn validate_shape(&self) -> Result<(), EventExprError> {
        validate_shape(vec![self])?;
        validate_contexts(self, self.output_marker())
    }

    pub(crate) fn output_space(&self) -> Option<&Space> {
        self.output_context().map(|context| context.space)
    }

    fn output_context(&self) -> Option<Context<'_>> {
        match self {
            Self::Map { operation, map, .. } => {
                let (_, space) = map.map_spaces(*operation)?;
                let (_, marker) = map.map_markers(*operation)?;
                Some(Context { marker, space })
            }
            Self::Relation {
                operation,
                relation,
            } => {
                let role = relation.role()?;
                Some(match operation {
                    RelationViewOp::Region => role.region(),
                    RelationViewOp::Domain => role.input(),
                    RelationViewOp::Range => role.output(),
                })
            }
            Self::Modal {
                operation,
                relation,
                ..
            } => {
                let role = relation.role()?;
                Some(if *operation == ModalOp::Post {
                    role.output()
                } else {
                    role.input()
                })
            }
            _ => children(self).into_iter().find_map(Self::output_context),
        }
    }

    fn output_marker(&self) -> Option<&[u8]> {
        self.output_context().map(|context| context.marker)
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
        variables(roots.into_iter().map(Node::Event).collect())
    }

    /// # Errors
    /// As [`EventExpr::validate_shape`], bounding the whole test.
    pub fn validate_shape(&self) -> Result<(), EventExprError> {
        let roots = self.roots();
        validate_shape(roots.clone())?;
        let expected = roots.iter().find_map(|expr| expr.output_marker());
        for root in roots {
            validate_contexts(root, expected)?;
        }
        Ok(())
    }
}

pub(crate) fn children(expr: &EventExpr) -> Vec<&EventExpr> {
    match expr {
        EventExpr::Var(_)
        | EventExpr::Empty(_)
        | EventExpr::Full(_)
        | EventExpr::Relation { .. } => vec![],
        EventExpr::Not(value) => vec![value],
        EventExpr::Apply { left, right, .. } => vec![left, right],
        EventExpr::Ite {
            condition,
            high,
            low,
        } => vec![condition, high, low],
        EventExpr::Cardinality { events, .. } => events.iter().collect(),
        EventExpr::Map { input, .. } | EventExpr::Modal { input, .. } => vec![input],
    }
}

enum Node<'a> {
    Event(&'a EventExpr),
    Relation(&'a RelationExpr),
}

impl<'a> Node<'a> {
    fn children(&self) -> Vec<Node<'a>> {
        match self {
            Self::Event(EventExpr::Relation { relation, .. }) => vec![Self::Relation(relation)],
            Self::Event(EventExpr::Modal {
                relation, input, ..
            }) => {
                vec![Self::Relation(relation), Self::Event(input)]
            }
            Self::Event(value) => children(value).into_iter().map(Self::Event).collect(),
            Self::Relation(value) => match value {
                RelationExpr::Bind { region, .. } => vec![Self::Event(region)],
                RelationExpr::Identity { .. } => vec![],
                RelationExpr::Test { predicate, .. } => vec![Self::Event(predicate)],
                RelationExpr::Not(value)
                | RelationExpr::Converse(value)
                | RelationExpr::Star {
                    relation: value, ..
                } => vec![Self::Relation(value)],
                RelationExpr::Apply { left, right, .. }
                | RelationExpr::Product { left, right, .. } => {
                    vec![Self::Relation(left), Self::Relation(right)]
                }
            },
        }
    }

    fn import(&self) -> Result<Option<&EventImport>, EventExprError> {
        let (data, valid) = match self {
            Self::Event(EventExpr::Map { map, .. }) => (map, map.map().is_some()),
            Self::Relation(
                RelationExpr::Bind { faces, .. }
                | RelationExpr::Identity { faces }
                | RelationExpr::Test { faces, .. },
            ) => (faces, faces.faces().is_some()),
            Self::Relation(
                RelationExpr::Product { plan, .. } | RelationExpr::Star { plan, .. },
            ) => (plan, plan.product().is_some()),
            _ => return Ok(None),
        };
        if valid {
            Ok(Some(data))
        } else {
            Err(EventExprError::ImportKind)
        }
    }
}

fn variables(mut pending: Vec<Node<'_>>) -> impl Iterator<Item = VarId> {
    std::iter::from_fn(move || {
        while let Some(expr) = pending.pop() {
            match expr {
                Node::Event(EventExpr::Var(var) | EventExpr::Empty(var) | EventExpr::Full(var)) => {
                    return Some(*var);
                }
                _ => pending.extend(expr.children().into_iter().rev()),
            }
        }
        None
    })
}

fn validate_shape(roots: Vec<&EventExpr>) -> Result<(), EventExprError> {
    let mut pending: Vec<_> = roots
        .into_iter()
        .map(|root| (Node::Event(root), 1))
        .collect();
    let mut nodes = 0;
    let mut import_bytes = 0usize;
    while let Some((expr, depth)) = pending.pop() {
        nodes += 1;
        if depth > 128 {
            return Err(EventExprError::TooDeep);
        }
        if nodes + pending.len() > 4096 {
            return Err(EventExprError::TooLarge);
        }
        if let Node::Event(EventExpr::Cardinality { events, .. }) = expr {
            if events.is_empty() {
                return Err(EventExprError::EmptyRoster);
            }
            if events.len() > 4096 {
                return Err(EventExprError::TooLarge);
            }
        }
        if let Some(import) = expr.import()? {
            import_bytes = import_bytes.saturating_add(import.bytes().len());
            if import_bytes > 16 * 1024 * 1024 {
                return Err(EventExprError::ImportBudget);
            }
        }
        pending.extend(expr.children().into_iter().map(|child| (child, depth + 1)));
    }
    Ok(())
}

// All ordinary Boolean nodes preserve one context. A readout supplies a typed
// boundary; its input constraints cannot leak into its output component.
fn validate_contexts(expr: &EventExpr, expected: Option<&[u8]>) -> Result<(), EventExprError> {
    if let EventExpr::Relation { relation, .. } | EventExpr::Modal { relation, .. } = expr {
        let output = expr.output_marker().ok_or(EventExprError::ImportKind)?;
        if expected.is_some_and(|expected| expected != output) {
            return Err(EventExprError::IncompatibleContexts);
        }
        relation.validate_contexts()?;
        if let EventExpr::Modal {
            operation, input, ..
        } = expr
        {
            let role = relation.role().ok_or(EventExprError::ImportKind)?;
            let needed = if *operation == ModalOp::Post {
                role.input()
            } else {
                role.output()
            };
            validate_contexts(input, Some(needed.marker))?;
        }
        return Ok(());
    }
    if let EventExpr::Map {
        operation,
        map,
        input,
    } = expr
    {
        let (source, target) = map
            .map_markers(*operation)
            .ok_or(EventExprError::ImportKind)?;
        if expected.is_some_and(|expected| expected != target) {
            return Err(EventExprError::IncompatibleContexts);
        }
        return validate_contexts(input, Some(source));
    }
    for child in children(expr) {
        validate_contexts(child, expected)?;
    }
    Ok(())
}
