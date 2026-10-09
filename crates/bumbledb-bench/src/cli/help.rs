use crate::oracle::sqlite::verify::DEFAULT_RANDOM_CASES;

const COMMANDS: &str = "COMMANDS:\n\
    \x20 gen      generate + load both stores into the digest-keyed dir\n\
    \x20 verify   the oracle: families + randomized queries on both engines\n\
    \x20 verify-store  the offline sweeper (Db::verify_store): namespace\n\
    \x20          coherence + global judgments over the committed store\n\
    \x20 bench    the timing run (requires a fresh verify stamp)\n\
    \x20 profile  repeat one real engine read for native stack sampling\n\
    \x20 scenarios non-ledger worlds (joins/graph/olap/points/rings/temporal), gated then timed\n\
    \x20 crud     the OLTP home-turf world: round-trips under matched\n\
    \x20          durability pairs (report-class; writes crud.md + crud.json)\n\
    \x20 lawful   the law home-turf world: judged-law admission vs SQL\n\
    \x20          constraint enforcement (report-class; writes\n\
    \x20          lawful.md + lawful.json)\n\
    \x20 storage  on-disk bytes per corpus scale, both engines\n\
    \x20          (report-class; no timing)\n\
    \x20 writes   write/commit/delete throughput ladder (report-class)\n\
    \x20 curves   scale-curve runner + cold/warm/memoized panel\n\
    \x20          (report-class)\n\
    \x20 heap     heap-arm ladder: frozen-vs-LMDB point reads, admission\n\
    \x20          A/I/R/F/J prefixes (report-class)\n\
    \x20 app-perf regime lane: cold-open, warm, post-write first read,\n\
    \x20          large-result, tenant churn\n\
    \x20 queries  print the versioned query list (QUERIES.md)\n\
    \x20 help     print this text\n";

#[must_use]
pub fn help() -> String {
    format!(
        "bumbledb-bench {}\n\
         \n\
         The benchmark and oracle suite.\n\
         \n\
         USAGE:\n\
         \x20 bumbledb-bench <COMMAND> [FLAGS]\n\
         \n\
         {COMMANDS}\
         \n\
         SHARED FLAGS (gen, verify, verify-store, bench, profile):\n\
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
         \x20 --read-batch N  operations per timed read sample, 1..=16 (default auto)\n\
         \x20                N > 1 reports batch-average quantiles, not per-call tails\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>)\n\
         \x20 --i-am-lying    skip the stamp gate; the report reads UNVERIFIED\n\
         \n\
         PROFILE (diagnostic, not a benchmark score):\n\
         \x20 --family NAME   registered read or scenario query (required)\n\
         \x20 --seconds N     native sampling window, 1..3600 (default 10)\n\
         \x20 --out PATH      fresh workload report directory\n\
         \x20                 complete draw cycles; ledger/calendar need verify stamp\n\
         \x20                 closure/displaced/scenarios: fresh corpus + SQLite gate\n\
         \x20                 scenarios have fixed scale S; displaced retains foreign stream\n\
         \x20                 use cargo --profile profiling\n\
         \n\
         SCENARIOS / CRUD / LAWFUL (the world commands share one flag vocabulary):\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --only a,b      run only these scenarios/families\n\
         \x20 --samples N     measured samples/query   (default 64; crud and\n\
         \x20                 lawful fall back to their registered protocols)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-<command>)\n\
         \n\
         STORAGE:\n\
         \x20 --scales S,M,L  corpus scales            (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      corpus cache root        (default bench-data)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-storage)\n\
         \n\
         WRITES:\n\
         \x20 --scale S|M|L   corpus scale             (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --batches a,b   rows per commit          (default 1,10,100,1000)\n\
         \x20 --samples N     measured samples per cell\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-writes)\n\
         \n\
         CURVES:\n\
         \x20 --scales S,M,L  corpus scales            (default S)\n\
         \x20 --families a,b  run only these families  (default the full roster)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      corpus cache root        (default bench-data)\n\
         \x20 --samples N     measured samples per point\n\
         \x20 --cap-ms N      per-sample SQLite wall-clock cap (default 30000)\n\
         \x20 --warmth        add the cold/warm/memoized panel\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-curves)\n\
         \n\
         HEAP:\n\
         \x20 --scale S|M|L   point-read corpus scale  (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --samples N     measured samples/family  (default 32)\n\
         \x20 --prefixes a,b  posting-count admit prefixes\n\
         \x20                 (default 256,1024,4096,16384)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-heap)\n\
         \n\
         APP-PERF:\n\
         \x20 --scale S|M|L   corpus scale             (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --regimes a,b   warm, cold-open, post-write, large-result,\n\
         \x20                 tenant-churn (default all)\n\
         \x20 --samples N     measured samples per regime cell\n\
         \x20 --tenants N     churn tenant count       (default 8, min 2)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-app-perf)\n\
         \n\
         SHARED-MACHINE BOOST:\n\
         \x20 BUMBLEDB_BENCH_BOOST=1  claim user-interactive QoS before any\n\
         \x20                 measurement subcommand (macOS); on Linux set and\n\
         \x20                 verify absolute nice -10 (needs priority permission)\n\
         \x20                 and stamp shared_machine provenance. Default off\n\
         \x20                 (unset/0); bench-night.sh always sets it.\n\
         \n\
         EXIT CODES: 0 ok / gate won; 1 verify mismatch, store findings, or\n\
         gate loss; 2 usage.\n",
        env!("CARGO_PKG_VERSION"),
        DEFAULT_RANDOM_CASES,
    )
}
