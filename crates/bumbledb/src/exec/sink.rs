//! The two consumers of bindings: set-projection with dedup and
//! the D2 subtree-skip signal, and aggregate folds with binding dedup
//! .
//! **The sinks are where union lives**: one sink hears every rule of a query, its seen-set
//! spanning rules — reset once per execution, never per rule — so a later
//! rule re-deriving a head fact is absorbed exactly like a within-rule
//! `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
use crate::encoding::encode_i64;
use crate::exec::wordmap::WordMap;

mod aggregate;
mod projection;
#[cfg(test)]
mod tests;

/// A fold aggregate's operator, execution-side: exactly the ops that fold
/// over a slot into an [`Acc`]. Nullary [`AggSpec::Count`] is a sibling
/// arm, not a `FoldOp`.
pub use crate::ir::FoldOp;

/// Nullary Count vs a fold over a slot. Trusted layer: Count cannot
/// carry a slot and folds cannot omit one. Hostile Count-with-variable
/// is unrepresentable on [`crate::ir::FindTerm`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggSpec {
    Count,
    Fold {
        op: FoldOp,
        slot: usize,
        width: usize,

        /// biased form; Sum must decode before accumulating).
        signed: bool,
    },
}

impl AggSpec {
    pub(in crate::exec::sink) fn seed_acc(self) -> Acc {
        match self {
            Self::Count => Acc::Count(0),
            Self::Fold {
                op: FoldOp::Sum,
                signed: true,
                ..
            } => Acc::SumSigned(0),
            Self::Fold {
                op: FoldOp::Sum,
                signed: false,
                ..
            } => Acc::SumUnsigned(0),
            Self::Fold {
                op: FoldOp::Min, ..
            } => Acc::Min(u64::MAX),
            Self::Fold {
                op: FoldOp::Max, ..
            } => Acc::Max(u64::MIN),
        }
    }
}

/// One find term in execution form: a projected slot span or a fold
/// aggregate. Widths come from the plan's
/// binding-slot layout (`ValidatedPlan::slots`) — never assumed 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindSpec {
    Var { slot: usize, width: usize },

    Agg(AggSpec),

    Pack { slot: usize },
}

/// What a sink executes after construction parsed [`FindSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinkSpec {
    Var { slot: usize, width: usize },

    Agg(AggSpec),

    Pack { slot: usize },
}

#[derive(Debug)]
pub(in crate::exec::sink) enum DedupState {
    Bindings {
        seen: WordMap<()>,
    },

    Union {
        seen: WordMap<()>,
        spans: Vec<(usize, usize)>,
    },

    DnfUnion {
        seen: WordMap<()>,
        spans: Vec<(usize, usize)>,
    },
    Elided {
        #[allow(dead_code)]
        witness: crate::plan::fj::DistinctWitness,
    },
}

impl DedupState {
    pub(in crate::exec::sink) fn consider(
        &mut self,
        binding_scratch: &[u64],
        union_scratch: &mut Vec<u64>,
    ) -> bool {
        match self {
            Self::Elided { .. } => true,
            Self::Bindings { seen } => seen.insert(binding_scratch),
            Self::Union { seen, spans } | Self::DnfUnion { seen, spans } => {
                union_scratch.clear();
                for &(slot, width) in spans.iter() {
                    union_scratch.extend_from_slice(&binding_scratch[slot..slot + width]);
                }
                seen.insert(union_scratch)
            }
        }
    }

    pub(in crate::exec::sink) fn seen(&self) -> Option<&WordMap<()>> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }

    pub(in crate::exec::sink) fn seen_mut(&mut self) -> Option<&mut WordMap<()>> {
        match self {
            Self::Bindings { seen } | Self::Union { seen, .. } | Self::DnfUnion { seen, .. } => {
                Some(seen)
            }
            Self::Elided { .. } => None,
        }
    }
}

fn word_to_i64(word: u64) -> i64 {
    (word ^ (1 << 63)).cast_signed()
}

fn i64_to_word(value: i64) -> u64 {
    u64::from_be_bytes(encode_i64(value))
}

