use bumbledb::{AggOp, CmpOp, FindTerm, Query, Term, VarId};

use crate::gen::{GenConfig, Rng};
use crate::querygen::construct::random_query_tagged;
use crate::querygen::{Coverage, GenTags, Shape, CMP_OPS};

/// Whether an (op, type) cell is legal under the roster: `Eq`/`Ne`
/// everywhere, order operators over the two integer types only.
#[must_use]
pub fn cmp_cell_legal(op_idx: usize, type_idx: usize) -> bool {
    op_idx < 2 || type_idx < 2
}

fn op_index(op: CmpOp) -> usize {
    CMP_OPS.iter().position(|o| *o == op).expect("all six ops")
}

fn type_index(ty: &bumbledb::schema::ValueType) -> usize {
    use bumbledb::schema::ValueType;
    match ty {
        ValueType::U64 => 0,
        ValueType::I64 => 1,
        ValueType::Enum { .. } => 2,
        ValueType::Bool => 3,
        ValueType::String => 4,
        ValueType::Bytes => 5,
    }
}

impl Coverage {
    #[allow(clippy::too_many_lines)]
    fn record(&mut self, query: &Query, shape: Shape, tags: GenTags) {
        match shape {
            Shape::Guard => self.guard += 1,
            Shape::Star => self.star += 1,
            Shape::Chain => self.chain += 1,
            Shape::SelfJoin => self.self_join += 1,
            Shape::Gated => self.gated += 1,
            Shape::Aggregate => self.aggregate += 1,
        }
        self.gates += query
            .atoms
            .iter()
            .filter(|atom| atom.bindings.is_empty())
            .count() as u64;
        self.misses += u64::from(tags.miss);
        self.bytes_hits += u64::from(tags.bytes_hit);
        self.bytes_misses += u64::from(tags.bytes_miss);
        // Per-variable anchors: the (relation, field) that types each
        // var, and the atom set it binds in (cross-residual detection).
        let mut var_type = std::collections::HashMap::new();
        let mut var_atoms: std::collections::HashMap<VarId, Vec<usize>> =
            std::collections::HashMap::new();
        for (atom_idx, atom) in query.atoms.iter().enumerate() {
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
            for (field, term) in &atom.bindings {
                if let Term::Var(var) = term {
                    var_type.entry(*var).or_insert_with(|| {
                        crate::schema::schema()
                            .relation(atom.relation)
                            .field(*field)
                            .value_type
                            .clone()
                    });
                    var_atoms.entry(*var).or_default().push(atom_idx);
                }
            }
        }
        for comparison in &query.predicates {
            let ty = match (&comparison.lhs, &comparison.rhs) {
                (Term::Var(var), _) | (_, Term::Var(var)) => var_type
                    .get(var)
                    .expect("comparison variables are atom-bound"),
                _ => unreachable!("the grammar never compares two constants"),
            };
            self.matrix[op_index(comparison.op)][type_index(ty)] += 1;
            if let (Term::Var(lhs), Term::Var(rhs)) = (&comparison.lhs, &comparison.rhs) {
                let shared = var_atoms[lhs].iter().any(|a| var_atoms[rhs].contains(a));
                if !shared {
                    self.cross_residuals += 1;
                }
            }
            for term in [&comparison.lhs, &comparison.rhs] {
                if matches!(term, Term::Param(_)) {
                    self.params += 1;
                }
            }
        }
        for atom in &query.atoms {
            for (_, term) in &atom.bindings {
                if matches!(term, Term::Param(_)) {
                    self.params += 1;
                }
            }
        }
        let mut aggregates = 0u64;
        for term in &query.finds {
            if let FindTerm::Aggregate { op, over } = term {
                aggregates += 1;
                match op {
                    AggOp::Sum => self.agg_sum += 1,
                    AggOp::Min => self.agg_min += 1,
                    AggOp::Max => self.agg_max += 1,
                    AggOp::Count => self.agg_count += 1,
                }
                if let Some(var) = over {
                    if matches!(var_type.get(var), Some(bumbledb::schema::ValueType::U64)) {
                        self.agg_u64 += 1;
                    }
                }
            }
        }
        self.multi_aggregate += u64::from(aggregates > 1);
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
