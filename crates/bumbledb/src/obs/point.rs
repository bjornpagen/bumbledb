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

    Count(u64),

    Pair(u64, u64),

    Flag(bool),
}

impl TraceArgs {
    #[must_use]
    pub const fn a0(self) -> u64 {
        match self {
            Self::None => 0,
            Self::Count(n) | Self::Pair(n, _) => n,
            Self::Flag(b) => b as u64,
        }
    }

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

            Rule(u8),

            JoinPhase { phase: u8, node: u8 },
        }

        impl TracePoint {

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

        pub mod names {
            use super::TracePoint;
            $(
                $(#[$meta])*
                pub const $variant: TracePoint = TracePoint::$variant;
            )*

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

    PREPARE => "prepare", Prepare,

    VALIDATE => "validate", Prepare,

    VALIDATE_LOWER => "validate_lower", Prepare,

    VALIDATE_SEAL => "validate_seal", Prepare,

    VALIDATE_RULES => "validate_rules", Prepare,

    NORMALIZE => "normalize", Prepare,

    PLACE_COMPARISONS => "place_comparisons", Prepare,

    NORMALIZE_FOLD => "normalize_fold", Prepare,

    CLASSIFY => "classify", Prepare,

    STATS => "stats", Prepare,

    PLAN_DP => "plan_dp", Prepare,

    LOWER => "lower", Prepare,

    BUILD_COLTS => "build_colts", Prepare,

    PLAN_DENSIFY => "plan_densify", Prepare,

    PLAN_FILL => "plan_fill", Prepare,

    RELATION_ROWS => "relation_rows", Prepare,

    DISTINCT_LADDER => "distinct_ladder", Prepare,

    EXECUTE => "execute", Execute,

    INTERIORS => "interiors", Execute,

    REACH => "reach", Execute,

    FIXPOINT_ROUND => "fixpoint_round", Execute,

    BIND_PARAMS => "bind_params", Execute,

    RESOLVE_FILTERS => "resolve_filters", Execute,

    VIEWS => "views", Execute,

    VIEW_BUILD => "view_build", Execute,

    VIEW_MEMO_HIT => "view_memo_hit", Execute,

    VIEW_DEDUP => "view_dedup", Execute,

    JOIN => "join", Execute,

    FINALIZE => "finalize", Execute,

    KEY_PROBE => "key_probe", Execute,

    POINT_READ => "point_read", Storage,

    SELECTIONS => "selections", Execute,

    SELECT_PROBE => "select_probe", Execute,

    CACHE_HIT => "cache_hit", Cache,

    IMAGE_BUILD => "image_build", Image,

    IMAGE_APPEND => "image_append", Image,

    IMAGE_DISTINCTS => "image_distincts", Image,

    DECODE_BATCH => "decode_batch", Image,

    CACHE_CARRY => "cache_carry", Cache,

    CACHE_ADOPT => "cache_adopt", Cache,

    CACHE_QUERY_LOCAL => "cache_query_local", Cache,

    COLT_FORCE => "colt_force", Execute,

    DICT_RESOLVE => "dict_resolve", Execute,

    PARAM_WORD_MEMO => "param_word_memo", Execute,

    LITERAL_LATCH => "literal_latch", Execute,

    COMMIT => "commit", Commit,

    COMMIT_NOOP => "commit_noop", Commit,

    APPLY_DELETES => "apply_deletes", Commit,

    APPLY_INSERTS => "apply_inserts", Commit,

    JUDGMENT_SOURCE => "judgment_source", Commit,

    JUDGMENT_TARGET => "judgment_target", Commit,

    JUDGMENT_CAPACITIES => "judgment_capacities", Commit,

    COUNTERS_FLUSH => "counters_flush", Commit,
    /// Phase 5: the LMDB commit alone.
    LMDB_COMMIT => "lmdb_commit", Commit,

    COMMIT_SYNC_RETRY => "commit_sync_retry", Commit,

    COMPACT_DURABLE => "compact_durable", Commit,

    CREATE_DURABLE => "create_durable", Commit,

    WRITE_TXN => "write_txn", Commit,

    PUBLISH_COPY => "publish_copy", Storage,

    PUBLISH_SYNC => "publish_sync", Storage,

    VERIFY_STORE => "verify_store", Storage,

    VERIFY_FACTS => "verify_facts", Storage,

    VERIFY_MEMBERSHIP => "verify_membership", Storage,

    VERIFY_DETERMINANTS => "verify_determinants", Storage,

    VERIFY_REVERSE => "verify_reverse", Storage,

    VERIFY_MARKS => "verify_marks", Storage,

    VERIFY_COUNTERS => "verify_counters", Storage,

    VERIFY_FRESH => "verify_fresh", Storage,

    VERIFY_DICT => "verify_dict", Storage,

    SAMPLE => "sample", Harness,
    /// One cold-protocol touch commit.
    TOUCH => "touch", Harness,

    WORDMAP_GROW => "wordmap_grow", Execute,

    PREFETCH_PASS => "prefetch_pass", Execute,

    KERNEL_FILTER => "kernel_filter", Execute,

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
