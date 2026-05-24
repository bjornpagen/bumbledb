use std::collections::BTreeMap;

use crate::colt::ColtSource;
use crate::query::free_join::{FjSubatom, ValidatedFjNode};
use crate::query::model::AtomOccurrenceId;
use crate::tuple::{GhtSource, KeyCountEstimate};
use crate::{Error, Result};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CoverPolicy {
    StaticFirst,
    DynamicMinKeys,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ExecutionStats {
    pub(crate) cover_choices: Vec<CoverChoiceEvent>,
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
    node: &ValidatedFjNode,
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    policy: CoverPolicy,
    stats: &mut ExecutionStats,
) -> Result<usize> {
    if node.covers.is_empty() {
        return Err(Error::invalid_query(format!(
            "node {} has no cover",
            node.id
        )));
    }

    let mut observations = Vec::with_capacity(node.covers.len());
    for cover in &node.covers {
        let subatom = &node.subatoms[cover.subatom];
        let source = source_for(sources, subatom)?;
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

fn source_for(
    sources: &BTreeMap<AtomOccurrenceId, ColtSource>,
    subatom: &FjSubatom,
) -> Result<ColtSource> {
    let source = sources
        .get(&subatom.atom)
        .cloned()
        .ok_or_else(|| Error::corrupt(format!("missing source for atom {:?}", subatom.atom)))?;
    if source.atom() != Some(subatom.atom) || source.vars() != subatom.vars.as_slice() {
        return Err(Error::corrupt(format!(
            "source schema mismatch for atom {:?}",
            subatom.atom
        )));
    }
    Ok(source)
}

fn key_count_value(count: KeyCountEstimate) -> usize {
    match count {
        KeyCountEstimate::Exact(value) | KeyCountEstimate::Estimate(value) => value,
    }
}
