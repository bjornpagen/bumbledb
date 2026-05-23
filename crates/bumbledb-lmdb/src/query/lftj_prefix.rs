use super::*;

pub(super) fn lftj_prefix_proves_empty(
    image: &crate::QueryImage,
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    atoms: &[NormAtom],
    variable_order_ids: &[usize],
    counters: &mut PlanCounters,
) -> Result<bool> {
    if query.predicates.is_empty()
        && !atoms.iter().any(|atom| {
            atom.fields
                .iter()
                .any(|field| matches!(field.term, NormTerm::Input(_) | NormTerm::Literal(_)))
        })
    {
        return Ok(false);
    }
    let max_depth = variable_order_ids.len().saturating_sub(1).min(3);
    for depth in 0..=max_depth {
        let prefix_vars = variable_order_ids
            .iter()
            .take(depth + 1)
            .copied()
            .collect::<BTreeSet<_>>();
        let prefix_atoms = atoms
            .iter()
            .filter(|atom| {
                let variables = atom_variables(atom);
                if depth == 0 {
                    variables
                        .iter()
                        .any(|variable| prefix_vars.contains(variable))
                } else {
                    !variables.is_empty()
                        && variables
                            .iter()
                            .all(|variable| prefix_vars.contains(variable))
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        if prefix_atoms.is_empty() {
            continue;
        }
        let atom_plans =
            build_lftj_atom_plans(image, inputs, &prefix_atoms, variable_order_ids, counters)?;
        if atom_plans.iter().any(|atom| atom.fact_count == 0) {
            return Ok(true);
        }
        if !lftj_prefix_has_binding(txn, query, inputs, variable_order_ids, &atom_plans, depth)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn lftj_prefix_has_binding(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    inputs: &EncodedInputs,
    variable_order_ids: &[usize],
    atom_plans: &[LftjAtomPlan<'_>],
    max_depth: usize,
) -> Result<bool> {
    let participants_by_variable = lftj_participants_by_variable(query.vars.len(), atom_plans);
    let iters = atom_plans.iter().map(|atom| atom.source.iter()).collect();
    let mut probe = LftjPrefixFilter {
        txn,
        query,
        inputs,
        variable_order_ids,
        max_depth,
        participants_by_variable,
        iters,
        binding: EncodedBinding::new(query.vars.len()),
        counters: PlanCounters::default(),
    };
    probe.execute(0)
}

struct LftjPrefixFilter<'txn, 'input, 'query, 'image> {
    txn: &'input ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    variable_order_ids: &'input [usize],
    max_depth: usize,
    participants_by_variable: Vec<SmallParticipants>,
    iters: Vec<LftjTrieIter<'image>>,
    binding: EncodedBinding,
    counters: PlanCounters,
}

impl LftjPrefixFilter<'_, '_, '_, '_> {
    fn execute(&mut self, depth: usize) -> Result<bool> {
        if depth > self.max_depth {
            return Ok(true);
        }
        let variable = self.variable_order_ids[depth];
        let participants = self
            .participants_by_variable
            .get(variable)
            .cloned()
            .unwrap_or_default();
        if participants.is_empty() {
            return Ok(true);
        }

        for atom_id in &participants {
            self.iters[*atom_id].open();
        }
        let mut leapfrog = LeapfrogState::new(participants.clone());
        leapfrog.init(&mut self.iters, &mut self.counters)?;
        while !leapfrog.at_end() {
            let value = leapfrog.key(&self.iters, &mut self.counters)?;
            if self.binding.bind(variable, value) {
                let keep = comparisons_ready_pass(
                    self.txn,
                    &self.query.predicates,
                    self.query,
                    self.inputs,
                    &self.binding,
                    &mut self.counters,
                )?;
                if keep && self.execute(depth + 1)? {
                    self.binding.unbind(variable);
                    for atom_id in participants.iter().rev() {
                        self.iters[*atom_id].up();
                    }
                    return Ok(true);
                }
                self.binding.unbind(variable);
            }
            leapfrog.next(&mut self.iters, &mut self.counters)?;
        }
        for atom_id in participants.iter().rev() {
            self.iters[*atom_id].up();
        }
        Ok(false)
    }
}
