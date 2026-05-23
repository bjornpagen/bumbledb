fn run_dataset(
    dataset: Dataset,
    config: &Config,
) -> Result<Vec<BenchmarkRunResult>, Box<dyn std::error::Error>> {
    let selected_queries = dataset
        .queries
        .into_iter()
        .filter(|query| {
            config.queries.is_empty() || config.queries.iter().any(|name| name == query.name)
        })
        .collect::<Vec<_>>();
    if selected_queries.is_empty() {
        return Ok(Vec::new());
    }

    let format = config.format;
    if format.includes_text() {
        println!("== {} ==", dataset.name);
        match &dataset.fact_source {
            Some(_) => println!("facts=streaming"),
            None => println!("facts={}", dataset.facts.len()),
        }
        println!("queries={}", selected_queries.len());
    }

    let bumble_dir = tempfile::tempdir()?;
    let bumble_env = Environment::open(bumble_dir.path())?;
    let bumble_schema = StorageSchema::new(dataset.schema.clone(), bumble_env.max_key_size())?;

    if dataset.fact_source.is_some() {
        eprintln!(
            "[bench:{}] loading bumbledb from streaming source",
            dataset.name
        );
    }
    let bumble_load = timed(|| match &dataset.fact_source {
        Some(source) => bumble_env.write(|txn| {
            txn.bulk_load_streaming(|txn| {
                let mut inserted = 0;
                open::stream_facts(source, |fact| {
                    if txn.insert(&bumble_schema, fact)? == bumbledb_lmdb::InsertOutcome::Inserted {
                        inserted += 1;
                    }
                    Ok(())
                })?;
                Ok::<usize, Box<dyn std::error::Error>>(inserted)
            })
        }),
        None => bumble_env
            .bulk_load(&bumble_schema, dataset.facts.clone())
            .map(|report| report.facts_inserted)
            .map_err(Into::into),
    })?;
    if format.includes_text() {
        println!("load.bumbledb={:?}", bumble_load.elapsed);
    }
    if dataset.fact_source.is_some() {
        eprintln!(
            "[bench:{}] bumbledb load complete facts={} elapsed={:?}",
            dataset.name, bumble_load.value, bumble_load.elapsed
        );
    }
    let query_image_stats = if dataset.fact_source.is_some() {
        QueryImageBenchStats::empty()
    } else {
        let stats = bumble_env.query_image_stats(&bumble_schema)?;
        QueryImageBenchStats {
            relation_count: stats.relation_count,
            fact_count: stats.fact_count,
            encoded_column_bytes: stats.encoded_column_bytes,
            sorted_trie_bytes: stats.sorted_trie_bytes,
            build_micros: stats.build_micros,
        }
    };
    if format.includes_text() {
        if dataset.fact_source.is_some() {
            println!("query_image eager_build=skipped_for_streaming_dataset");
        } else {
            println!(
                "query_image build_micros={}",
                query_image_stats.build_micros,
            );
        }
    }

    let sqlite_dir = tempfile::tempdir()?;
    let mut sqlite = if dataset.fact_source.is_some() {
        Connection::open(sqlite_dir.path().join("sqlite-bench.db"))?
    } else {
        Connection::open_in_memory()?
    };
    sqlite.execute_batch(dataset.sqlite_schema)?;
    if dataset.fact_source.is_some() {
        eprintln!(
            "[bench:{}] loading sqlite from streaming source",
            dataset.name
        );
    }
    let sqlite_load = timed(|| match &dataset.fact_source {
        Some(source) => open::insert_sqlite_streaming(source, &mut sqlite),
        None => (dataset.sqlite_insert)(&sqlite, &dataset.facts).map(|()| dataset.facts.len()),
    })?;
    if format.includes_text() {
        println!("load.sqlite={:?}", sqlite_load.elapsed);
    }
    if dataset.fact_source.is_some() {
        eprintln!(
            "[bench:{}] sqlite load complete facts={} elapsed={:?}",
            dataset.name, sqlite_load.value, sqlite_load.elapsed
        );
    }

    let mut results = Vec::new();
    for query in selected_queries {
        let typed = (query.build)(bumble_schema.descriptor())?;
        let inputs = InputBindings::from_values(query.inputs.clone());
        let params = query.sqlite_params.clone();

        let materialized_once =
            timed(|| bumble_env.read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs)))?;
        let materialized_output = materialized_once.value;
        let bumble_cold_execution = materialized_once.elapsed;
        let bumble_output = materialized_output.clone();
        let correctness_mode = correctness_mode(&typed);
        let sqlite_correctness = timed(|| sqlite_result_facts(&mut sqlite, query.sqlite, &params))?;
        let sqlite_correctness_execution = sqlite_correctness.elapsed;
        let sqlite_expected = sorted_sql_facts(sqlite_correctness.value);
        let bumbledb_actual = sorted_sql_facts(bumbledb_sql_facts(&materialized_output)?);
        if bumbledb_actual != sqlite_expected {
            return Err(format!(
                "{}:{} result mismatch bumbledb={:?} sqlite={:?}",
                dataset.name, query.name, bumbledb_actual, sqlite_expected
            )
            .into());
        }
        let sqlite_once = timed(|| sqlite_count(&mut sqlite, query.sqlite, &params))?;
        let sqlite_cold_execution = sqlite_once.elapsed;
        let sqlite_once = sqlite_once.value;
        if materialized_output.result.facts.len() != sqlite_once {
            return Err(format!(
                "{}:{} fact-count mismatch after value match bumbledb={} sqlite={}",
                dataset.name,
                query.name,
                materialized_output.result.facts.len(),
                sqlite_once
            )
            .into());
        }

        let (bumble_warmup, _) = timed_bumbledb_samples(config.warmup, || {
            let output = bumble_env.read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs))?;
            black_box(output.result.facts.len());
            Ok::<_, bumbledb_lmdb::Error>(output.plan)
        })?;
        let sqlite_warmup = timed_samples(config.warmup, || {
            let facts = sqlite_count(&mut sqlite, query.sqlite, &params)?;
            black_box(facts);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;

        let (bumble_samples, bumble_sample_cache_hits) = timed_bumbledb_samples(config.repeats, || {
            let output = bumble_env.read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs))?;
            black_box(output.result.facts.len());
            Ok::<_, bumbledb_lmdb::Error>(output.plan)
        })?;
        let sqlite_samples = timed_samples(config.repeats, || {
            let facts = sqlite_count(&mut sqlite, query.sqlite, &params)?;
            black_box(facts);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;

        let result = benchmark_result(
            dataset.name,
            &query,
            &bumble_output,
            bumble_sample_cache_hits,
            correctness_mode,
            QueryTimingSamples {
                bumbledb_correctness_execution: materialized_once.elapsed,
                sqlite_correctness_execution,
                bumbledb_cold_execution: bumble_cold_execution,
                sqlite_cold_execution,
                bumbledb_warmup: bumble_warmup,
                sqlite_warmup,
                bumbledb_samples: bumble_samples,
                sqlite_samples,
            },
            query_image_stats,
        );
        emit_profile_summary(dataset.name, query.name, &bumble_output);
        if format.includes_text() {
            println!(
                "query={} facts={} sink_emit_calls={} encoded_project_facts_seen={} lftj_next_calls={} bumbledb_cold_execution={:?} bumbledb_samples={} bumbledb_avg={:?} sqlite_cold_execution={:?} sqlite_samples={} sqlite_avg={:?} gate={}",
                query.name,
                bumble_output.result.facts.len(),
                result.counters.sink_emit_calls,
                result.counters.encoded_project_facts_seen,
                result.counters.lftj_next_calls,
                bumble_cold_execution,
                result.bumbledb_samples.samples,
                result.bumbledb_avg,
                sqlite_cold_execution,
                result.sqlite_samples.samples,
                result.sqlite_avg,
                if result.gate.passed { "pass" } else { "fail" },
            );
            print_explain(&bumble_output.explain());
            for note in &result.gate.notes {
                println!("  gate_note: {note}");
            }
        }
        results.push(result);
    }

    Ok(results)
}
