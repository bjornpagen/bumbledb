//! The coverage contract's evidence collector: one pass per generated
//! query, counting every construct the n = 1000 test asserts. Structural
//! facts (negation shapes, membership kinds, the comparison matrix, the
//! sinks) are re-derived from the query itself; only corpus-content
//! facts (hit-vs-miss, boundary polarity) come from generation tags.

use bumbledb::schema::{Generation, IntervalElement, ValueType};
use bumbledb::{AggOp, Atom, CmpOp, FindTerm, MaskTerm, Query, Term, VarId};
use std::collections::{HashMap, HashSet};

use crate::gen::{GenConfig, Rng};
use crate::querygen::construct::random_query_tagged;
use crate::querygen::target::{self, ids};
use crate::querygen::{ChaseVariant, Coverage, GenTags, Shape};

/// Whether an (op, type) cell is legal under the roster: `Eq`/`Ne` over
/// all seven types, order operators over the two integer types only,
/// `Allen` (any mask) and `Contains` only at their interval-anchored
/// shapes.
#[must_use]
pub fn cmp_cell_legal(op_idx: usize, type_idx: usize) -> bool {
    match op_idx {
        0 | 1 => true,
        2..=5 => type_idx < 2,
        _ => type_idx == 6,
    }
}

/// The matrix row of an operator — every `Allen` mask shares one row (the
/// mask is a value of the operator, not a new operator).
fn op_index(op: CmpOp) -> usize {
    match op {
        CmpOp::Eq => 0,
        CmpOp::Ne => 1,
        CmpOp::Lt => 2,
        CmpOp::Le => 3,
        CmpOp::Gt => 4,
        CmpOp::Ge => 5,
        CmpOp::Allen { .. } => 6,
        CmpOp::Contains => 7,
    }
}

fn type_index(ty: &ValueType) -> usize {
    match ty {
        ValueType::U64 => 0,
        ValueType::I64 => 1,
        ValueType::Enum { .. } => 2,
        ValueType::Bool => 3,
        ValueType::String => 4,
        ValueType::Bytes => 5,
        ValueType::Interval { .. } => 6,
    }
}

/// The typing walk's product: variable and param resolutions mirroring
/// the validation boundary's bivalent-anchor rule for exactly the
/// shapes the generator emits (a scalar anchor wins; an interval-field
/// position with no scalar anchor is interval-valued).
struct Typing {
    var_types: HashMap<VarId, ValueType>,
    scalar_params: HashSet<u16>,
    var_atoms: HashMap<VarId, Vec<usize>>,
    var_pos: HashMap<VarId, (bumbledb::RelationId, bumbledb::FieldId)>,
}

fn field_type(atom: &Atom, field: bumbledb::FieldId) -> ValueType {
    target::schema()
        .relation(atom.relation)
        .field(field)
        .value_type
        .clone()
}

fn typing(query: &Query) -> Typing {
    let mut t = Typing {
        var_types: HashMap::new(),
        scalar_params: HashSet::new(),
        var_atoms: HashMap::new(),
        var_pos: HashMap::new(),
    };
    // Pass one: scalar-field positions anchor vars and params.
    for (atom_idx, atom) in query.rules[0].atoms.iter().enumerate() {
        for (field, term) in &atom.bindings {
            let ty = field_type(atom, *field);
            if let Term::Var(var) = term {
                t.var_atoms.entry(*var).or_default().push(atom_idx);
            }
            if matches!(ty, ValueType::Interval { .. }) {
                continue;
            }
            match term {
                Term::Var(var) => {
                    t.var_types.entry(*var).or_insert(ty);
                    t.var_pos.entry(*var).or_insert((atom.relation, *field));
                }
                Term::Param(p) | Term::ParamSet(p) => {
                    t.scalar_params.insert(p.0);
                }
                // The measure never appears in bindings (validated).
                Term::Literal(_) | Term::Duration(_) => {}
            }
        }
    }
    for atom in &query.rules[0].negated {
        for (field, term) in &atom.bindings {
            if matches!(field_type(atom, *field), ValueType::Interval { .. }) {
                continue;
            }
            if let Term::Param(p) | Term::ParamSet(p) = term {
                t.scalar_params.insert(p.0);
            }
        }
    }
    // Pass two: interval-field var positions with no scalar anchor are
    // interval-typed (the bivalent default).
    for atom in &query.rules[0].atoms {
        for (field, term) in &atom.bindings {
            let ty = field_type(atom, *field);
            if !matches!(ty, ValueType::Interval { .. }) {
                continue;
            }
            if let Term::Var(var) = term {
                t.var_types.entry(*var).or_insert(ty.clone());
                t.var_pos.entry(*var).or_insert((atom.relation, *field));
            }
        }
    }
    t
}

