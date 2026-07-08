use super::ValidatedQuery;
use crate::ir::ParamId;
use crate::schema::ValueType;

impl ValidatedQuery {
    /// Every param with its resolved type, in id order (bind-time checking,
    /// The 30-execution doc).
    pub fn param_types(&self) -> impl Iterator<Item = (ParamId, &ValueType)> {
        self.param_types.iter().map(|(p, t)| (*p, t))
    }
}
