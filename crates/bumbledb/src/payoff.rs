//! Exact payoff expressions read relational columns or owned numerical imports.
//! Division here is
//! rational construction, never an integer scalar calculation with rounding.
use crate::VarId;

mod import;
pub use import::{ImportedPayoff, PayoffImport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayoffExpr {
    Imported(PayoffImport),
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
        match self {
            Self::Imported(_) => [None, None],
            Self::Integer(value) => [Some(*value), None],
            Self::Ratio {
                numerator,
                denominator,
            } => [Some(*numerator), Some(*denominator)],
        }
        .into_iter()
        .flatten()
    }
}
