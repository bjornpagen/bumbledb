//! The instrumentation-point registry: every span and event is a
//! [`TracePoint`]. Labels derive as [`Category::label`] does — Chrome
//! export still prints names; call sites cannot typo-drift a string.
//! Payloads are [`TraceArgs`]: unused is not `0`.

use super::Category;

/// Payload of one recorded span or point. `None` is the unset/aborted
/// default — distinct from [`Self::Count`]`(0)` (a completed empty pass)
/// and from [`Self::Flag`]`(false)` (an explicit negative outcome).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceArgs {
    #[default]
    None,
    /// One witnessed quantity (count, id, length, rung, …).
    Count(u64),
    /// Two witnessed quantities.
    Pair(u64, u64),
    /// An explicit boolean outcome (committed, hit, …).
    Flag(bool),
}

impl TraceArgs {
    /// Chrome-trace `a0`: `None` and `Flag(false)` both print `0`, but
    /// [`TraceEvent::args`] still distinguishes them.
    #[must_use]
    pub const fn a0(self) -> u64 {
        match self {
            Self::None => 0,
            Self::Count(n) | Self::Pair(n, _) => n,
            Self::Flag(b) => b as u64,
        }
    }

    /// Chrome-trace `a1`: only [`Self::Pair`] carries a second slot.
    #[must_use]
    pub const fn a1(self) -> u64 {
        match self {
            Self::Pair(_, n) => n,
            Self::None | Self::Count(_) | Self::Flag(_) => 0,
        }
    }
}

macro_rules! trace_points {
    ($(
        $(#[$meta:meta])*
        $variant:ident => $label:literal, $cat:ident
    ),* $(,)?) => {
        /// One instrumentation point. Category and Chrome label are
        /// derived from the variant — never passed beside a string name.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[allow(
            non_camel_case_types,
            reason = "variants are the former registry constants (PREPARE, VALIDATE, …)"
        )]
        pub enum TracePoint {
            $(
                $(#[$meta])*
                $variant,
            )*
            /// One rule of the execute loop. Index is bounded by
            /// [`crate::ir::MAX_RULES`].
            Rule(u8),
            /// Executor phase accumulator: `JOIN_PHASE[phase][min(node, 8)]`.
            JoinPhase { phase: u8, node: u8 },
        }

        impl TracePoint {
            /// Chrome-trace `name` field. Stable ASCII, derived like
            /// [`Category::label`].
            #[must_use]
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label,)*
                    Self::Rule(i) => RULE_LABELS[sat_idx(i, RULE_LABELS.len())],
                    Self::JoinPhase { phase, node } => {
                        JOIN_PHASE_LABELS[sat_idx(phase, JOIN_PHASE_LABELS.len())]
                            [sat_idx(node, JOIN_PHASE_LABELS[0].len())]
                    }
                }
            }

            /// The point's category, derived — call sites never pass it.
            #[must_use]
            pub const fn category(self) -> Category {
                match self {
                    $(Self::$variant => Category::$cat,)*
                    Self::Rule(_) => Category::Execute,
                    Self::JoinPhase { .. } => Category::Phase,
                }
            }
        }

        impl core::fmt::Display for TracePoint {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(self.label())
            }
        }

        /// Registry constants — the former string names, now points.
        pub mod names {
            use super::TracePoint;
            $(
                $(#[$meta])*
                pub const $variant: TracePoint = TracePoint::$variant;
            )*

            /// One rule of the execute loop. Index is bounded by
            /// [`crate::ir::MAX_RULES`].
            pub const RULE: [TracePoint; 16] = [
                TracePoint::Rule(0),
                TracePoint::Rule(1),
                TracePoint::Rule(2),
                TracePoint::Rule(3),
                TracePoint::Rule(4),
                TracePoint::Rule(5),
                TracePoint::Rule(6),
                TracePoint::Rule(7),
                TracePoint::Rule(8),
                TracePoint::Rule(9),
                TracePoint::Rule(10),
                TracePoint::Rule(11),
                TracePoint::Rule(12),
                TracePoint::Rule(13),
                TracePoint::Rule(14),
                TracePoint::Rule(15),
            ];

            /// Phase table: `JOIN_PHASE[phase][min(node, 8)]`.
            pub const JOIN_PHASE: [[TracePoint; 9]; 7] = {
                let mut table = [[TracePoint::JoinPhase { phase: 0, node: 0 }; 9]; 7];
                let mut phase = 0u8;
                while phase < 7 {
                    let mut node = 0u8;
                    while node < 9 {
                        table[phase as usize][node as usize] =
                            TracePoint::JoinPhase { phase, node };
                        node += 1;
                    }
                    phase += 1;
                }
                table
            };
        }
    };
}

const fn sat_idx(i: u8, len: usize) -> usize {
    let i = i as usize;
    if i < len { i } else { len - 1 }
}

