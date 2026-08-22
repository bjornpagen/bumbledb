use bumbledb::ir::Rule;
use bumbledb::schema::{Generation, IntervalElement, ValueType};
use bumbledb::{Atom, AtomSource, Basic, CmpOp, FindTerm, FoldOp, Query, Term, VarId};
use std::collections::{HashMap, HashSet};

use crate::corpus_gen::{GenConfig, Rng};
use crate::querygen::construct::random_query_tagged;
use crate::querygen::target::{self, ids};
use crate::querygen::{ClosedVariant, Coverage, GenTags, GroundVariant, RulesVariant, Shape};
use crate::walk;

#[must_use]
pub fn cmp_cell_legal(op_idx: usize, type_idx: usize) -> bool {
    match op_idx {
        0 | 1 => true,
        2..=5 => type_idx < 2,
        _ => type_idx == 5,
    }
}

fn op_index(op: CmpOp) -> usize {
    match op {
        CmpOp::Eq => 0,
        CmpOp::Ne => 1,
        CmpOp::Lt => 2,
        CmpOp::Le => 3,
        CmpOp::Gt => 4,
        CmpOp::Ge => 5,
        CmpOp::Allen { .. } => 6,
        CmpOp::PointIn => 7,
    }
}

fn type_index(ty: &ValueType) -> usize {
    match ty {
        ValueType::U64 => 0,
        ValueType::I64 => 1,
        ValueType::Bool => 2,
        ValueType::String => 3,
        ValueType::FixedBytes { .. } => 4,
        ValueType::Interval { .. } | ValueType::FixedInterval { .. } => 5,
    }
}

struct Typing {
    var_types: HashMap<VarId, ValueType>,
    scalar_params: HashSet<u16>,
    var_atoms: HashMap<VarId, Vec<usize>>,
    var_pos: HashMap<VarId, (bumbledb::RelationId, bumbledb::FieldId)>,
}

fn field_type(atom: &Atom, field: bumbledb::FieldId) -> ValueType {
    match atom.source {
        AtomSource::Edb(relation) => target::schema().relation(relation).field(field).value_type,

        AtomSource::Interior(_) => ValueType::U64,
    }
}

fn typing(rule: &Rule) -> Typing {
    let mut t = Typing {
        var_types: HashMap::new(),
        scalar_params: HashSet::new(),
        var_atoms: HashMap::new(),
        var_pos: HashMap::new(),
    };

    for (atom_idx, atom) in rule.atoms.iter().enumerate() {
        for (field, term) in &atom.bindings {
            let ty = field_type(atom, *field);
            if let Term::Var(var) = term {
                t.var_atoms.entry(*var).or_default().push(atom_idx);
            }
            if ty.is_interval() {
                continue;
            }
            match term {
                Term::Var(var) => {
                    t.var_types.entry(*var).or_insert(ty);
                    if let AtomSource::Edb(relation) = atom.source {
                        t.var_pos.entry(*var).or_insert((relation, *field));
                    }
                }
                Term::Param(p) | Term::ParamSet(p) => {
                    t.scalar_params.insert(p.0);
                }
                Term::Literal(_) => {}
            }
        }
    }
    for atom in &rule.negated {
        for (field, term) in &atom.bindings {
            if field_type(atom, *field).is_interval() {
                continue;
            }
            if let Term::Param(p) | Term::ParamSet(p) = term {
                t.scalar_params.insert(p.0);
            }
        }
    }

    for atom in &rule.atoms {
        for (field, term) in &atom.bindings {
            let ty = field_type(atom, *field);
            if !ty.is_interval() {
                continue;
            }
            if let Term::Var(var) = term {
                t.var_types.entry(*var).or_insert(ty);
                if let AtomSource::Edb(relation) = atom.source {
                    t.var_pos.entry(*var).or_insert((relation, *field));
                }
            }
        }
    }
    t
}

fn element_of(ty: &ValueType) -> Option<IntervalElement> {
    ty.interval_element()
}

