use crate::verify::DEFAULT_RANDOM_CASES;

/// The usage text.
#[must_use]
pub fn help() -> String {
    format!(
        "bumbledb-bench {}\n\
         \n\
         The benchmark and oracle suite (docs/architecture/50-validation.md).\n\
         \n\
         USAGE:\n\
         \x20 bumbledb-bench <COMMAND> [FLAGS]\n\
         \n\
         COMMANDS:\n\
         \x20 gen      generate + load both stores into the digest-keyed dir\n\
         \x20 verify   the oracle: families + randomized queries on both engines\n\
         \x20 bench    the timing run (requires a fresh verify stamp)\n\
         \x20 trace    one traced warm+cold pair for one family\n\
         \x20 scenarios non-ledger worlds (joins/graph/olap/points), gated then timed\n\
         \x20 merge    min-of-runs table from N run dirs' report.json\n\
         \x20 queries  print the versioned query list (QUERIES.md)\n\
         \x20 help     print this text\n\
         \n\
         SHARED FLAGS (gen, verify, bench, trace):\n\
         \x20 --scale S|M|L   corpus scale        (default S)\n\
         \x20 --seed N        corpus seed         (default 1)\n\
         \x20 --dir PATH      corpus cache root   (default bench-data)\n\
         \n\
         VERIFY:\n\
         \x20 --cases N       randomized cases    (default {})\n\
         \n\
         BENCH:\n\
         \x20 --families a,b  run only these families (verdict becomes PARTIAL)\n\
         \x20 --samples N     measured samples per read family (default 256)\n\
         \x20 --trace         capture one traced warm+cold sample per family\n\
         \x20 --alloc         allocation windows (needs the obs feature build)\n\
         \x20 --proxy-per-rep per-sample GHz stamps + normalized p50 (confirm runs)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>)\n\
         \x20 --i-am-lying    skip the stamp gate; the report reads UNVERIFIED\n\
         \n\
         TRACE:\n\
         \x20 --family NAME   the family to trace (required)\n\
         \n\
         MERGE:\n\
         \x20 merge DIR [DIR ...]   run directories holding report.json\n\
         \n\
         SCENARIOS:\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --only a,b      run only these scenarios (joins graph olap points)\n\
         \x20 --samples N     measured samples/query   (default 64)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-scenarios)\n\
         \n\
         EXIT CODES: 0 ok / gate won; 1 verify mismatch or gate loss; 2 usage.\n",
        env!("CARGO_PKG_VERSION"),
        DEFAULT_RANDOM_CASES,
    )
}
