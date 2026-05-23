use super::*;

pub(super) fn execute_free_join<'txn, 'query, S: FactSink>(
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
    plan.summary.free_join.validate()?;
    execute_lftj(image, txn, query, inputs, plan, sink)
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
        let _span =
            tracing::debug_span!("bumbledb.query.lftj.build", atoms = query.atoms.len()).entered();
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

pub(super) fn lftj_participants_by_variable(
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
        while !leapfrog.at_end() {
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