fn spine_violations(rule: &Rule, t: &Typing) -> u64 {
    use std::collections::BTreeSet;

    let mut eq_atoms: HashMap<VarId, BTreeSet<usize>> = HashMap::new();
    for (index, atom) in rule.atoms.iter().enumerate() {
        for (field, term) in &atom.bindings {
            let Term::Var(var) = term else { continue };
            let field_interval = field_type(atom, *field).is_interval();
            let var_interval = t.var_types.get(var).is_some_and(|ty| ty.is_interval());
            if !field_interval || var_interval {
                eq_atoms.entry(*var).or_default().insert(index);
            }
        }
    }
    let has_eq_edge = |index: usize| {
        eq_atoms
            .values()
            .any(|atoms| atoms.contains(&index) && atoms.len() >= 2)
    };
    let has_eq_selection = |atom: &Atom| {
        atom.bindings.iter().any(|(field, term)| {
            !field_type(atom, *field).is_interval()
                && matches!(term, Term::Literal(_) | Term::Param(_) | Term::ParamSet(_))
        })
    };

    let mut needs: BTreeSet<usize> = BTreeSet::new();
    for (index, atom) in rule.atoms.iter().enumerate() {
        for (field, term) in &atom.bindings {
            if !field_type(atom, *field).is_interval() {
                continue;
            }
            if let Term::Var(var) = term
                && !t.var_types.get(var).is_some_and(|ty| ty.is_interval())
            {
                needs.insert(index);
            }
        }
    }

    for comparison in rule.conditions.iter().map(super::leaf) {
        if !matches!(comparison.op, CmpOp::Allen { .. } | CmpOp::PointIn) {
            continue;
        }
        if let (Term::Var(lhs), Term::Var(rhs)) = (&comparison.lhs, &comparison.rhs) {
            if t.var_atoms[lhs]
                .iter()
                .any(|a| t.var_atoms[rhs].contains(a))
            {
                continue;
            }
            for var in [lhs, rhs] {
                if t.var_types.get(var).is_some_and(|ty| ty.is_interval()) {
                    needs.extend(t.var_atoms[var].iter().copied());
                }
            }
        }
    }
    let mut violations = needs
        .into_iter()
        .filter(|index| !has_eq_edge(*index) && !has_eq_selection(&rule.atoms[*index]))
        .count() as u64;
    for atom in &rule.negated {
        let mut memberships = 0usize;
        let mut others = 0usize;
        for (field, term) in &atom.bindings {
            let field_interval = field_type(atom, *field).is_interval();
            let is_membership = field_interval
                && match term {
                    Term::Var(var) => !t.var_types.get(var).is_some_and(|ty| ty.is_interval()),
                    Term::Literal(bumbledb::Value::U64(_) | bumbledb::Value::I64(_)) => true,
                    _ => false,
                };
            if is_membership {
                memberships += 1;
            } else {
                others += 1;
            }
        }
        if memberships > 0 && others == 0 {
            violations += 1;
        }
    }
    violations
}

impl Coverage {
    fn record_shape(&mut self, shape: Shape) {
        match shape {
            Shape::KeyProbe => self.key_probe += 1,
            Shape::Star => self.star += 1,
            Shape::Chain => self.chain += 1,
            Shape::SelfJoin => self.self_join += 1,
            Shape::Gated => self.gated += 1,
            Shape::Aggregate => self.aggregate += 1,
            Shape::Membership => self.membership += 1,
            Shape::IntervalJoin => self.interval_join += 1,
            Shape::Boundary => self.boundary += 1,
            Shape::ExistenceWalk => self.existence_walk += 1,
            Shape::DuWalk => self.du_walk += 1,
            Shape::Rules => self.rules += 1,
            Shape::ClosedJoin => self.closed_join += 1,
            Shape::GroundFold => self.ground_fold += 1,
            Shape::Pack => self.pack += 1,
            Shape::Measure => self.measure += 1,
        }
    }

