use std::collections::BTreeMap;

use crate::colt::ColtSource;
use crate::query::free_join::FjSubatom;
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QueryTrace, TraceCounters, TraceSpanId};
use crate::tuple::GhtSource;
use crate::{Error, Result};

#[derive(Clone)]
pub(super) struct SourceUndo {
    atom: AtomOccurrenceId,
    previous: ColtSource,
}

pub(super) fn replace_source(
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
    atom: AtomOccurrenceId,
    next: ColtSource,
    undo: &mut Vec<SourceUndo>,
) -> Result<()> {
    let previous = sources
        .insert(atom, next)
        .ok_or_else(|| Error::corrupt(format!("missing source for atom {atom:?}")))?;
    undo.push(SourceUndo { atom, previous });
    Ok(())
}

pub(super) fn restore_sources(
    sources: &mut BTreeMap<AtomOccurrenceId, ColtSource>,
    undo: &mut Vec<SourceUndo>,
    mark: usize,
) {
    while undo.len() > mark {
        let Some(entry) = undo.pop() else { break };
        sources.insert(entry.atom, entry.previous);
    }
}

pub(super) fn source_for(
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

pub(super) fn finish_binding_span(
    trace: &mut QueryTrace,
    span: Option<TraceSpanId>,
    binding_writes: u64,
    binding_conflicts: u64,
    source_replacements: u64,
) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                binding_writes,
                binding_conflicts,
                source_replacements,
                source_frame_changes: source_replacements,
                ..TraceCounters::default()
            },
        );
    }
}

pub(super) fn finish_probe_span(trace: &mut QueryTrace, span: Option<TraceSpanId>, missed: bool) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                probe_calls: 1,
                probe_misses: u64::from(missed),
                source_replacements: u64::from(!missed),
                source_frame_changes: u64::from(!missed),
                ..TraceCounters::default()
            },
        );
    }
}
