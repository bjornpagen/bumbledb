use super::*;

#[test]
fn job_queries_typecheck_against_job_schema() -> Result<(), Box<dyn std::error::Error>> {
    let schema = job_schema();
    for query in job_queries() {
        (query.build)(&schema)?;
    }
    Ok(())
}

#[test]
fn job_dataset_runs_against_minimal_csv_export() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    for (file, contents) in [
        ("aka_name.csv", "1,1,Jane Alias,,,,\n"),
        ("aka_title.csv", "1,1,Series Alias,,2,2012,,0,0,60,,\n"),
        ("cast_info.csv", "1,1,1,1,,1,1\n2,1,2,1,,1,1\n"),
        ("char_name.csv", "1,Heroine,,0,,,\n"),
        ("comp_cast_type.csv", "1,cast\n2,complete\n"),
        ("company_name.csv", "1,Acme,[us],0,,,\n"),
        ("company_type.csv", "1,production companies\n"),
        ("complete_cast.csv", "1,1,1,2\n"),
        (
            "info_type.csv",
            "1,top 250 rank\n2,rating\n3,release dates\n",
        ),
        ("keyword.csv", "1,character-name-in-title,\n2,hero,\n"),
        ("kind_type.csv", "1,movie\n2,tv series\n"),
        ("link_type.csv", "1,sequel\n"),
        ("movie_companies.csv", "1,1,1,1,\n2,2,1,1,\n"),
        ("movie_info.csv", "1,1,3,USA:2011,\n"),
        (
            "movie_info_idx.csv",
            "1,1,1,10,\n2,1,2,7.0,\n3,2,2,2.5,\n4,1,3,USA:2011,\n",
        ),
        ("movie_keyword.csv", "1,1,1\n2,1,2\n"),
        ("movie_link.csv", "1,1,2,1\n"),
        ("name.csv", "1,Jane Doe,,0,m,,,\n"),
        ("person_info.csv", "1,1,3,bio,note\n"),
        ("role_type.csv", "1,actor\n"),
        (
            "title.csv",
            "1,Series One,,2,2012,0,,0,0,60,\n2,Series Two,,2,2006,0,,0,0,0,\n",
        ),
    ] {
        std::fs::write(dir.path().join(file), contents)?;
    }

    let limited = job_dataset(dir.path(), Some(1))?;
    let dataset = job_dataset(dir.path(), None)?;
    assert_eq!(dataset.name, "job");
    assert_eq!(dataset.queries.len(), 8);
    let Some(limited_source) = limited.fact_source.as_ref() else {
        return Err("limited JOB dataset should be streaming".into());
    };
    let Some(full_source) = dataset.fact_source.as_ref() else {
        return Err("full JOB dataset should be streaming".into());
    };
    let limited_facts = stream_facts(limited_source, |_| Ok(()))?;
    let full_facts = stream_facts(full_source, |_| Ok(()))?;
    assert!(limited_facts < full_facts);

    let config = crate::Config {
        scale: 10,
        open_limit: None,
        repeats: 1,
        warmup: 0,
        datasets: vec!["job".to_owned()],
        queries: Vec::new(),
        imdb_dir: None,
        job_dir: None,
        tpch_dir: None,
        lahman_dir: None,
        ldbc_dir: None,
        preset: None,
        trace: false,
        trace_output: None,
        trace_format: crate::TraceFormat::Fmt,
        format: crate::OutputFormat::Json,
        compare_mode: crate::CompareMode::Materialized,
        cache_mode: crate::CacheMode::PreparedPlan,
        fail_gates: false,
    };
    let results = crate::run_dataset(dataset, &config)?;
    assert_eq!(results.len(), 8);
    assert!(results.iter().all(|result| result.facts >= 1));
    Ok(())
}
