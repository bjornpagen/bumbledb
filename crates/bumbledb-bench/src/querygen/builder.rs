use bumbledb::{Atom, FieldId, FindTerm, ParamId, Query, RelationId, Term, VarId};

use crate::querygen::Builder;

impl Builder {
    fn fresh_var(&mut self) -> VarId {
        let var = VarId(self.next_var);
        self.next_var += 1;
        var
    }

    pub(super) fn fresh_param(&mut self) -> ParamId {
        let param = ParamId(self.next_param);
        self.next_param += 1;
        param
    }

    pub(super) fn atom(&mut self, relation: RelationId) -> usize {
        self.atoms.push(Atom {
            relation,
            bindings: Vec::new(),
        });
        self.atoms.len() - 1
    }

    pub(super) fn bind(&mut self, atom: usize, field: FieldId, term: Term) {
        debug_assert!(
            !self.atoms[atom].bindings.iter().any(|(f, _)| *f == field),
            "duplicate field binding"
        );
        self.atoms[atom].bindings.push((field, term));
    }

    /// A fresh variable bound to the field, registered as a group-key
    /// candidate.
    pub(super) fn bind_var(&mut self, atom: usize, field: FieldId) -> VarId {
        let var = self.fresh_var();
        self.bind(atom, field, Term::Var(var));
        self.bound.push(var);
        var
    }

    /// The variable already bound to the field, binding a fresh one if the
    /// field is free; `None` when the field is bound to a non-variable.
    pub(super) fn var_at(&mut self, atom: usize, field: FieldId) -> Option<VarId> {
        match self.atoms[atom].bindings.iter().find(|(f, _)| *f == field) {
            Some((_, Term::Var(var))) => Some(*var),
            Some(_) => None,
            None => Some(self.bind_var(atom, field)),
        }
    }

    pub(super) fn find_var(&mut self, var: VarId) {
        self.finds.push(FindTerm::Var(var));
    }

    pub(super) fn into_query(self) -> Query {
        Query {
            finds: self.finds,
            atoms: self.atoms,
            predicates: self.predicates,
        }
    }
}