    fn record_closed(&mut self, closed: Option<ClosedVariant>) {
        match closed {
            Some(ClosedVariant::Join) => self.closed_join_plain += 1,
            Some(ClosedVariant::JoinSelected) => self.closed_join_selected += 1,
            Some(ClosedVariant::HandleLiteral) => self.closed_handle_literal += 1,
            Some(ClosedVariant::HandleSet) => self.closed_handle_set += 1,
            Some(ClosedVariant::Fold) | None => {}
        }
    }

    /// near-miss refusal classes.
    fn record_ground(&mut self, ground: Option<GroundVariant>) {
        match ground {
            Some(GroundVariant::Walk) => self.ground_eliminable += 1,
            Some(GroundVariant::DuHeader) => {
                self.ground_eliminable += 1;
                self.du_header_falls += 1;
            }
            Some(GroundVariant::DuChild) => {
                self.ground_eliminable += 1;
                self.du_child_falls += 1;
            }
            Some(GroundVariant::WalkExtraField) => self.ground_extra_field += 1,
            Some(GroundVariant::DuMissingPhi) => self.ground_missing_phi += 1,
            None => {}
        }
    }

    fn record_membership(&mut self, rule: &Rule, t: &Typing) -> bool {
        let mut any = false;
        for atom in &rule.atoms {
            for (field, term) in &atom.bindings {
                let Some(element) = element_of(&field_type(atom, *field)) else {
                    continue;
                };
                let is_point = match term {
                    Term::Literal(bumbledb::Value::U64(_) | bumbledb::Value::I64(_)) => {
                        self.membership_literal += 1;
                        true
                    }
                    Term::Param(p) if t.scalar_params.contains(&p.0) => {
                        self.membership_param += 1;
                        true
                    }
                    Term::Var(var) if !t.var_types.get(var).is_some_and(|ty| ty.is_interval()) => {
                        self.membership_var += 1;
                        true
                    }
                    _ => false,
                };
                if is_point {
                    any = true;
                    match element {
                        IntervalElement::U64 => self.membership_u64 += 1,
                        IntervalElement::I64 => self.membership_i64 += 1,
                    }
                }
            }
        }
        any
    }

    fn record_comparisons(&mut self, rule: &Rule, t: &Typing) -> bool {
        let mut has_allen = false;
        for comparison in rule.conditions.iter().map(super::leaf) {
            let ty = match (&comparison.lhs, &comparison.rhs) {
                (Term::Var(var), _) | (_, Term::Var(var)) => t
                    .var_types
                    .get(var)
                    .copied()
                    .expect("comparison variables are atom-bound"),
                _ => unreachable!("the grammar never compares two constants"),
            };
            self.matrix[op_index(comparison.op)][type_index(&ty)] += 1;
            match comparison.op {
                CmpOp::Allen { mask } => {
                    has_allen = true;
                    match element_of(&ty) {
                        Some(IntervalElement::U64) => self.allen_u64 += 1,
                        Some(IntervalElement::I64) => self.allen_i64 += 1,
                        None => unreachable!("Allen is interval-typed by construction"),
                    }
                    if mask.popcount() > 1 {
                        self.allen_composite += 1;
                    } else {
                        self.allen_singleton += 1;
                    }

                    for (index, basic) in Basic::ALL.iter().enumerate() {
                        if mask.contains(*basic) {
                            self.allen_basics[index] += 1;
                        }
                    }
                }
                CmpOp::PointIn => match element_of(&ty) {
                    Some(IntervalElement::U64) => self.point_in_u64 += 1,
                    Some(IntervalElement::I64) => self.point_in_i64 += 1,
                    None => unreachable!("PointIn's left side is interval-typed"),
                },
                _ => {}
            }
            if let (Term::Var(lhs), Term::Var(rhs)) = (&comparison.lhs, &comparison.rhs) {
                let shared = t.var_atoms[lhs]
                    .iter()
                    .any(|a| t.var_atoms[rhs].contains(a));
                if !shared {
                    self.cross_residuals += 1;
                }
            }
            for term in [&comparison.lhs, &comparison.rhs] {
                match term {
                    Term::Param(_) => self.params += 1,
                    Term::ParamSet(_) => self.param_sets += 1,
                    _ => {}
                }
            }
        }
        has_allen
    }

