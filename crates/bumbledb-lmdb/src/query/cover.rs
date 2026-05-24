use crate::query::free_join::{ValidatedFjNode, ValidatedFjPlan};
use crate::query::model::AtomOccurrenceId;
use crate::query::runtime_frame::{SourceStore, source_for as frame_source_for};
use crate::tuple::{GhtSource, KeyCountEstimate};
use crate::{Error, Result};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoverPolicy {
    StaticFirst,
    DynamicMinKeys,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionMode {
    Scalar,
    Vectorized { batch_size: usize },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExecutionStats {
    pub(crate) cover_choices: Vec<CoverChoiceEvent>,
    pub(crate) vectorized: VectorizedStats,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct VectorizedStats {
    pub(crate) batch_size: usize,
    pub(crate) batches: usize,
    pub(crate) input_tuples: usize,
    pub(crate) survivor_tuples: usize,
    pub(crate) failed_tuples: usize,
    pub(crate) probe_calls: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CoverChoiceEvent {
    pub(crate) node: usize,
    pub(crate) candidates: Vec<CoverCandidateObservation>,
    pub(crate) chosen_subatom: usize,
    pub(crate) tie_break: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CoverCandidateObservation {
    pub(crate) subatom: usize,
    pub(crate) atom: AtomOccurrenceId,
    pub(crate) key_count: KeyCountEstimate,
}

pub(crate) fn choose_cover(
    plan: &ValidatedFjPlan,
    node: &ValidatedFjNode,
    sources: &SourceStore,
    policy: CoverPolicy,
    stats: &mut ExecutionStats,
) -> Result<usize> {
    let covers = plan.node_covers(node);
    let subatoms = plan.node_subatoms(node);
    if covers.is_empty() {
        return Err(Error::invalid_query(format!(
            "node {} has no cover",
            node.id
        )));
    }

    let mut observations = Vec::with_capacity(covers.len());
    for cover in covers {
        let subatom = &subatoms[cover.subatom];
        let source = frame_source_for(sources, plan, subatom)?;
        observations.push(CoverCandidateObservation {
            subatom: cover.subatom,
            atom: subatom.atom,
            key_count: source.key_count(),
        });
    }

    let chosen_subatom = match policy {
        CoverPolicy::StaticFirst => observations[0].subatom,
        CoverPolicy::DynamicMinKeys => observations
            .iter()
            .min_by_key(|observation| (key_count_value(observation.key_count), observation.subatom))
            .map(|observation| observation.subatom)
            .ok_or_else(|| Error::invalid_query(format!("node {} has no cover", node.id)))?,
    };
    let min_count = observations
        .iter()
        .map(|observation| key_count_value(observation.key_count))
        .min()
        .unwrap_or(0);
    let tie_break = observations
        .iter()
        .filter(|observation| key_count_value(observation.key_count) == min_count)
        .count()
        > 1;
    stats.cover_choices.push(CoverChoiceEvent {
        node: node.id,
        candidates: observations,
        chosen_subatom,
        tie_break,
    });
    Ok(chosen_subatom)
}

fn key_count_value(count: KeyCountEstimate) -> usize {
    match count {
        KeyCountEstimate::Exact(value) | KeyCountEstimate::Estimate(value) => value,
    }
}