trace_points! {
    /// The whole prepare pipeline.
    PREPARE => "prepare", Prepare,
    /// IR validation.
    VALIDATE => "validate", Prepare,
    /// One rule-set lowering.
    VALIDATE_LOWER => "validate_lower", Prepare,
    /// The signature-sealing pass.
    VALIDATE_SEAL => "validate_seal", Prepare,
    /// The strict per-rule roster pass.
    VALIDATE_RULES => "validate_rules", Prepare,
    /// Normalization.
    NORMALIZE => "normalize", Prepare,
    /// One rule's comparison placement.
    PLACE_COMPARISONS => "place_comparisons", Prepare,
    /// One rule's statically-empty constant fold.
    NORMALIZE_FOLD => "normalize_fold", Prepare,
    /// Key-probe-vs-join classification.
    CLASSIFY => "classify", Prepare,
    /// Statistics reads.
    STATS => "stats", Prepare,
    /// The exhaustive left-deep DP.
    PLAN_DP => "plan_dp", Prepare,
    /// binary2fj + factor + plan validation.
    LOWER => "lower", Prepare,
    /// COLT construction at prepare.
    BUILD_COLTS => "build_colts", Prepare,
    /// Densifying participating occurrences + Allen residuals.
    PLAN_DENSIFY => "plan_densify", Prepare,
    /// The exhaustive left-deep subset DP's table-fill pass.
    PLAN_FILL => "plan_fill", Prepare,
    /// One planner row-count read.
    RELATION_ROWS => "relation_rows", Prepare,
    /// One resolution of the per-field distinct-count ladder.
    DISTINCT_LADDER => "distinct_ladder", Prepare,
    /// One prepared execution.
    EXECUTE => "execute", Execute,
    /// The interior preamble under the execute span.
    INTERIORS => "interiors", Execute,
    /// The rec least-fixpoint under the execute span.
    REACH => "reach", Execute,
    /// One reach round under the REACH span.
    FIXPOINT_ROUND => "fixpoint_round", Execute,
    /// Parameter binding.
    BIND_PARAMS => "bind_params", Execute,
    /// Filter-constant resolution.
    RESOLVE_FILTERS => "resolve_filters", Execute,
    /// The per-occurrence view loop.
    VIEWS => "views", Execute,
    /// One occurrence's view rebuild.
    VIEW_BUILD => "view_build", Execute,
    /// The warm memo fast path fired.
    VIEW_MEMO_HIT => "view_memo_hit", Execute,
    /// The occurrence-dedup path fired.
    VIEW_DEDUP => "view_dedup", Execute,
    /// The Free Join executor.
    JOIN => "join", Execute,
    /// Sink finalization into the result buffer.
    FINALIZE => "finalize", Execute,
    /// The key-probe access path.
    KEY_PROBE => "key_probe", Execute,
    /// One snapshot point read.
    POINT_READ => "point_read", Storage,
    /// The whole selection-probe loop.
    SELECTIONS => "selections", Execute,
    /// One occurrence's selection-level probe.
    SELECT_PROBE => "select_probe", Execute,
    /// Image found in the shared cache.
    CACHE_HIT => "cache_hit", Cache,
    /// A full image decode.
    IMAGE_BUILD => "image_build", Image,
    /// An append-path image extension.
    IMAGE_APPEND => "image_append", Image,
    /// The exact per-column distinct counting pass.
    IMAGE_DISTINCTS => "image_distincts", Image,
    /// The columnar fact decode inside an image build.
    DECODE_BATCH => "decode_batch", Image,
    /// An untouched relation's image carried forward.
    CACHE_CARRY => "cache_carry", Cache,
    /// Lost the insert race; adopted the winner's image.
    CACHE_ADOPT => "cache_adopt", Cache,
    /// Old-generation reader built without caching.
    CACHE_QUERY_LOCAL => "cache_query_local", Cache,
    /// One COLT node forced.
    COLT_FORCE => "colt_force", Execute,
    /// One dictionary resolution in finalize.
    DICT_RESOLVE => "dict_resolve", Execute,
    /// One scalar String param bind served from the per-slot word memo.
    PARAM_WORD_MEMO => "param_word_memo", Execute,
    /// A `str` literal latched.
    LITERAL_LATCH => "literal_latch", Execute,
    /// One state-changing commit.
    COMMIT => "commit", Commit,
    /// A commit that netted to nothing.
    COMMIT_NOOP => "commit_noop", Commit,
    /// Phase 1. (facts deleted)
    APPLY_DELETES => "apply_deletes", Commit,
    /// Phase 2. (facts inserted)
    APPLY_INSERTS => "apply_inserts", Commit,
    /// Phase 3, containment source side.
    JUDGMENT_SOURCE => "judgment_source", Commit,
    /// Phase 3, containment target side.
    JUDGMENT_TARGET => "judgment_target", Commit,
    /// Phase 3, capacity statements.
    JUDGMENT_CAPACITIES => "judgment_capacities", Commit,
    /// Phase 4.
    COUNTERS_FLUSH => "counters_flush", Commit,
    /// Phase 5: the LMDB commit alone.
    LMDB_COMMIT => "lmdb_commit", Commit,
    /// One bounded retry of the durability boundary.
    COMMIT_SYNC_RETRY => "commit_sync_retry", Commit,
    /// `Db::compact`'s durability chain completed.
    COMPACT_DURABLE => "compact_durable", Commit,
    /// `Db::create`'s birth dirent chain completed.
    CREATE_DURABLE => "create_durable", Commit,
    /// One `Db::write`, closure plus commit.
    WRITE_TXN => "write_txn", Commit,
    /// The whole `Db::verify_store` sweep.
    VERIFY_STORE => "verify_store", Storage,
    /// The `F` (fact) namespace pass.
    VERIFY_FACTS => "verify_facts", Storage,
    /// The `M` (membership) namespace pass.
    VERIFY_MEMBERSHIP => "verify_membership", Storage,
    /// The `U` (determinant) namespace pass.
    VERIFY_DETERMINANTS => "verify_determinants", Storage,
    /// The `R` (reverse-edge) namespace pass.
    VERIFY_REVERSE => "verify_reverse", Storage,
    /// The `Q` (fresh marks) namespace pass.
    VERIFY_MARKS => "verify_marks", Storage,
    /// The `S`-counter reconciliation pass.
    VERIFY_COUNTERS => "verify_counters", Storage,
    /// The fresh-field ratchet-law pass.
    VERIFY_FRESH => "verify_fresh", Storage,
    /// The dictionary liveness pass.
    VERIFY_DICT => "verify_dict", Storage,
    /// One harness-timed sample.
    SAMPLE => "sample", Harness,
    /// One cold-protocol touch commit.
    TOUCH => "touch", Harness,
    /// One sink-map rehash.
    WORDMAP_GROW => "wordmap_grow", Execute,
    /// One residency-gated phase-1.5 prefetch pass.
    PREFETCH_PASS => "prefetch_pass", Execute,
    /// One predicate-scan kernel invocation.
    KERNEL_FILTER => "kernel_filter", Execute,
    /// One Allen configuration-kernel dense scan.
    KERNEL_ALLEN => "kernel_allen", Execute,
}

