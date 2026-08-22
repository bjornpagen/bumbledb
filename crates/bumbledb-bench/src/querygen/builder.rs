use bumbledb::{
    Atom, ConditionTree, FieldId, FindTerm, ParamId, Query, RelationId, Rule, Term, VarId,
};

use crate::edb::builder_relation;
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

    pub(super) fn add_atom(&mut self, relation: RelationId) -> usize {
        self.atoms.push(Atom {
            source: bumbledb::AtomSource::Edb(relation),
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

    pub(super) fn bind_var(&mut self, atom: usize, field: FieldId) -> VarId {
        let var = self.fresh_var();
        let relation = builder_relation(&self.atoms[atom]);
        self.bind(atom, field, Term::Var(var));
        self.bound.push(var);
        self.anchors.push((var, relation, field));
        var
    }

    pub(super) fn var_at(&mut self, atom: usize, field: FieldId) -> Option<VarId> {
        match self.atoms[atom].bindings.iter().find(|(f, _)| *f == field) {
            Some((_, Term::Var(var))) => Some(*var),
            Some(_) => None,
            None => Some(self.bind_var(atom, field)),
        }
    }

    pub(super) fn anchored_at(&self, positions: &[(RelationId, FieldId)]) -> Option<VarId> {
        self.anchors
            .iter()
            .find(|(_, rel, field)| positions.contains(&(*rel, *field)))
            .map(|(var, _, _)| *var)
    }

    pub(super) fn interval_valued(&self, var: VarId) -> bool {
        use crate::querygen::target::ids;
        let mut anchors = self
            .anchors
            .iter()
            .filter(|(candidate, _, _)| *candidate == var);
        anchors.all(|(_, relation, field)| {
            (*relation, *field) == (ids::MANDATE, ids::mandate::ACTIVE)
                || (*relation, *field) == (ids::TRANSFER, ids::transfer::WINDOW)
        })
    }

    pub(super) fn find_var(&mut self, var: VarId) {
        self.finds.push(FindTerm::Var(var));
    }

    pub(super) fn negated_atom(&mut self, relation: RelationId) -> usize {
        self.negated.push(Atom {
            source: bumbledb::AtomSource::Edb(relation),
            bindings: Vec::new(),
        });
        self.negated.len() - 1
    }

    pub(super) fn bind_negated(&mut self, atom: usize, field: FieldId, term: Term) {
        debug_assert!(
            !self.negated[atom].bindings.iter().any(|(f, _)| *f == field),
            "duplicate field binding"
        );
        self.negated[atom].bindings.push((field, term));
    }

    pub(super) fn into_query(self) -> Query {
        Query::single(Rule {
            finds: self.finds,
            atoms: self.atoms,
            negated: self.negated,

            conditions: self
                .conditions
                .into_iter()
                .map(ConditionTree::Leaf)
                .collect(),
        })
    }
}