    fn record_negations(&mut self, rule: &Rule, t: &Typing) {
        for atom in &rule.negated {
            self.negations += 1;
            if atom.bindings.is_empty() {
                self.negation_gate += 1;
                continue;
            }
            let relation = match atom.source {
                AtomSource::Edb(relation) => target::schema().relation(relation),
                AtomSource::Interior(_) => {
                    self.negation_open += 1;
                    for (_, term) in &atom.bindings {
                        match term {
                            Term::Literal(_) => self.negation_literal += 1,
                            Term::Param(_) => self.negation_param += 1,
                            Term::ParamSet(_) => self.negation_set += 1,
                            Term::Var(_) => {}
                        }
                    }
                    continue;
                }
            };
            let key_covered = atom
                .bindings
                .iter()
                .any(|(field, _)| relation.field(*field).generation == Generation::Fresh);
            if key_covered {
                self.negation_key_covered += 1;
            } else {
                self.negation_open += 1;
                if let AtomSource::Edb(rel) = atom.source
                    && (rel == ids::POSTING_TAG || rel == ids::POSTING)
                {
                    self.negation_multi_witness += 1;
                }
            }
            for (field, term) in &atom.bindings {
                match term {
                    Term::Literal(_) => self.negation_literal += 1,
                    Term::Param(_) => self.negation_param += 1,
                    Term::ParamSet(_) => self.negation_set += 1,
                    Term::Var(var) => {
                        if element_of(&field_type(atom, *field)).is_some()
                            && t.var_types.get(var).is_some_and(|ty| !ty.is_interval())
                        {
                            self.negation_membership += 1;
                        }
                    }
                }
            }
        }
    }

    fn record_finds(&mut self, rule: &Rule, t: &Typing) -> bool {
        let mut aggregates = 0u64;
        let mut projected_words = 0u64;
        let mut interval_finds = 0u64;
        for term in &rule.finds {
            match term {
                FindTerm::Var(var) => {
                    if t.var_types.get(var).is_some_and(|ty| ty.is_interval()) {
                        interval_finds += 1;
                        projected_words += 2;
                    } else {
                        projected_words += 1;
                    }
                }
                FindTerm::Aggregate { op, over } => {
                    aggregates += 1;
                    match op {
                        FoldOp::Sum => self.agg_sum += 1,
                        FoldOp::Min => self.agg_min += 1,
                        FoldOp::Max => self.agg_max += 1,
                    }
                    if matches!(t.var_types.get(over), Some(ValueType::U64)) {
                        self.agg_u64 += 1;
                    }
                }
                FindTerm::Count => {
                    aggregates += 1;
                    self.agg_count += 1;
                }
                FindTerm::Pack { .. } => {
                    aggregates += 1;
                }
            }
        }
        self.multi_aggregate += u64::from(aggregates > 1);

        self.wide_scalar += u64::from(interval_finds == 0 && projected_words > 8);
        self.wide_interval += u64::from(interval_finds >= 4);
        aggregates > 0
    }

    fn record_rules(&mut self, query: &Query, variant: Option<RulesVariant>) {
        let Some(variant) = variant else { return };
        match query.rules().len() {
            2 => self.rules_arms[0] += 1,
            3 => self.rules_arms[1] += 1,
            4 => self.rules_arms[2] += 1,
            arms => unreachable!("the rules shape emits 2-4 arms, got {arms}"),
        }
        match variant {
            RulesVariant::Disjoint => self.rules_disjoint += 1,
            RulesVariant::Overlap => self.rules_overlap += 1,
            RulesVariant::Aggregate => self.rules_aggregate += 1,
        }
    }