const RULE_LABELS: [&str; 16] = [
    "rule_0", "rule_1", "rule_2", "rule_3", "rule_4", "rule_5", "rule_6", "rule_7", "rule_8",
    "rule_9", "rule_10", "rule_11", "rule_12", "rule_13", "rule_14", "rule_15",
];

const JOIN_PHASE_LABELS: [[&str; 9]; 7] = [
    [
        "jp_iter_n0",
        "jp_iter_n1",
        "jp_iter_n2",
        "jp_iter_n3",
        "jp_iter_n4",
        "jp_iter_n5",
        "jp_iter_n6",
        "jp_iter_n7",
        "jp_iter_nX",
    ],
    [
        "jp_hash_n0",
        "jp_hash_n1",
        "jp_hash_n2",
        "jp_hash_n3",
        "jp_hash_n4",
        "jp_hash_n5",
        "jp_hash_n6",
        "jp_hash_n7",
        "jp_hash_nX",
    ],
    [
        "jp_probe_n0",
        "jp_probe_n1",
        "jp_probe_n2",
        "jp_probe_n3",
        "jp_probe_n4",
        "jp_probe_n5",
        "jp_probe_n6",
        "jp_probe_n7",
        "jp_probe_nX",
    ],
    [
        "jp_residual_n0",
        "jp_residual_n1",
        "jp_residual_n2",
        "jp_residual_n3",
        "jp_residual_n4",
        "jp_residual_n5",
        "jp_residual_n6",
        "jp_residual_n7",
        "jp_residual_nX",
    ],
    [
        "jp_descend_n0",
        "jp_descend_n1",
        "jp_descend_n2",
        "jp_descend_n3",
        "jp_descend_n4",
        "jp_descend_n5",
        "jp_descend_n6",
        "jp_descend_n7",
        "jp_descend_nX",
    ],
    [
        "jp_force_n0",
        "jp_force_n1",
        "jp_force_n2",
        "jp_force_n3",
        "jp_force_n4",
        "jp_force_n5",
        "jp_force_n6",
        "jp_force_n7",
        "jp_force_nX",
    ],
    [
        "jp_gather_n0",
        "jp_gather_n1",
        "jp_gather_n2",
        "jp_gather_n3",
        "jp_gather_n4",
        "jp_gather_n5",
        "jp_gather_n6",
        "jp_gather_n7",
        "jp_gather_nX",
    ],
];

const _: () = assert!(crate::ir::MAX_RULES == names::RULE.len());

#[cfg(feature = "trace")]
const _: () = {
    assert!(names::JOIN_PHASE.len() == crate::exec::run::JoinPhase::COUNT);
    assert!(names::JOIN_PHASE[0].len() == crate::exec::run::PHASE_NODE_CAP + 1);
};
