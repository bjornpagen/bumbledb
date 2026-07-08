use bumbledb::{ParamId, Term, VarId};

pub(super) fn var(id: u16) -> Term {
    Term::Var(VarId(id))
}

pub(super) fn param(id: u16) -> Term {
    Term::Param(ParamId(id))
}
