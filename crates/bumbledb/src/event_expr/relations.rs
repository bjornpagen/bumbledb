//! Pure-data typed relation programs. Every product/workspace identity is authored.
use super::{
    EventExpr, EventExprError, EventImport,
    import::{Context, Role},
    validate_contexts,
};
use crate::event::BoolOp4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationViewOp {
    Region,
    Domain,
    Range,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationProductOp {
    Compose,
    LeftResidual,
    RightResidual,
}

impl RelationProductOp {
    /// Left input, right input and result in the captured ST/TU/SU plan.
    pub(crate) const fn positions(self) -> [usize; 3] {
        match self {
            Self::Compose => [0, 1, 2],
            Self::LeftResidual => [0, 2, 1],
            Self::RightResidual => [2, 1, 0],
        }
    }
}

/// A relation expression has explicit endpoint roles over an ordinary Event.
/// Inspectable imports retain full products; composition/residuals retain one
/// shared workspace. No owner or source name is inferred from its operands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationExpr {
    Bind {
        faces: EventImport,
        region: Box<EventExpr>,
    },
    Identity {
        faces: EventImport,
    },
    Test {
        faces: EventImport,
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
        plan: EventImport,
        left: Box<Self>,
        right: Box<Self>,
    },
    Star {
        plan: EventImport,
        relation: Box<Self>,
    },
}

impl RelationExpr {
    pub(crate) fn role(&self) -> Option<Role<'_>> {
        match self {
            Self::Bind { faces, .. } | Self::Identity { faces } | Self::Test { faces, .. } => {
                faces.faces()
            }
            Self::Not(value) | Self::Apply { left: value, .. } => value.role(),
            Self::Converse(value) => value.role().map(Role::converse),
            Self::Product {
                operation, plan, ..
            } => plan
                .product()
                .map(|(_, roles)| roles[operation.positions()[2]]),
            Self::Star { plan, .. } => plan.product().map(|(_, roles)| roles[2]),
        }
    }

    pub(crate) fn validate_contexts(&self, bounds: &[Context<'_>]) -> Result<(), EventExprError> {
        let role = self.role().ok_or(EventExprError::ImportKind)?;
        match self {
            Self::Bind { region, .. } => {
                validate_contexts(region, Some(role.region().marker), bounds)
            }
            Self::Identity { .. } => require_endo(role),
            Self::Test { predicate, .. } => {
                require_endo(role)?;
                validate_contexts(predicate, Some(role.input().marker), bounds)
            }
            Self::Not(value) | Self::Converse(value) => value.validate_contexts(bounds),
            Self::Star { plan, relation } => {
                require_endo(role)?;
                let (_, roles) = plan.product().ok_or(EventExprError::ImportKind)?;
                require_roles(role, Some(roles[0]))?;
                require_roles(role, Some(roles[1]))?;
                require_roles(role, relation.role())?;
                relation.validate_contexts(bounds)
            }
            Self::Apply { left, right, .. } => {
                require_roles(role, right.role())?;
                left.validate_contexts(bounds)?;
                right.validate_contexts(bounds)
            }
            Self::Product {
                operation,
                plan,
                left,
                right,
            } => {
                let (_, roles) = plan.product().ok_or(EventExprError::ImportKind)?;
                let [l, r, _] = operation.positions();
                require_roles(roles[l], left.role())?;
                require_roles(roles[r], right.role())?;
                left.validate_contexts(bounds)?;
                right.validate_contexts(bounds)
            }
        }
    }
}

fn require_roles(expected: Role<'_>, actual: Option<Role<'_>>) -> Result<(), EventExprError> {
    if expected.same(actual.ok_or(EventExprError::ImportKind)?) {
        Ok(())
    } else {
        Err(EventExprError::IncompatibleRoles)
    }
}

fn require_endo(role: Role<'_>) -> Result<(), EventExprError> {
    if role.endorelation() {
        Ok(())
    } else {
        Err(EventExprError::IncompatibleRoles)
    }
}
