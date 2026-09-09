use super::{AggregateSink, DedupState, GroupState, GroupTable, ProjectionSink, SpillSet};
fn words<T>(value: &Vec<T>) -> (usize, usize, usize) {
    (value.len(), value.capacity(), std::mem::size_of::<T>())
}
impl SpillSet {
    fn q1_control_report(&self, label: &str) {
        self.ram.q1_control_report(label, "seen");
        let dense = self.unique_rows.as_ref().map(|r| (r.len, words(&r.words)));
        println!(
            "SET {label} dense={dense:?} key_bytes={:?} spilled={}",
            words(&self.key_bytes),
            self.spilled.is_some()
        );
    }
}
impl ProjectionSink {
    pub(crate) fn q1_control_report(&self, label: &str) {
        println!(
            "PROJECTION {label} scratch={:?} scan={:?}",
            words(&self.scratch),
            words(&self.scan_rows)
        );
        self.seen.q1_control_report(label);
    }
}
impl AggregateSink {
    pub(crate) fn q1_control_report(&self, label: &str) {
        let regime = match self.dedup {
            DedupState::Bindings { .. } => "bindings",
            DedupState::Union { .. } => "union",
            DedupState::DnfUnion { .. } => "dnf",
            DedupState::Elided { .. } => "elided",
        };
        println!(
            "AGGREGATE {label} regime={regime} groups={} spilled={}",
            self.groups.len(),
            self.spill.is_some()
        );
        if let Some(seen) = self.dedup.seen() {
            seen.q1_control_report(label);
        }
        match &self.groups {
            GroupTable::Hashed(map) => map.q1_control_report(label, "groups"),
            GroupTable::Dense {
                radixes,
                table,
                ordinals,
            } => println!(
                "DENSE {label} radixes={radixes:?} table_len={} table_bytes={} ordinals={:?}",
                table.len(),
                4 * table.len(),
                words(ordinals)
            ),
        }
        match &self.group_state {
            GroupState::Folds { accs, n_aggs } => {
                println!("FOLDS {label} n_aggs={n_aggs} accs={:?}", words(accs))
            }
            GroupState::Pack { .. } => panic!("not a Pack control"),
        };
        println!(
            "AGGOWNERS {label} counts={:?} float={:?} union={:?} key={:?} binding={:?} survivors={:?} sources={:?} inputs={:?} key_slots={:?} outer_slots={:?}",
            words(&self.group_counts),
            words(&self.float_accs),
            words(&self.union_scratch),
            words(&self.key_scratch),
            words(&self.binding_scratch),
            words(&self.dedup_survivors),
            words(&self.fold_sources),
            words(&self.fold_inputs),
            words(&self.cached_key_slots),
            words(&self.cached_outer_slots)
        );
    }
}
