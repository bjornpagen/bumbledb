trait FactSink {
    fn emit(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()>;

    fn emit_project_batch(
        &mut self,
        _query: &NormalizedQuery,
        _binding: &EncodedBinding,
        _counters: &mut PlanCounters,
    ) -> Result<bool> {
        Ok(false)
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>>
    where
        Self: Sized;
}

// Output sinks own projection, cardinality, and result-set materialization.
#[derive(Clone, Debug)]
enum OutputSink {
    Project(EncodedProjectSink),
}

impl OutputSink {
    fn new(output: &OutputPlan) -> Self {
        match output {
            OutputPlan::Project(plan) => OutputSink::Project(EncodedProjectSink::new(plan)),
        }
    }
}

impl FactSink for OutputSink {
    fn emit(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        counters.sink_emit_calls += 1;
        match self {
            OutputSink::Project(sink) => sink.emit(txn, query, binding, counters),
        }
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        match self {
            OutputSink::Project(sink) => sink.finish(txn, query, counters),
        }
    }

    fn emit_project_batch(
        &mut self,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        let OutputSink::Project(sink) = self;
        sink.push_binding(query, binding, counters)?;
        Ok(true)
    }
}

#[derive(Clone, Debug)]
struct EncodedProjectSink {
    vars: Vec<VarId>,
    facts: BTreeSet<SmallEncodedFact>,
}

impl EncodedProjectSink {
    fn new(plan: &ProjectPlan) -> Self {
        Self {
            vars: plan.vars.clone(),
            facts: BTreeSet::new(),
        }
    }

    fn push_binding(
        &mut self,
        _query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<u64> {
        let mut fact = SmallEncodedFact::new();
        let mut fact_width = 0u64;
        for variable in &self.vars {
            let value = bound_encoded_variable(binding, variable.0 as usize)?;
            fact_width = fact_width.saturating_add(value.as_bytes().len() as u64);
            fact.push(value.clone());
        }
        counters.encoded_project_facts_seen += 1;
        if self.facts.insert(fact) {
            counters.encoded_project_facts_inserted =
                counters.encoded_project_facts_inserted.saturating_add(1);
            counters.encoded_project_fact_bytes = counters
                .encoded_project_fact_bytes
                .saturating_add(fact_width);
            return Ok(fact_width);
        }
        Ok(0)
    }
}

impl FactSink for EncodedProjectSink {
    fn emit(
        &mut self,
        _txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        self.push_binding(query, binding, counters).map(|_| ())
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        let EncodedProjectSink { vars, facts } = self;
        let _span = tracing::debug_span!("bumbledb.query.project", facts = facts.len(),).entered();
        if facts.is_empty() {
            return Ok(Vec::new());
        }
        facts
            .into_iter()
            .map(|fact| {
                vars.iter()
                    .zip(fact)
                    .map(|(variable, value)| {
                        counters.project_decode_values += 1;
                        decode_output_value(
                            txn,
                            &query.vars[variable.0 as usize].value_type,
                            value,
                            counters,
                        )
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect()
    }
}

fn bound_encoded_variable(binding: &EncodedBinding, variable: usize) -> Result<&EncodedOwned> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}

fn decode_output_value(
    txn: &ReadTxn<'_>,
    value_type: &ValueType,
    value: EncodedOwned,
    counters: &mut PlanCounters,
) -> Result<Value> {
    counters.materialized_output_values += 1;
    record_decode(value_type, counters);
    txn.decode_query_value(value_type, value.as_bytes())
}
