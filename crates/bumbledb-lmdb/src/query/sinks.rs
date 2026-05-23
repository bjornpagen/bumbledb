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

// Output sinks own projection, aggregation, cardinality, and result-set materialization.
#[derive(Clone, Debug)]
enum OutputSink {
    Cardinality(CardinalitySink),
    Project(EncodedProjectSink),
    Aggregate(AggregateSink),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SinkMode {
    Materialize,
    CardinalityOnly,
}

impl OutputSink {
    fn new(output: &OutputPlan) -> Self {
        Self::new_with_mode(output, SinkMode::Materialize)
    }

    fn new_count_facts(output: &OutputPlan) -> Self {
        Self::new_with_mode(output, SinkMode::CardinalityOnly)
    }

    fn new_with_mode(output: &OutputPlan, mode: SinkMode) -> Self {
        if mode == SinkMode::CardinalityOnly {
            return OutputSink::Cardinality(CardinalitySink::new(output));
        }
        match output {
            OutputPlan::Project(plan) => OutputSink::Project(EncodedProjectSink::new(plan)),
            OutputPlan::Aggregate(plan) => OutputSink::Aggregate(AggregateSink::new(plan)),
        }
    }

    fn finish_count(self) -> Result<usize> {
        let OutputSink::Cardinality(sink) = self else {
            return Err(Error::internal(
                "count facts requested from materializing sink",
            ));
        };
        Ok(sink.finish_count())
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
            OutputSink::Cardinality(sink) => sink.emit(txn, query, binding, counters),
            OutputSink::Project(sink) => sink.emit(txn, query, binding, counters),
            OutputSink::Aggregate(sink) => sink.emit(txn, query, binding, counters),
        }
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        match self {
            OutputSink::Cardinality(sink) => sink.finish(txn, query, counters),
            OutputSink::Project(sink) => sink.finish(txn, query, counters),
            OutputSink::Aggregate(sink) => sink.finish(txn, query, counters),
        }
    }