fn element_of(ty: &ValueType) -> Option<IntervalElement> {
    match ty {
        ValueType::Interval { element } => Some(*element),
        _ => None,
    }
}

/// The equality-spine cost-bound check
/// (`docs/architecture/60-validation.md` § the generator contract;
/// `40-execution.md` names the degenerate): every atom carrying a
/// var-point membership binding or an interval-typed side of a
/// cross-atom `Allen`/`Contains` must share an equality join
/// variable with another atom or carry an equality selection
/// (literal/param/set) on a scalar field; a negated atom whose only
/// bindings are memberships is the same Cartesian. Returns the count of
/// violating atoms — asserted zero by the contract test.
fn spine_violations(query: &Query, t: &Typing) -> u64 {
    use std::collections::BTreeSet;
    // Equality positions: a var at a scalar field, or an interval-typed
    // var at an interval field (value equality). A membership position
    // (element-typed var at an interval field) is not an equality.
    let mut eq_atoms: HashMap<VarId, BTreeSet<usize>> = HashMap::new();
    for (index, atom) in query.rules[0].atoms.iter().enumerate() {
        for (field, term) in &atom.bindings {
            let Term::Var(var) = term else { continue };
            let field_interval = matches!(field_type(atom, *field), ValueType::Interval { .. });
            let var_interval = matches!(t.var_types.get(var), Some(ValueType::Interval { .. }));
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
            !matches!(field_type(atom, *field), ValueType::Interval { .. })
                && matches!(term, Term::Literal(_) | Term::Param(_) | Term::ParamSet(_))
        })
    };
    // The atoms the rule binds: var-point membership occurrences…
    let mut needs: BTreeSet<usize> = BTreeSet::new();
    for (index, atom) in query.rules[0].atoms.iter().enumerate() {
        for (field, term) in &atom.bindings {
            if !matches!(field_type(atom, *field), ValueType::Interval { .. }) {
                continue;
            }
            if let Term::Var(var) = term {
                if !matches!(t.var_types.get(var), Some(ValueType::Interval { .. })) {
                    needs.insert(index);
                }
            }
        }
    }
    // …and interval-typed sides of cross-atom Allen/Contains.
    for comparison in query.rules[0].predicates.iter().map(super::leaf) {
        if !matches!(comparison.op, CmpOp::Allen { .. } | CmpOp::Contains) {
            continue;
        }
        if let (Term::Var(lhs), Term::Var(rhs)) = (&comparison.lhs, &comparison.rhs) {
            if t.var_atoms[lhs]
                .iter()
                .any(|a| t.var_atoms[rhs].contains(a))
            {
                continue; // a same-atom pair is a filter, not a join
            }
            for var in [lhs, rhs] {
                if matches!(t.var_types.get(var), Some(ValueType::Interval { .. })) {
                    needs.extend(t.var_atoms[var].iter().copied());
                }
            }
        }
    }
    let mut violations = needs
        .into_iter()
        .filter(|index| !has_eq_edge(*index) && !has_eq_selection(&query.rules[0].atoms[*index]))
        .count() as u64;
    for atom in &query.rules[0].negated {
        let mut memberships = 0usize;
        let mut others = 0usize;
        for (field, term) in &atom.bindings {
            let field_interval = matches!(field_type(atom, *field), ValueType::Interval { .. });
            let is_membership = field_interval
                && match term {
                    Term::Var(var) => {
                        !matches!(t.var_types.get(var), Some(ValueType::Interval { .. }))
                    }
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
            Shape::Guard => self.guard += 1,
            Shape::Star => self.star += 1,
            Shape::Chain => self.chain += 1,
            Shape::SelfJoin => self.self_join += 1,
            Shape::Gated => self.gated += 1,
            Shape::Aggregate => self.aggregate += 1,
            Shape::Membership => self.membership += 1,
            Shape::IntervalJoin => self.interval_join += 1,
            Shape::Boundary => self.boundary += 1,
            Shape::CountDistinct => self.count_distinct += 1,
            Shape::Arg => self.arg += 1,
            Shape::ExistenceWalk => self.existence_walk += 1,
            Shape::DuWalk => self.du_walk += 1,
        }
    }

    /// The chase-variant tallies (`shapes_chase.rs`): eliminable shapes
    /// (existence walks and both DU `==` directions) vs the two
    /// near-miss refusal classes.
    fn record_chase(&mut self, chase: Option<ChaseVariant>) {
        match chase {
            Some(ChaseVariant::Walk) => self.chase_eliminable += 1,
            Some(ChaseVariant::DuHeader) => {
                self.chase_eliminable += 1;
                self.du_header_falls += 1;
            }
            Some(ChaseVariant::DuChild) => {
                self.chase_eliminable += 1;
                self.du_child_falls += 1;
            }
            Some(ChaseVariant::WalkExtraField) => self.chase_extra_field += 1,
            Some(ChaseVariant::DuMissingPhi) => self.chase_missing_phi += 1,
            None => {}
        }
    }

    /// Membership bindings in the positive atoms: an interval-typed
    /// field carrying an element-typed term. Returns whether any exist
    /// (the composition detector's input).
    fn record_membership(&mut self, query: &Query, t: &Typing) -> bool {
        let mut any = false;
        for atom in &query.rules[0].atoms {
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
                    Term::Var(var)
                        if !matches!(t.var_types.get(var), Some(ValueType::Interval { .. })) =>
                    {
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

    fn record_comparisons(&mut self, query: &Query, t: &Typing) -> bool {
        let mut has_allen = false;
        for comparison in query.rules[0].predicates.iter().map(super::leaf) {
            let ty = match (&comparison.lhs, &comparison.rhs) {
                (Term::Var(var), _) | (_, Term::Var(var)) => t
                    .var_types
                    .get(var)
                    .expect("comparison variables are atom-bound")
                    .clone(),
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
                    let MaskTerm::Literal(mask) = mask else {
                        unreachable!("the generator emits literal masks (PRD 15 owns params)")
                    };
                    if mask.popcount() > 1 {
                        self.allen_composite += 1;
                    } else {
                        self.allen_singleton += 1;
                    }
                }
                CmpOp::Contains => match element_of(&ty) {
                    Some(IntervalElement::U64) => self.contains_u64 += 1,
                    Some(IntervalElement::I64) => self.contains_i64 += 1,
                    None => unreachable!("Contains' left side is interval-typed"),
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

    /// Negated-atom shapes: gate / key-covered / open (with the
    /// multiply-witnessed relations tracked), and the binding-term mix.
    fn record_negations(&mut self, query: &Query, t: &Typing) {
        for atom in &query.rules[0].negated {
            self.negations += 1;
            if atom.bindings.is_empty() {
                self.negation_gate += 1;
                continue;
            }
            let relation = target::schema().relation(atom.relation);
            let key_covered = atom
                .bindings
                .iter()
                .any(|(field, _)| relation.field(*field).generation == Generation::Fresh);
            if key_covered {
                self.negation_key_covered += 1;
            } else {
                self.negation_open += 1;
                if atom.relation == ids::POSTING_TAG || atom.relation == ids::POSTING {
                    self.negation_multi_witness += 1;
                }
            }
            for (field, term) in &atom.bindings {
                match term {
                    Term::Literal(_) => self.negation_literal += 1,
                    Term::Param(_) => self.negation_param += 1,
                    Term::ParamSet(_) => self.negation_set += 1,
                    Term::Duration(_) => unreachable!("validated: no measure in bindings"),
                    Term::Var(var) => {
                        // Membership inside negation: an element-typed
                        // var at an interval field.
                        if element_of(&field_type(atom, *field)).is_some()
                            && !matches!(
                                t.var_types.get(var),
                                Some(ValueType::Interval { .. }) | None
                            )
                        {
                            self.negation_membership += 1;
                        }
                    }
                }
            }
        }
    }

    fn record_finds(&mut self, query: &Query, t: &Typing) -> bool {
        let mut aggregates = 0u64;
        let mut has_var_find = false;
        let mut arg_key: Option<VarId> = None;
        let mut arg_key_projected = false;
        let mut projected_words = 0u64;
        let mut interval_finds = 0u64;
        for term in &query.rules[0].finds {
            match term {
                FindTerm::Var(var) => {
                    has_var_find = true;
                    if matches!(t.var_types.get(var), Some(ValueType::Interval { .. })) {
                        interval_finds += 1;
                        projected_words += 2;
                    } else {
                        projected_words += 1;
                    }
                }
                FindTerm::Aggregate { op, over } => {
                    aggregates += 1;
                    match op {
                        AggOp::Sum => self.agg_sum += 1,
                        AggOp::Min => self.agg_min += 1,
                        AggOp::Max => self.agg_max += 1,
                        AggOp::Count => self.agg_count += 1,
                        AggOp::CountDistinct => {
                            let var = over.expect("CountDistinct carries its input");
                            let ty = t.var_types.get(&var).expect("finds are bound");
                            self.count_distinct_types[type_index(ty)] += 1;
                        }
                        AggOp::ArgMax { key } => {
                            self.arg_max += 1;
                            arg_key = Some(*key);
                            arg_key_projected |= *over == Some(*key);
                        }
                        AggOp::ArgMin { key } => {
                            self.arg_min += 1;
                            arg_key = Some(*key);
                            arg_key_projected |= *over == Some(*key);
                        }
                    }
                    if let Some(var) = over {
                        if matches!(t.var_types.get(var), Some(ValueType::U64)) {
                            self.agg_u64 += 1;
                        }
                    }
                }
                // The measure positions: the generator does not emit them
                // yet (PRD 15 adds the measure's oracle rows); counted as
                // one projected word / one fold like their plain twins.
                FindTerm::Duration(_) => {
                    projected_words += 1;
                }
                FindTerm::AggregateDuration { .. } => {
                    aggregates += 1;
                }
            }
        }
        if let Some(key) = arg_key {
            if arg_key_projected {
                self.arg_key_projected += 1;
            }
            if !has_var_find {
                self.arg_global += 1;
            }
            match t.var_pos.get(&key) {
                Some(&(ids::POSTING, field)) if field == ids::posting::AMOUNT => {
                    self.arg_tie_key += 1;
                }
                Some(&(ids::POSTING, field)) if field == ids::posting::AT => {
                    self.arg_tie_free_key += 1;
                }
                _ => {}
            }
        }
        self.multi_aggregate += u64::from(aggregates > 1);
        // The wide-projection classes (the executor's hoist paths are
        // width-unbounded; the >8-word class stays oracle-covered).
        self.wide_scalar += u64::from(interval_finds == 0 && projected_words > 8);
        self.wide_interval += u64::from(interval_finds >= 4);
        aggregates > 0
    }

    fn record(&mut self, query: &Query, shape: Shape, tags: GenTags) {
        self.record_shape(shape);
        self.record_chase(tags.chase);
        self.gates += query.rules[0]
            .atoms
            .iter()
            .filter(|atom| atom.bindings.is_empty())
            .count() as u64;
        self.misses += u64::from(tags.miss);
        self.bytes_hits += u64::from(tags.bytes_hit);
        self.bytes_misses += u64::from(tags.bytes_miss);
        self.adjacent_left += u64::from(tags.adjacent_left);
        self.adjacent_right += u64::from(tags.adjacent_right);
        let t = typing(query);
        // Repeated in-atom variables.
        for atom in &query.rules[0].atoms {
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
        // Param and param-set binding occurrences (positive + negated).
        for atom in query.rules[0].atoms.iter().chain(&query.rules[0].negated) {
            for (_, term) in &atom.bindings {
                match term {
                    Term::Param(_) => self.params += 1,
                    Term::ParamSet(_) => self.param_sets += 1,
                    _ => {}
                }
            }
        }
        let has_membership = self.record_membership(query, &t);
        let has_allen = self.record_comparisons(query, &t);
        self.record_negations(query, &t);
        let has_aggregate = self.record_finds(query, &t);
        // The structural compositions where bugs hide.
        let has_negation = !query.rules[0].negated.is_empty();
        let uses_set =
            query.rules[0]
                .atoms
                .iter()
                .chain(&query.rules[0].negated)
                .flat_map(|atom| &atom.bindings)
                .any(|(_, term)| matches!(term, Term::ParamSet(_)))
                || query.rules[0].predicates.iter().map(super::leaf).any(|c| {
                    matches!(c.lhs, Term::ParamSet(_)) || matches!(c.rhs, Term::ParamSet(_))
                });
        self.neg_and_aggregate += u64::from(has_negation && has_aggregate);
        self.set_and_negation += u64::from(has_negation && uses_set);
        self.membership_and_allen += u64::from(has_membership && has_allen);
        self.spine_violations += spine_violations(query, &t);
    }
}

/// Generates `n` queries at the seed and counts every construct.
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