    fn record(&mut self, query: &Query, shape: Shape, tags: GenTags) {
        self.record_shape(shape);
        self.record_ground(tags.ground);
        self.record_closed(tags.closed);
        self.record_rules(query, tags.rules);
        self.misses += u64::from(tags.miss);
        self.bytes_hits += u64::from(tags.bytes_hit);
        self.bytes_misses += u64::from(tags.bytes_miss);
        self.adjacent_left += u64::from(tags.adjacent_left);
        self.adjacent_right += u64::from(tags.adjacent_right);
        for (index, drawn) in tags.ladder.iter().enumerate() {
            self.ladder[index] += u64::from(*drawn);
        }
        self.allen_random_mask += u64::from(tags.random_mask);

        let (mut has_membership, mut has_allen, mut has_negation, mut has_aggregate) =
            (false, false, false, false);
        let mut uses_set = false;
        for (rule_idx, rule) in walk::rules(query).enumerate() {
            self.gates += rule
                .atoms
                .iter()
                .filter(|atom| atom.bindings.is_empty())
                .count() as u64;
            let t = typing(&rule);

            for atom in &rule.atoms {
                let vars: Vec<&Term> = atom
                    .bindings
                    .iter()
                    .filter(|(_, term)| matches!(term, Term::Var(_)))
                    .map(|(_, term)| term)
                    .collect();
                if vars
                    .iter()
                    .enumerate()
                    .any(|(index, term)| vars[..index].contains(term))
                {
                    self.repeated_vars += 1;
                }
            }

            for atom in rule.atoms.iter().chain(&rule.negated) {
                for (_, term) in &atom.bindings {
                    match term {
                        Term::Param(_) => self.params += 1,
                        Term::ParamSet(_) => self.param_sets += 1,
                        _ => {}
                    }
                }
            }
            has_membership |= self.record_membership(&rule, &t);
            has_allen |= self.record_comparisons(&rule, &t);
            self.record_negations(&rule, &t);

            if rule_idx == 0 {
                has_aggregate = self.record_finds(&rule, &t);
            }
            has_negation |= !rule.negated.is_empty();
            uses_set |= rule
                .atoms
                .iter()
                .chain(&rule.negated)
                .flat_map(|atom| &atom.bindings)
                .any(|(_, term)| matches!(term, Term::ParamSet(_)))
                || rule.conditions.iter().map(super::leaf).any(|c| {
                    matches!(c.lhs, Term::ParamSet(_)) || matches!(c.rhs, Term::ParamSet(_))
                });
            self.spine_violations += spine_violations(&rule, &t);
        }

        self.neg_and_aggregate += u64::from(has_negation && has_aggregate);
        self.set_and_negation += u64::from(has_negation && uses_set);
        self.membership_and_allen += u64::from(has_membership && has_allen);
        self.mask_and_negation += u64::from(has_allen && has_negation);
    }
}

#[must_use]
pub fn coverage(n: u64, seed: u64, cfg: GenConfig) -> Coverage {
    let mut rng = Rng::new(seed);
    let mut cov = Coverage::default();
    for _ in 0..n {
        let (query, shape, tags) = random_query_tagged(&mut rng, cfg);
        cov.record(&query, shape, tags);
    }
    cov
}

#[cfg(test)]
mod reach_walk {
    use super::*;
    use crate::corpus_gen::{GenConfig, Rng, Scale};
    use crate::querygen::random_reach_query;
    use crate::walk;

    #[test]
    fn typing_and_record_do_not_panic_on_a_reach_query() {
        let mut rng = Rng::new(12);
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::S,
        };
        let (query, _) = random_reach_query(&mut rng, cfg);
        for rule in walk::rules(&query) {
            let _ = typing(&rule);
        }
        let mut cov = Coverage::default();
        cov.record(&query, Shape::KeyProbe, GenTags::default());
    }
}
