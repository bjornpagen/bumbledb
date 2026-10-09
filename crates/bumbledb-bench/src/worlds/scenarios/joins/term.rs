use bumbledb::{ParamId, Term};

pub(super) use crate::fixture::var;

pub(super) fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}