    fn emit_project_batch(
        &mut self,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<bool> {
        let OutputSink::Project(sink) = self else {
            return Ok(false);
        };
        sink.push_binding(query, binding, counters)?;
        Ok(true)
    }
}

fn is_global_count_plan(plan: &AggregatePlan) -> bool {
    plan.group_vars.is_empty()
        && plan.aggregates.len() == 1
        && matches!(
            plan.aggregates[0].function,
            AggregateFunction::CountDomain | AggregateFunction::CountDistinct
        )
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

#[derive(Clone, Debug)]
struct CardinalitySink {
    output: OutputPlan,
    global_count: u64,
    project_facts: BTreeSet<SmallEncodedFact>,
    aggregate_groups: BTreeSet<SmallEncodedFact>,
}

impl CardinalitySink {
    fn new(output: &OutputPlan) -> Self {
        Self {
            output: output.clone(),
            global_count: 0,
            project_facts: BTreeSet::new(),
            aggregate_groups: BTreeSet::new(),
        }
    }

    fn finish_count(self) -> usize {
        match self.output {
            OutputPlan::Project(_) => self.project_facts.len(),
            OutputPlan::Aggregate(plan) if is_global_count_plan(&plan) => 1,
            OutputPlan::Aggregate(_) => self.aggregate_groups.len(),
        }
    }
}

impl FactSink for CardinalitySink {
    fn emit(
        &mut self,
        _txn: &ReadTxn<'_>,
        _query: &NormalizedQuery,
        binding: &EncodedBinding,
        _counters: &mut PlanCounters,
    ) -> Result<()> {
        match &self.output {
            OutputPlan::Project(plan) => {
                let fact = plan
                    .vars
                    .iter()
                    .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
                    .collect::<Result<SmallEncodedFact>>()?;
                self.project_facts.insert(fact);
            }
            OutputPlan::Aggregate(plan) => {
                if is_global_count_plan(plan) {
                    self.global_count = self.global_count.saturating_add(1);
                    return Ok(());
                }
                let key = plan
                    .group_vars
                    .iter()
                    .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
                    .collect::<Result<SmallEncodedFact>>()?;
                self.aggregate_groups.insert(key);
            }
        }
        Ok(())
    }

    fn finish(
        self,
        _txn: &ReadTxn<'_>,
        _query: &NormalizedQuery,
        _counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        Ok(Vec::new())
    }
}

#[derive(Clone, Debug)]
struct AggregateSink {
    group_vars: Vec<VarId>,
    terms: Vec<AggregateTerm>,
    groups: BTreeMap<SmallEncodedFact, Vec<AggregateState>>,
    seen_domains: BTreeMap<(SmallEncodedFact, usize), BTreeSet<SmallEncodedFact>>,
}

impl AggregateSink {
    fn new(plan: &AggregatePlan) -> Self {
        Self {
            group_vars: plan.group_vars.clone(),
            terms: plan.aggregates.clone(),
            groups: BTreeMap::new(),
            seen_domains: BTreeMap::new(),
        }
    }

    fn group_key(&self, binding: &EncodedBinding) -> Result<SmallEncodedFact> {
        self.group_vars
            .iter()
            .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
            .collect()
    }

    fn domain_key(term: &AggregateTerm, binding: &EncodedBinding) -> Result<SmallEncodedFact> {
        term.domain_vars
            .iter()
            .map(|variable| bound_encoded_variable(binding, variable.0 as usize).cloned())
            .collect()
    }
}

impl FactSink for AggregateSink {
    fn emit(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        counters.aggregate_emit_calls += 1;
        let key = self.group_key(binding)?;
        let states = ensure_aggregate_group(&mut self.groups, &self.terms, key.clone());
        for (ordinal, (state, term)) in states.iter_mut().zip(&self.terms).enumerate() {
            let domain_key = Self::domain_key(term, binding)?;
            let seen = self.seen_domains.entry((key.clone(), ordinal)).or_default();
            if !seen.insert(domain_key) {
                continue;
            }
            state.apply_encoded(txn, query, binding, term, counters)?;
        }
        Ok(())
    }

    fn finish(
        self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        counters: &mut PlanCounters,
    ) -> Result<Vec<Vec<Value>>> {
        let _span =
            tracing::debug_span!("bumbledb.query.aggregate", groups = self.groups.len()).entered();
        let mut facts = Vec::new();
        let mut groups = self.groups;
        if groups.is_empty()
            && self.group_vars.is_empty()
            && self.terms.len() == 1
            && matches!(
                self.terms[0].function,
                AggregateFunction::CountDomain | AggregateFunction::CountDistinct
            )
        {
            groups.insert(
                SmallEncodedFact::new(),
                initial_aggregate_states(&self.terms),
            );
        }
        for (key, states) in groups {
            let mut fact = Vec::new();
            let mut key_iter = key.into_iter();
            let mut state_iter = states.into_iter();
            for term in &query.find {
                match term {
                    NormFindTerm::Variable { variable } => {
                        let value = key_iter
                            .next()
                            .ok_or_else(|| Error::internal("aggregate group key is missing"))?;
                        fact.push(decode_output_value(
                            txn,
                            &query.vars[variable.0 as usize].value_type,
                            value,
                            counters,
                        )?);
                    }
                    NormFindTerm::Aggregate { value_type, .. } => {
                        counters.materialized_output_values += 1;
                        let state = state_iter
                            .next()
                            .ok_or_else(|| Error::internal("aggregate state is missing"))?;
                        fact.push(state.finish_encoded(txn, value_type, counters)?);
                    }
                }
            }
            facts.push(fact);
        }
        facts.sort();
        Ok(facts)
    }
}

fn initial_aggregate_states(terms: &[AggregateTerm]) -> Vec<AggregateState> {
    terms
        .iter()
        .map(|term| AggregateState::new_encoded(term.function, term.value_type.clone()))
        .collect()
}

fn ensure_aggregate_group<'a>(
    groups: &'a mut BTreeMap<SmallEncodedFact, Vec<AggregateState>>,
    terms: &[AggregateTerm],
    key: SmallEncodedFact,
) -> &'a mut Vec<AggregateState> {
    match groups.entry(key) {
        std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(initial_aggregate_states(terms))
        }
    }
}

fn bound_encoded_variable(binding: &EncodedBinding, variable: usize) -> Result<&EncodedOwned> {
    binding
        .get(variable)
        .ok_or_else(|| Error::internal(format!("variable {variable} is unbound at projection")))
}

