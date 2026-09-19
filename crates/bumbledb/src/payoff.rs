//! Exact payoff expressions read ordinary relational columns. Division here is
//! rational construction, never an integer scalar calculation with rounding.
use crate::VarId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayoffExpr {
    Integer(VarId),
    Ratio {
        numerator: VarId,
        denominator: VarId,
    },
}

impl From<VarId> for PayoffExpr {
    fn from(value: VarId) -> Self {
        Self::Integer(value)
    }
}

impl PayoffExpr {
    /// Every column read by this payoff, including a zero numerator's divisor.
    pub fn variables(&self) -> impl Iterator<Item = VarId> {
        match *self {
            Self::Integer(value) => [Some(value), None],
            Self::Ratio {
                numerator,
                denominator,
            } => [Some(numerator), Some(denominator)],
        }
        .into_iter()
        .flatten()
    }
}
