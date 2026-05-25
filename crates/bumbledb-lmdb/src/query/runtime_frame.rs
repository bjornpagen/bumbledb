use crate::base_image::RelationBaseImageRef;
use crate::colt::{ColtSource, ColtSourceOwner, SourceBuildConfig};
use crate::query::free_join::{ValidatedFjPlan, ValidatedFjSubatom};
use crate::query::model::AtomOccurrenceId;
use crate::query::trace::{QUERY_TRACING_ENABLED, QueryTrace, TraceCounters, TraceSpanId};
use crate::tuple::{GhtSource, TupleSchema};
use crate::{Error, Result};

pub(crate) struct SourceStore {
    owner: ColtSourceOwner,
    current: Vec<Option<ColtSource>>,
}

#[derive(Clone)]
pub(super) struct SourceUndo {
    atom: AtomOccurrenceId,
    previous: ColtSource,
}

impl SourceStore {
    pub(super) fn with_atom_count(atom_count: usize) -> Self {
        Self {
            owner: ColtSourceOwner::with_capacity(atom_count),
            current: vec![None; atom_count],
        }
    }

    pub(super) fn insert_filtered_traced_labeled(
        &mut self,
        atom: AtomOccurrenceId,
        base: RelationBaseImageRef,
        schemas: Vec<TupleSchema>,
        config: SourceBuildConfig,
        trace: &mut QueryTrace,
    ) {
        self.ensure_atom(atom);
        let source = self
            .owner
            .add_filtered_traced_labeled(atom, base, schemas, config, trace);
        self.current[atom.0] = Some(source);
    }

    pub(crate) fn source_for_atom(&self, atom: AtomOccurrenceId) -> Option<ColtSource> {
        self.current.get(atom.0).copied().flatten()
    }

    fn replace(&mut self, atom: AtomOccurrenceId, next: ColtSource) -> Option<ColtSource> {
        self.ensure_atom(atom);
        self.current[atom.0].replace(next)
    }

    fn restore(&mut self, atom: AtomOccurrenceId, previous: ColtSource) {
        self.ensure_atom(atom);
        self.current[atom.0] = Some(previous);
    }

    fn ensure_atom(&mut self, atom: AtomOccurrenceId) {
        if atom.0 >= self.current.len() {
            self.current.resize(atom.0 + 1, None);
        }
    }
}

pub(super) fn replace_source(
    sources: &mut SourceStore,
    atom: AtomOccurrenceId,
    next: ColtSource,
    undo: &mut Vec<SourceUndo>,
) -> Result<()> {
    let previous = sources
        .replace(atom, next)
        .ok_or_else(|| Error::corrupt(format!("missing source for atom {atom:?}")))?;
    undo.push(SourceUndo { atom, previous });
    Ok(())
}

pub(super) fn restore_sources(sources: &mut SourceStore, undo: &mut Vec<SourceUndo>, mark: usize) {
    while undo.len() > mark {
        let Some(entry) = undo.pop() else { break };
        sources.restore(entry.atom, entry.previous);
    }
}

pub(super) fn source_for(
    sources: &SourceStore,
    plan: &ValidatedFjPlan,
    subatom: &ValidatedFjSubatom,
) -> Result<ColtSource> {
    let source = sources
        .source_for_atom(subatom.atom)
        .ok_or_else(|| Error::corrupt(format!("missing source for atom {:?}", subatom.atom)))?;
    if source.atom() != Some(subatom.atom) || source.vars() != plan.subatom_vars(subatom) {
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

pub(super) fn finish_node_span(trace: &mut QueryTrace, span: Option<TraceSpanId>, depth: usize) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                recursive_node_entries: 1,
                max_recursion_depth: depth as u64,
                frame_pushes: 1,
                frame_pops: 1,
                ..TraceCounters::default()
            },
        );
    }
}

pub(super) fn finish_skipped_node_span(
    trace: &mut QueryTrace,
    span: Option<TraceSpanId>,
    depth: usize,
) {
    if let Some(span) = span {
        trace.finish_span(
            span,
            TraceCounters {
                recursive_node_entries: 1,
                max_recursion_depth: depth as u64,
                frame_pushes: 1,
                frame_pops: 1,
                factorized_expansions_avoided: 1,
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

pub(super) fn lazy_label(prefix: &'static str, node: usize, atom: AtomOccurrenceId) -> String {
    if QUERY_TRACING_ENABLED {
        format!("{prefix} node={node} atom={atom:?}")
    } else {
        String::new()
    }
}

pub(super) fn lazy_atom_label(prefix: &'static str, atom: AtomOccurrenceId) -> String {
    if QUERY_TRACING_ENABLED {
        format!("{prefix} atom={atom:?}")
    } else {
        String::new()
    }
}
