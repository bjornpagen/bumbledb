use super::ValidatedQuery;
use crate::ir::ParamId;
use crate::schema::ValueType;
use std::collections::BTreeSet;

impl ValidatedQuery {
    /// Every param with its resolved type, in id order (bind-time checking,
    /// The 30-execution doc). A set param's type is its *element* type.
    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types.iter().map(|(p, t)| (*p, t))
    }

    /// The params bound as sets (`Term::ParamSet`) — bind-time expects a
    /// slice of values of the element type for each.
    #[must_use]
    pub fn set_params(&self) -> &BTreeSet<ParamId> {
        &self.set_params
    }
}
