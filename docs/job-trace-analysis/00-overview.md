# JOB Trace Analysis Overview

Historical note: this analysis predates the v2 schema/query cleanup. It may refer to Datalog, primary-key internals, current-row payloads, or other v1 implementation details that no longer exist in the current architecture.

## Inputs

| Artifact | Path |
|---|---|
| Results JSON | `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-results.json` |
| Raw trace JSONL | `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-trace.jsonl` |
| Raw trace summary | `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-trace-summary.txt` |

The raw trace contains 92,799,337 JSONL lines and is 28G. The TSV files in this directory are pre-extracted fact tables keyed by query so reports do not need to rescan the raw trace.

## Reports

| Query | Report |
|---|---|
| `job_broad_cast_keyword_company` | `01-job_broad_cast_keyword_company.md` |
| `job_broad_movie_info_star` | `02-job_broad_movie_info_star.md` |
| `job_q01_top_production` | `03-job_q01_top_production.md` |
| `job_q09_voice_us_actor` | `04-job_q09_voice_us_actor.md` |
| `job_q16_character_title_us` | `05-job_q16_character_title_us.md` |
| `job_q24_voice_keyword_actor` | `06-job_q24_voice_keyword_actor.md` |
| `job_movie_link_bridge` | `07-job_movie_link_bridge.md` |
| `job_q33_linked_series_companies` | `08-job_q33_linked_series_companies.md` |

## Cross-Query Waste Buckets

| Bucket | Quantitative evidence | Primary reports |
|---|---|---|
| Whole-schema cold query-image build | `job_broad_cast_keyword_company` spends 899.1 ms in image build, with 32.72M allocation calls, 2.591 GB allocated, and 984.7 MB net live in `query_image`. The top three relation image spans alone are 665 ms. | `01` |
| Cold LFTJ atom build heap churn | `job_q09_voice_us_actor` spends 682.8 ms in `lftj_build`, with 16.58M calls and 2.649 GB allocated. `job_q24_voice_keyword_actor` spends 33.6 ms in `lftj_build`, with 920k calls and 107.7 MB allocated. | `04`, `06` |
| Steady LFTJ execution cost | `job_q09_voice_us_actor` samples spend 8.894 s of 8.899 s in `lftj.execute` across 30 runs. `job_q24_voice_keyword_actor` samples spend 74.78 ms of 80.01 ms in `lftj.execute`. | `04`, `06` |
| Direct count kernel heap and probe work | `job_broad_movie_info_star` spends 98.53% of sample busy time in direct dispatch and first execution has 35,728 execute allocation calls. `job_movie_link_bridge` samples still do 4,080 prefix-count probes per run. | `02`, `07` |
| Late static-empty fast path | `job_q33_linked_series_companies` loses to SQLite at 91 us vs 65 us after the proof is cached. Static-empty samples still pay normalization, image lookup, debug-string cache key construction, cache lookup, and result metadata. | `03`, `05`, `08` |
| Generic planner churn before direct/runtime shortcuts | `job_movie_link_bridge` first execution spends 42.7% in planning and 18,247 plan allocation calls. `job_broad_movie_info_star` first execution spends 17.9% in planning and 17,350 plan allocation calls. | `02`, `07` |

## Allocation Kill List

| Priority | Kill | Expected effect |
|---:|---|---|
| 1 | Replace query-image per-cell `Vec<Vec<u8>>` conversion with direct flat segment decoding into typed column arrays. | Targets the single largest allocation source: 32.72M calls and 2.591 GB in `query_image` for the cold image build. |
| 2 | Make query-image acquisition relation-scoped or direct-kernel scoped. | Avoids building 21 relation images for queries that need only a subset or a direct count index path. |
| 3 | Delete LFTJ temporary `Vec<Vec<u8>>` atom rows and use width-specialized column builders. | Targets 16.58M calls and 2.649 GB in q09 plus 920k calls and 107.7 MB in q24. |
| 4 | Replace per-row `BTreeMap<usize, Vec<u8>>` atom extraction with dense slot storage or `SmallVec`. | Removes tree-node allocation and repeated byte copies in `scan_filter_copy`, the dominant cold LFTJ build subphase. |
| 5 | Stream indexed-prefix rows directly into LFTJ builders instead of returning `Vec<Vec<Vec<u8>>>`. | Eliminates nested intermediate row storage for indexed atom paths such as q09 `Name` and q24 `Keyword`. |
| 6 | Stream distinct central values in direct factorized count instead of `BTreeSet<Vec<u8>>`. | Attacks `job_broad_movie_info_star` direct-kernel heap churn: 35,728 execute calls and 834 KB in the first execution, plus sample CPU. |
| 7 | Add direct prefix-count APIs or cached prefix counts for direct kernels. | Reduces recurring direct dispatch CPU, especially `job_movie_link_bridge` at 4,080 prefix-count probes per sample. |
| 8 | Move static-empty cached result lookup before normalization for no-input/static-literal queries. | Should cut most of the 82-91 us static-empty sample floor and make q33 beat SQLite. |
| 9 | Replace `format!("{query:?}")` cache keys with structural fixed-size fingerprints. | Removes string/debug allocation and CPU from prepared-plan and static-empty cache hits. |
| 10 | Add compact direct plans before generic free-join planning. | Removes avoidable planner work for direct count kernels: 18k plan calls in `job_movie_link_bridge` and 17k in `job_broad_movie_info_star`. |
| 11 | Borrow or scalar-copy LFTJ keys during traversal instead of cloning owned key bytes. | Targets q09 steady-state transient allocation: 645k `lftj_execute` calls and 97.4 MB allocated in first execution. |
| 12 | Specialize global count and tiny dedup sinks. | Removes small but repeated aggregate/project sink overhead for direct counts and low-output LFTJ queries. |

## Immediate RCA Conclusions

- The largest cold allocation cliff is not query execution. It is image/build representation: full schema image construction and LFTJ atom trie construction both convert flat encoded bytes into many heap-owned byte vectors.
- The largest steady-state CPU cliff is query-specific. q09 is real LFTJ traversal; movie-info-star is direct factorized count probe work; static-empty queries are frontend/cache overhead.
- Allocation stats in `job-results.json` are first-execution stats. They are still product-relevant cold-path data, but they are not per-sample allocation profiles.
- The fastest path to visible wins is to kill heap-owned encoded-value containers first, then move direct/static fast paths before generic planning and normalization.
