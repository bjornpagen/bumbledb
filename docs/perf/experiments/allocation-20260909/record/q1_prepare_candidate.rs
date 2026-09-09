// Reuse the exact saved queries, schema and owner observers from audit 1.
include!("/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/q1_saved.rs");

#[test]
#[ignore = "preparation counters and owners before any execution; no timings"]
fn saved_preparation() {
    assert!(cfg!(feature = "alloc-counter"));
    let path = std::env::var_os("BUMBLEDB_Q1_DB").unwrap();
    let path = Path::new(&path);
    let work = crate::WorkContext::new();
    for family in ["range", "triangle"] {
        let db = open(path, &work);
        let query = query(family == "range");
        crate::alloc_counter::reset();
        let prepared = db.prepare(&query, work.clone()).unwrap();
        let alloc = crate::alloc_counter::snapshot().window;
        println!("PREPARE {family} alloc={alloc:?}");
        owners(&format!("prepare-{family}"), &prepared, &Answers::new());
    }
    println!("PASS saved preparation; two queries before any execution");
}