fn decode_bound_variable(
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    binding: &EncodedBinding,
    variable: usize,
    counters: &mut PlanCounters,
) -> Result<Value> {
    let value = bound_encoded_variable(binding, variable)?;
    record_decode(&query.vars[variable].value_type, counters);
    txn.decode_query_value(&query.vars[variable].value_type, value.as_bytes())
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

#[derive(Clone, Debug)]
enum AggregateState {
    Count(u64),
    SumU64(u64),
    SumI64(i64),
    SumDecimal(i128),
    EncodedMin(Option<EncodedOwned>),
    EncodedMax(Option<EncodedOwned>),
    Min(Option<Value>),
    Max(Option<Value>),
}

impl AggregateState {
    fn new(function: AggregateFunction, value_type: ValueType) -> Self {
        match (function, value_type) {
            (AggregateFunction::CountDomain | AggregateFunction::CountDistinct, _) => {
                AggregateState::Count(0)
            }
            (AggregateFunction::Sum, ValueType::U64) => AggregateState::SumU64(0),
            (AggregateFunction::Sum, ValueType::I64) => AggregateState::SumI64(0),
            (AggregateFunction::Sum, ValueType::Decimal { .. }) => AggregateState::SumDecimal(0),
            (AggregateFunction::Min, _) => AggregateState::Min(None),
            (AggregateFunction::Max, _) => AggregateState::Max(None),
            _ => AggregateState::Count(0),
        }
    }

    fn new_encoded(function: AggregateFunction, value_type: ValueType) -> Self {
        match function {
            AggregateFunction::Min if encoded_minmax_supported(&value_type) => {
                AggregateState::EncodedMin(None)
            }
            AggregateFunction::Max if encoded_minmax_supported(&value_type) => {
                AggregateState::EncodedMax(None)
            }
            _ => AggregateState::new(function, value_type),
        }
    }

    fn apply_count(&mut self) -> Result<()> {
        let AggregateState::Count(count) = self else {
            return Err(Error::internal("count aggregate state mismatch"));
        };
        *count = count
            .checked_add(1)
            .ok_or_else(|| Error::integer_overflow("count"))?;
        Ok(())
    }

    fn apply_encoded(
        &mut self,
        txn: &ReadTxn<'_>,
        query: &NormalizedQuery,
        binding: &EncodedBinding,
        term: &AggregateTerm,
        counters: &mut PlanCounters,
    ) -> Result<()> {
        match self {
            AggregateState::Count(_) => self.apply_count(),
            AggregateState::EncodedMin(current) => {
                let value = bound_encoded_variable(binding, term.var.0 as usize)?.clone();
                if current.as_ref().is_none_or(|existing| &value < existing) {
                    *current = Some(value);
                }
                Ok(())
            }
            AggregateState::EncodedMax(current) => {
                let value = bound_encoded_variable(binding, term.var.0 as usize)?.clone();
                if current.as_ref().is_none_or(|existing| &value > existing) {
                    *current = Some(value);
                }
                Ok(())
            }
            _ => {
                let value =
                    decode_bound_variable(txn, query, binding, term.var.0 as usize, counters)?;
                self.apply(&value)
            }
        }
    }

    fn apply(&mut self, value: &Value) -> Result<()> {
        match self {
            AggregateState::Count(_) => self.apply_count()?,
            AggregateState::SumU64(sum) => {
                let Value::U64(value) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::integer_overflow("sum"))?;
            }
            AggregateState::SumI64(sum) => {
                let Value::I64(value) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::integer_overflow("sum"))?;
            }
            AggregateState::SumDecimal(sum) => {
                let Value::Decimal(DecimalRaw(value)) = value else {
                    return Err(Error::aggregate_type_mismatch("sum", value.kind_name()));
                };
                *sum = sum
                    .checked_add(*value)
                    .ok_or_else(|| Error::decimal_overflow("sum"))?;
            }
            AggregateState::EncodedMin(_) | AggregateState::EncodedMax(_) => {
                return Err(Error::internal(
                    "encoded aggregate state cannot apply logical value",
                ));
            }
            AggregateState::Min(current) => match current {
                Some(existing) if &*existing <= value => {}
                _ => *current = Some(value.clone()),
            },
            AggregateState::Max(current) => match current {
                Some(existing) if &*existing >= value => {}
                _ => *current = Some(value.clone()),
            },
        }
        Ok(())
    }

    fn finish(self) -> Result<Value> {
        Ok(match self {
            AggregateState::Count(count) => Value::U64(count),
            AggregateState::SumU64(sum) => Value::U64(sum),
            AggregateState::SumI64(sum) => Value::I64(sum),
            AggregateState::SumDecimal(sum) => Value::Decimal(DecimalRaw(sum)),
            AggregateState::EncodedMin(_) | AggregateState::EncodedMax(_) => {
                return Err(Error::internal(
                    "encoded aggregate state requires output decoder",
                ));
            }
            AggregateState::Min(Some(value)) | AggregateState::Max(Some(value)) => value,
            AggregateState::Min(None) | AggregateState::Max(None) => Value::U64(0),
        })
    }

    fn finish_encoded(
        self,
        txn: &ReadTxn<'_>,
        value_type: &ValueType,
        counters: &mut PlanCounters,
    ) -> Result<Value> {
        Ok(match self {
            AggregateState::EncodedMin(Some(value)) | AggregateState::EncodedMax(Some(value)) => {
                record_decode(value_type, counters);
                txn.decode_query_value(value_type, value.as_bytes())?
            }
            AggregateState::EncodedMin(None) | AggregateState::EncodedMax(None) => Value::U64(0),
            state => state.finish()?,
        })
    }
}

fn encoded_minmax_supported(value_type: &ValueType) -> bool {
    !matches!(value_type, ValueType::String | ValueType::Bytes)
}

fn value_type_name(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Bool => "bool".to_owned(),
        ValueType::U64 => "u64".to_owned(),
        ValueType::I64 => "i64".to_owned(),
        ValueType::TimestampMicros => "timestamp".to_owned(),
        ValueType::Decimal { scale } => format!("decimal(scale={scale})"),
        ValueType::Enum { name } => name.clone(),
        ValueType::String => "string".to_owned(),
        ValueType::Bytes => "bytes".to_owned(),
        ValueType::Serial {
            type_name,
            owning_relation,
        } => format!("{type_name}@{owning_relation}"),
    }
}