/// One projected word's source: a binding slot read verbatim. The
/// is gone); the variant stays so the sink match stays total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjSource {
    Slot(usize),
    Measure { start: usize },
}

#[derive(Debug)]
enum ProjectionSources {
    Plain(Vec<usize>),
}

fn sources_of(finds: &[SinkSpec], measures: &[(usize, usize)]) -> ProjectionSources {
    let mut sources = Vec::new();
    extend_sources(finds, measures, &mut sources);
    ProjectionSources::Plain(
        sources
            .into_iter()
            .filter_map(|source| match source {
                ProjSource::Slot(slot) => Some(slot),
                ProjSource::Measure { .. } => None,
            })
            .collect(),
    )
}

fn extend_sources(finds: &[SinkSpec], measures: &[(usize, usize)], out: &mut Vec<ProjSource>) {
    out.clear();
    for spec in finds {
        match spec {
            SinkSpec::Var { slot, width } => {
                if let Some((_, start)) = measures.iter().find(|(derived, _)| derived == slot) {
                    out.push(ProjSource::Measure { start: *start });
                } else {
                    out.extend((*slot..slot + width).map(ProjSource::Slot));
                }
            }
            SinkSpec::Agg(_) | SinkSpec::Pack { .. } => {}
        }
    }
}

/// The projection sink: dedups projected find tuples, and reports
/// staleness (`SkipSuffix`) so the executor can unwind suffixes that bind
/// nothing projection-relevant (D2 — legal for this sink only).
#[derive(Debug)]
pub struct ProjectionSink {
    finds: Vec<SinkSpec>,

    sources: ProjectionSources,
    seen: WordMap<()>,
    scratch: Vec<u64>,

    batch_sources: Vec<crate::exec::run::LeafSource>,

    scan_rows: Vec<u64>,

    scan_count: u64,
}

#[derive(Debug)]
enum GroupTable {
    Hashed(WordMap<usize>),

    Dense {
        radixes: Box<[u16]>,

        table: Box<[u32]>,

        ordinals: Vec<u32>,
    },
}

pub(crate) const DENSE_GROUPS_CAP: u32 = 4096;

impl GroupTable {
    fn len(&self) -> usize {
        match self {
            Self::Hashed(map) => map.len(),
            Self::Dense { ordinals, .. } => ordinals.len(),
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Hashed(map) => map.clear(),
            Self::Dense {
                table, ordinals, ..
            } => {
                table.fill(0);
                ordinals.clear();
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Acc {
    SumSigned(i128),
    SumUnsigned(u128),

    Min(u64),
    Max(u64),
    Count(u64),
}

#[derive(Debug, Clone, Copy)]
enum FoldSource {
    Outer,
    Column(usize),
}

#[derive(Debug)]
pub(in crate::exec::sink) enum GroupState {
    Folds {
        accs: Vec<Acc>,
        n_aggs: usize,
    },
    Pack {
        slot: usize,
        claims: Vec<Vec<[u64; 2]>>,
    },
}

/// The aggregate sink: group map keyed by the group-key words, folding each
/// distinct full binding exactly once. Never returns `SkipSuffix` — the
/// skip is illegal under aggregation (any new bound variable multiplies
/// the binding set the fold is defined over). The illegality is also
/// encoded structurally: aggregate plans mark every node sink-relevant
/// (run.rs's skip-absorption arm), so even a skip
/// signaled by mistake would be absorbed at its producing node.
#[derive(Debug)]
pub struct AggregateSink {
    dedup: DedupState,

    finds: Vec<SinkSpec>,

    measures: Vec<(usize, usize)>,

    real_slots: usize,

    group_spans: Vec<(usize, usize)>,

    groups: GroupTable,

    group_state: GroupState,

    union_scratch: Vec<u64>,
    key_scratch: Vec<u64>,
    binding_scratch: Vec<u64>,

    acc_scratch: Vec<Acc>,

    dedup_survivors: Vec<u32>,

    scan_sources: Vec<FoldSource>,

    scan_count: u64,

    cached_outer_slots: Vec<usize>,
    cached_constant_group: bool,

    #[cfg(test)]
    group_probes: usize,
}
