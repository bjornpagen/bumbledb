use super::*;

impl<'env> ReadTxn<'env> {
    /// Executes a typed positive query IR against current indexes.
    #[tracing::instrument(name = "bumbledb.query.execute", skip_all, fields(vars = query.variables.len(), clauses = query.clauses.len(), inputs = query.inputs.len()))]
    pub fn execute_query(
        &self,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        let total_start = Instant::now();
        let total_alloc_start = allocation::snapshot();
        let mut timings = QueryTimings::default();
        let mut allocations = QueryAllocationStats::default();

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        {
            let _span = tracing::debug_span!("bumbledb.query.validate_inputs").entered();
            validate_inputs(schema, query, inputs)?;
        }
        timings.validate_inputs_micros = elapsed_micros(phase_start);
        allocations.validate_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut normalized = {
            let _span = tracing::debug_span!(
                "bumbledb.query.normalize",
                vars = query.variables.len(),
                clauses = query.clauses.len()
            )
            .entered();
            normalize_query(self, schema, query)?
        };
        timings.normalize_micros = elapsed_micros(phase_start);
        allocations.normalize = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let encoded_inputs = {
            let _span = tracing::debug_span!(
                "bumbledb.query.encode_inputs",
                inputs = normalized.inputs.len()
            )
            .entered();
            encode_inputs(self, schema, &normalized, inputs)?
        };
        timings.encode_inputs_micros = elapsed_micros(phase_start);
        allocations.encode_inputs = allocation_delta_since(phase_alloc_start);

        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let image = {
            let _span = tracing::debug_span!("bumbledb.query.image").entered();
            self.query_images.get_or_build_scoped(
                self,
                schema,
                query_image_scope_for_query(schema, &normalized),
            )?
        };
        timings.query_image_micros = elapsed_micros(phase_start);
        allocations.query_image = allocation_delta_since(phase_alloc_start);

        let query_image_cache = self.query_images.diagnostics();
        let phase_start = Instant::now();
        let phase_alloc_start = allocation::snapshot();
        let mut plan = plan_query(schema, &mut normalized, image.as_ref(), query_image_cache)?;
        timings.plan_micros = elapsed_micros(phase_start);
        allocations.plan = allocation_delta_since(phase_alloc_start);
        plan.summary.timings = timings;
        plan.summary.allocations = allocations;
        tracing::debug!(variable_order = ?plan.summary.variable_order, nodes = plan.summary.free_join.nodes.len(), "free join query planned");
        let mut sink = OutputSink::new(&plan.summary.free_join.output);

        let execute_start = Instant::now();
        let execute_alloc_start = allocation::snapshot();
        execute_free_join(
            image.as_ref(),
            self,
            &normalized,
            &encoded_inputs,
            &mut plan,
            &mut sink,
        )?;
        plan.summary.timings.execute_micros = elapsed_micros(execute_start);
        plan.summary.allocations.execute = allocation_delta_since(execute_alloc_start);

        let columns = result_columns(&normalized);
        let sink_finish_start = Instant::now();
        let sink_finish_alloc_start = allocation::snapshot();
        let facts = {
            let _span = tracing::debug_span!("bumbledb.query.sink.finish").entered();
            sink.finish(self, &normalized, &mut plan.summary.counters)?
        };
        plan.summary.timings.sink_finish_micros = elapsed_micros(sink_finish_start);
        plan.summary.allocations.sink_finish = allocation_delta_since(sink_finish_alloc_start);
        plan.summary.counters.output_facts = facts.len() as u64;
        finish_timings(&mut plan.summary.timings, total_start);
        let total_alloc = allocation_delta_since(total_alloc_start);
        plan.summary.allocations = plan.summary.allocations.with_total(total_alloc);
        tracing::debug!(?plan.summary.counters, "free join query executed");
        Ok(QueryOutput {
            result: QueryResultSet::new(columns, facts),
            plan: plan.summary,
        })
    }
}
