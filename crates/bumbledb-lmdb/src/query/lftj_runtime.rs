fn execute_free_join<'txn, 'query, S: FactSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
) -> Result<()> {
    let _span = tracing::debug_span!(
        "bumbledb.query.free_join.dispatch",
        nodes = plan.summary.free_join.nodes.len()
    )
    .entered();
    if !plan.summary.free_join.is_free_join_sorted_leapfrog() {
        return Err(Error::internal("non-pure free join plan has no runtime"));
    }
    execute_lftj(image, txn, query, inputs, plan, sink)
}

fn encoded_owned_for_width(width: usize, bytes: &[u8]) -> Result<EncodedOwned> {
    match width {
        1 => {
            Ok(EncodedOwned::One(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        8 => {
            Ok(EncodedOwned::Eight(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        16 => {
            Ok(EncodedOwned::Sixteen(bytes.try_into().map_err(|_| {
                Error::internal("encoded value width mismatch")
            })?))
        }
        width => Err(Error::internal(format!(
            "unsupported encoded value width {width}"
        ))),
    }
}

fn encoded_ref_for_width(bytes: &[u8]) -> Option<crate::EncodedRef<'_>> {
    match bytes.len() {
        1 => Some(crate::EncodedRef::One(bytes.try_into().ok()?)),
        8 => Some(crate::EncodedRef::Eight(bytes.try_into().ok()?)),
        16 => Some(crate::EncodedRef::Sixteen(bytes.try_into().ok()?)),
        _ => None,
    }
}

fn execute_lftj<'txn, 'query, S: FactSink>(
    image: &crate::QueryImage,
    txn: &ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &EncodedInputs,
    plan: &mut ExecutionPlan,
    sink: &mut S,
) -> Result<()> {
    let variable_order_ids = free_join_variable_order_ids(&plan.summary.free_join)?;
    let build_start = Instant::now();
    let build_alloc_start = allocation::snapshot();
    let atom_plans = {
        let _span = tracing::debug_span!(
            "bumbledb.query.lftj.build",
            atoms = query.atoms.len()
        )
        .entered();
        if lftj_prefix_proves_empty(
            image,
            txn,
            query,
            inputs,
            &query.atoms,
            &variable_order_ids,
            &mut plan.summary.counters,
        )? {
            None
        } else {
            Some(build_lftj_atom_plans(
                image,
                query,
                inputs,
                &query.atoms,
                &variable_order_ids,
                &mut plan.summary.counters,
            )?)
        }
    };
    plan.summary.timings.lftj_build_micros = plan
        .summary
        .timings
        .lftj_build_micros
        .saturating_add(elapsed_micros(build_start));
    plan.summary.allocations.lftj_build = allocation_delta_since(build_alloc_start);
    let Some(atom_plans) = atom_plans else {
        return Ok(());
    };
    if atom_plans.iter().any(|atom| atom.fact_count == 0) {
        return Ok(());
    }
    let runtime = LftjRuntime {
        participants_by_variable: lftj_participants_by_variable(query.vars.len(), &atom_plans),
        iters: atom_plans.iter().map(|atom| atom.source.iter()).collect(),
    };
    let execute_start = Instant::now();
    let execute_alloc_start = allocation::snapshot();
    let result = {
        let _span =
            tracing::debug_span!("bumbledb.query.lftj.execute", variables = query.vars.len())
                .entered();
        let mut executor = LftjExecutor {
            txn,
            query,
            inputs,
            plan,
            runtime,
            variable_order_ids,
            binding: EncodedBinding::new(query.vars.len()),
            sink,
        };
        executor.execute(0)
    };
    plan.summary.timings.lftj_execute_micros = plan
        .summary
        .timings
        .lftj_execute_micros
        .saturating_add(elapsed_micros(execute_start));
    plan.summary.allocations.lftj_execute = allocation_delta_since(execute_alloc_start);
    result
}

fn free_join_variable_order_ids(plan: &FreeJoinPlan) -> Result<Vec<usize>> {
    plan.nodes
        .iter()
        .map(|node| {
            let [variable] = node.bind_vars.as_slice() else {
                return Err(Error::internal("Free Join node must bind one variable"));
            };
            Ok(variable.0 as usize)
        })
        .collect()
}

fn lftj_participants_by_variable(
    variable_count: usize,
    atom_plans: &[LftjAtomPlan<'_>],
) -> Vec<SmallParticipants> {
    let mut participants = vec![SmallParticipants::new(); variable_count];
    for (atom_id, atom) in atom_plans.iter().enumerate() {
        for variable in &atom.variables {
            participants[*variable].push(atom_id);
        }
    }
    participants
}

fn lftj_prefix_proves_empty(
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
        let atom_plans = build_lftj_atom_plans(
            image,
            query,
            inputs,
            &prefix_atoms,
            variable_order_ids,
            counters,
        )?;
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
        while !leapfrog.at_end {
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

struct LftjExecutor<'txn, 'input, 'query, 'plan, 'image, S: FactSink> {
    txn: &'input ReadTxn<'txn>,
    query: &'query NormalizedQuery,
    inputs: &'input EncodedInputs,
    plan: &'plan mut ExecutionPlan,
    runtime: LftjRuntime<'image>,
    variable_order_ids: Vec<usize>,
    binding: EncodedBinding,
    sink: &'plan mut S,
}

impl<S: FactSink> LftjExecutor<'_, '_, '_, '_, '_, S> {
    fn execute(&mut self, depth: usize) -> Result<()> {
        if depth == self.variable_order_ids.len() {
            if comparisons_ready_pass(
                self.txn,
                &self.plan.comparisons,
                self.query,
                self.inputs,
                &self.binding,
                &mut self.plan.summary.counters,
            )? {
                self.plan.summary.counters.bindings_yielded += 1;
                self.plan.summary.counters.bindings_completed += 1;
                self.plan.summary.counters.lftj_completed_bindings += 1;
                let _span = tracing::trace_span!("bumbledb.query.sink.emit").entered();
                if !self.sink.emit_project_batch(
                    self.query,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )? {
                    self.sink.emit(
                        self.txn,
                        self.query,
                        &self.binding,
                        &mut self.plan.summary.counters,
                    )?;
                }
            }
            return Ok(());
        }

        let variable = self.variable_order_ids[depth];
        let participants = self.participants(variable);
        if participants.is_empty() {
            return Err(Error::internal(format!(
                "variable {} is not constrained by any trie atom",
                self.query.vars[variable].name
            )));
        }

        for atom_id in &participants {
            self.runtime.iters[*atom_id].open();
            self.plan.summary.counters.trie_open += 1;
            self.plan.summary.counters.lftj_open_calls += 1;
        }

        let mut leapfrog = LeapfrogState::new(participants.clone());
        leapfrog.init(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
        while !leapfrog.at_end {
            let value = leapfrog.key(&self.runtime.iters, &mut self.plan.summary.counters)?;
            self.plan.summary.counters.variable_candidates += 1;
            self.plan.summary.counters.lftj_candidate_values += 1;
            if self.binding.bind(variable, value) {
                self.plan.summary.counters.lftj_bind_successes += 1;
                let keep = comparisons_ready_pass(
                    self.txn,
                    &self.plan.comparisons,
                    self.query,
                    self.inputs,
                    &self.binding,
                    &mut self.plan.summary.counters,
                )?;
                if keep {
                    if let Some(facts) = self.plan.summary.node_facts.get_mut(depth) {
                        facts.actual_facts = facts.actual_facts.saturating_add(1);
                    }
                    self.execute(depth + 1)?;
                }
                self.binding.unbind(variable);
            } else {
                self.plan.summary.counters.lftj_bind_rejects += 1;
            }
            leapfrog.next(&mut self.runtime.iters, &mut self.plan.summary.counters)?;
        }

        for atom_id in participants.iter().rev() {
            self.runtime.iters[*atom_id].up();
            self.plan.summary.counters.trie_up += 1;
            self.plan.summary.counters.lftj_up_calls += 1;
        }
        Ok(())
    }

    fn participants(&self, variable: usize) -> SmallParticipants {
        self.runtime
            .participants_by_variable
            .get(variable)
            .cloned()
            .unwrap_or_default()
    }
}

struct LeapfrogState {
    iter_ids: SmallParticipants,
    p: usize,
    at_end: bool,
}

impl LeapfrogState {
    fn new(iter_ids: SmallParticipants) -> Self {
        Self {
            iter_ids,
            p: 0,
            at_end: false,
        }
    }

    fn init(&mut self, iters: &mut [LftjTrieIter<'_>], counters: &mut PlanCounters) -> Result<()> {
        if self.iter_ids.iter().any(|id| iters[*id].at_end()) {
            self.at_end = true;
            return Ok(());
        }
        self.sort_iter_ids(iters, counters)?;
        self.p = 0;
        self.search(iters, counters)
    }

    fn sort_iter_ids(
        &mut self,
        iters: &[LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        let mut error = None;
        self.iter_ids.sort_by(|left, right| {
            if error.is_some() {
                return std::cmp::Ordering::Equal;
            }
            let Some(left) = key_ref_opt(&iters[*left], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            let Some(right) = key_ref_opt(&iters[*right], counters) else {
                error = Some(missing_trie_key_error());
                return std::cmp::Ordering::Equal;
            };
            compare_encoded_ref(left, right)
        });
        if let Some(error) = error {
            return Err(error);
        }
        Ok(())
    }

    fn key(&self, iters: &[LftjTrieIter<'_>], counters: &mut PlanCounters) -> Result<EncodedOwned> {
        self.iter_ids
            .first()
            .map(|id| key_owned(&iters[*id], counters))
            .transpose()?
            .ok_or_else(|| Error::internal("leapfrog join has no iterators"))
    }

    fn next(&mut self, iters: &mut [LftjTrieIter<'_>], counters: &mut PlanCounters) -> Result<()> {
        if self.at_end {
            return Ok(());
        }
        let id = self.iter_ids[self.p];
        iters[id].next();
        counters.trie_next += 1;
        counters.lftj_next_calls += 1;
        if iters[id].at_end() {
            self.at_end = true;
            return Ok(());
        }
        self.p = (self.p + 1) % self.iter_ids.len();
        self.search(iters, counters)
    }

    fn search(
        &mut self,
        iters: &mut [LftjTrieIter<'_>],
        counters: &mut PlanCounters,
    ) -> Result<()> {
        if self.iter_ids.is_empty() || self.at_end {
            return Ok(());
        }
        if self.iter_ids.len() == 1 {
            return Ok(());
        }
        let Some(mut max) = key_owned_opt(
            &iters[self.iter_ids[(self.p + self.iter_ids.len() - 1) % self.iter_ids.len()]],
            counters,
        ) else {
            return Err(missing_trie_key_error());
        };
        loop {
            let id = self.iter_ids[self.p];
            let Some(current) = key_ref_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            if compare_encoded_ref_owned(current, &max) == std::cmp::Ordering::Equal {
                return Ok(());
            }
            iters[id].seek(max.as_ref());
            counters.trie_seek += 1;
            counters.lftj_seek_calls += 1;
            if iters[id].at_end() {
                self.at_end = true;
                return Ok(());
            }
            let Some(next_max) = key_owned_opt(&iters[id], counters) else {
                return Err(missing_trie_key_error());
            };
            max = next_max;
            self.p = (self.p + 1) % self.iter_ids.len();
        }
    }
}

fn key_owned(iter: &LftjTrieIter<'_>, counters: &mut PlanCounters) -> Result<EncodedOwned> {
    key_owned_opt(iter, counters).ok_or_else(missing_trie_key_error)
}

fn key_owned_opt(iter: &LftjTrieIter<'_>, counters: &mut PlanCounters) -> Option<EncodedOwned> {
    key_ref_opt(iter, counters).map(EncodedOwned::from_ref)
}

fn key_ref_opt<'a>(
    iter: &'a LftjTrieIter<'a>,
    counters: &mut PlanCounters,
) -> Option<crate::EncodedRef<'a>> {
    let key = iter.key()?;
    counters.trie_key_reads += 1;
    counters.lftj_key_reads += 1;
    Some(key)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EncodedWidth {
    W1,
    W8,
    W16,
}

fn encoded_width_for_len(len: usize) -> Option<EncodedWidth> {
    match len {
        1 => Some(EncodedWidth::W1),
        8 => Some(EncodedWidth::W8),
        16 => Some(EncodedWidth::W16),
        _ => None,
    }
}

fn compare_encoded_ref(
    left: crate::EncodedRef<'_>,
    right: crate::EncodedRef<'_>,
) -> std::cmp::Ordering {
    compare_encoded_bytes(left.as_bytes(), right.as_bytes())
}

fn compare_encoded_ref_owned(
    left: crate::EncodedRef<'_>,
    right: &EncodedOwned,
) -> std::cmp::Ordering {
    compare_encoded_bytes(left.as_bytes(), right.as_bytes())
}

fn compare_encoded_bytes(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    match (encoded_width_for_len(left.len()), left.len() == right.len()) {
        (Some(EncodedWidth::W1), true) => left[0].cmp(&right[0]),
        (Some(EncodedWidth::W8), true) => {
            let mut left_bytes = [0u8; 8];
            let mut right_bytes = [0u8; 8];
            left_bytes.copy_from_slice(left);
            right_bytes.copy_from_slice(right);
            let left = u64::from_be_bytes(left_bytes);
            let right = u64::from_be_bytes(right_bytes);
            left.cmp(&right)
        }
        (Some(EncodedWidth::W16), true) | (None, _) | (_, false) => left.cmp(right),
    }
}

fn missing_trie_key_error() -> Error {
    Error::internal("trie key requested for exhausted iterator")
}
