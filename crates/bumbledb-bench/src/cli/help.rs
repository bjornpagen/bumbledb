use crate::verify::DEFAULT_RANDOM_CASES;

/// The command list — static usage data, no parameters ([`help`]
/// stitches it between the version header and the flag sections).
const COMMANDS: &str = "COMMANDS:\n\
    \x20 gen      generate + load both stores into the digest-keyed dir\n\
    \x20 verify   the oracle: families + randomized queries on both engines\n\
    \x20 verify-store  the offline sweeper (Db::verify_store): namespace\n\
    \x20          coherence + global judgments over the committed store\n\
    \x20 bench    the timing run (requires a fresh verify stamp)\n\
    \x20 trace    one traced warm+cold pair for one family\n\
    \x20 scenarios non-ledger worlds (joins/graph/olap/points/rings/temporal), gated then timed\n\
    \x20 crud     the OLTP home-turf world: round-trips under matched\n\
    \x20          durability pairs (report-class; writes crud.md + crud.json)\n\
    \x20 lawful   the law home-turf world: judged-law admission vs SQL\n\
    \x20          constraint enforcement (report-class; writes\n\
    \x20          lawful.md + lawful.json)\n\
    \x20 sweep-commit  the T8 commit-size sweep: judgment spans by\n\
    \x20          touched-parent count, delta vs key-sorted probe order\n\
    \x20          (ephemeral windowed twins; needs --features obs)\n\
    \x20 merge    min-of-runs table from N run dirs' report.json\n\
    \x20 storage  on-disk bytes per corpus scale, both engines\n\
    \x20          (report-class; no timing)\n\
    \x20 writes   write/commit/delete throughput ladder across\n\
    \x20          durability lanes (report-class)\n\
    \x20 curves   scale-curve runner + cold/warm/memoized panel\n\
    \x20          (report-class)\n\
    \x20 churn    long-lived churn: degradation time series, both engines\n\
    \x20 queries  print the versioned query list (QUERIES.md)\n\
    \x20 help     print this text\n";

/// The usage text.
#[must_use]
pub fn help() -> String {
    format!(
        "bumbledb-bench {}\n\
         \n\
         The benchmark and oracle suite (docs/architecture/60-validation.md).\n\
         \n\
         USAGE:\n\
         \x20 bumbledb-bench <COMMAND> [FLAGS]\n\
         \n\
         {COMMANDS}\
         \n\
         SHARED FLAGS (gen, verify, verify-store, bench, trace):\n\
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
         \x20 --ephemeral     time against Db::ephemeral stores (NOSYNC;\n\
         \x20                 the in-memory characterization lane)\n\
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
         SCENARIOS / CRUD / LAWFUL (the world commands share one flag vocabulary):\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --only a,b      run only these scenarios/families\n\
         \x20 --samples N     measured samples/query   (default 64; crud and\n\
         \x20                 lawful fall back to their registered protocols)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-<command>)\n\
         \n\
         SWEEP-COMMIT:\n\
         \x20 --sizes a,b,c   touched-parent counts (default 4,16,64,256,1024,4096)\n\
         \x20 --samples N     commits per (size, order) cell (default 8, max 48)\n\
         \x20 --seed N        draw seed                (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \n\
         STORAGE:\n\
         \x20 --scales S,M,L  corpus scales            (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      corpus cache root        (default bench-data)\n\
         \x20 --churn-dir PATH  scratch root for the churn ladder (default off)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-storage)\n\
         \n\
         WRITES:\n\
         \x20 --scale S|M|L   corpus scale             (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --lanes a,b     durability lanes, run order: durable, nosync\n\
         \x20                 (default nosync,durable — fsync shadows last)\n\
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
         CHURN:\n\
         \x20 --scale S|M|L   corpus scale             (default S)\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --cycles N      total cycles             (default 10000)\n\
         \x20 --sample-every N  probe stride, cycles   (default 250)\n\
         \x20 --vacuum-every N  SQLite VACUUM stride   (default 500)\n\
         \x20 --analyze-every N SQLite ANALYZE stride  (default 500)\n\
         \x20 --runs a,b      run only these runs\n\
         \x20                 (default steady,nosync,delete-heavy)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-churn)\n\
         \x20 report-class; series artifact churn-report.json — never a gate\n\
         \n\
         EXIT CODES: 0 ok / gate won; 1 verify mismatch, store findings, or\n\
         gate loss; 2 usage.\n",
        env!("CARGO_PKG_VERSION"),
        DEFAULT_RANDOM_CASES,
    )
}
