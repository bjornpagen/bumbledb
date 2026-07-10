use super::CountingCounters;
use crate::exec::run::Counters;

impl Counters for CountingCounters {
    fn node_entry(&mut self, node: usize) {
        self.node_entries[node] += 1;
    }
    fn batch(&mut self, node: usize, len: usize) {
        self.batches[node][0] += 1;
        self.batches[node][1] += u64::try_from(len).expect("batch fits u64");
    }
    fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool) {
        self.cover_choices[node * self.stride + subatom][usize::from(!exact)] += 1;
    }
    fn probe_hash(&mut self, node: usize, subatom: usize) {
        self.hashes[node * self.stride + subatom] += 1;
    }
    fn probe(&mut self, node: usize, subatom: usize, hit: bool) {
        self.probes[node * self.stride + subatom][usize::from(!hit)] += 1;
    }
    fn residual(&mut self, node: usize, pass: bool) {
        self.residuals[node][usize::from(!pass)] += 1;
    }
    fn anti_probe(&mut self, node: usize, hit: bool) {
        self.anti_probes[node][usize::from(hit)] += 1;
    }
    fn emit(&mut self) {
        self.emits += 1;
    }
    fn emits(&self) -> u64 {
        self.emits
    }
    fn skip(&mut self, node: usize) {
        self.skips[node] += 1;
    }
}
