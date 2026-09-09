use super::{Dir, FLAT_SWEEP_CEILING, OverlapCache};
use std::cell::RefCell;
use std::io::Write;

#[path = "/Users/bjorn/Documents/bumbledb/bench-out/autoresearch-20260908.pMXtzv/p3-flat-kernels.rs"]
mod kernels;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Sample {
    dir: u32,
    lo: u64,
    hi: u64,
    hits: usize,
    capacity: usize,
}

thread_local! {
    static SAMPLES: RefCell<Option<Vec<Sample>>> = const { RefCell::new(None) };
}

pub(crate) fn begin(capacity: usize) {
    SAMPLES.with_borrow_mut(|slot| {
        assert!(slot.is_none());
        *slot = Some(Vec::with_capacity(capacity));
    });
}

pub(crate) fn finish() -> Vec<Sample> {
    SAMPLES.with_borrow_mut(|slot| slot.take().unwrap())
}

pub(crate) fn identities(samples: &[Sample]) -> Vec<(u32, u64, u64, usize)> {
    samples
        .iter()
        .map(|sample| (sample.dir, sample.lo, sample.hi, sample.hits))
        .collect()
}

pub(crate) fn record(dir: u32, lo: u64, hi: u64, out: &Vec<u32>) {
    SAMPLES.with_borrow_mut(|slot| {
        if let Some(samples) = slot {
            assert!(
                samples.len() < samples.capacity(),
                "no observation allocation in query"
            );
            samples.push(Sample {
                dir,
                lo,
                hi,
                hits: out.len(),
                capacity: out.capacity(),
            });
        }
    });
}

impl OverlapCache {
    pub(crate) fn audit_demand(
        &self,
        label: &str,
        samples: &[Sample],
        initial_capacity: usize,
        output: Option<&std::path::Path>,
    ) {
        let mut baseline = Vec::with_capacity(initial_capacity);
        let mut candidate = Vec::with_capacity(initial_capacity);
        let mut direct = Vec::new();
        let mut lengths = [0u64; 129];
        let mut prefixes = [0u64; 129];
        let mut hits = [0u64; 129];
        let mut flat = 0u64;
        let mut large = 0u64;
        let mut transitions = 0u64;
        let mut early = 0u64;
        let mut no_hits = 0u64;
        let mut all_hits = 0u64;
        let mut cap_short = 0u64;
        let mut baseline_growths = 0u64;
        let mut candidate_growths = 0u64;
        let mut max_extra_cap = 0usize;
        for &sample in samples {
            let old_baseline = baseline.capacity();
            let old_candidate = candidate.capacity();
            self.query_into(sample.dir, sample.lo, sample.hi, &mut baseline);
            assert_eq!(baseline.len(), sample.hits);
            assert_eq!(
                baseline.capacity(),
                sample.capacity,
                "replay the actual output owner"
            );
            let Dir::Built {
                base,
                len,
                tree_base,
                ..
            } = self.dirs[sample.dir as usize]
            else {
                panic!("observed queries only use built groups");
            };
            let len = len as usize;
            if len <= FLAT_SWEEP_CEILING {
                flat += 1;
                let base = base as usize;
                let starts = &self.starts[base..base + len];
                let ends = &self.tree[tree_base as usize..][..len];
                let positions = &self.positions[base..base + len];
                let prefix = starts.partition_point(|&start| start < sample.hi);
                lengths[len] += 1;
                prefixes[prefix] += 1;
                hits[baseline.len()] += 1;
                early += u64::from(prefix < len);
                no_hits += u64::from(baseline.is_empty());
                all_hits += u64::from(baseline.len() == prefix);
                cap_short += u64::from(prefix > sample.capacity);
                transitions += ends[..prefix]
                    .windows(2)
                    .filter(|pair| (pair[0] > sample.lo) != (pair[1] > sample.lo))
                    .count() as u64;
                kernels::branchy(starts, ends, positions, sample.lo, sample.hi, &mut direct);
                assert_eq!(baseline, direct, "matched branchy mechanism");
                kernels::compact(
                    starts,
                    ends,
                    positions,
                    sample.lo,
                    sample.hi,
                    &mut candidate,
                );
            } else {
                large += 1;
                self.query_into(sample.dir, sample.lo, sample.hi, &mut candidate);
            }
            assert_eq!(
                baseline, candidate,
                "exact position order, sample={sample:?}"
            );
            baseline_growths += u64::from(baseline.capacity() != old_baseline);
            candidate_growths += u64::from(candidate.capacity() != old_candidate);
            max_extra_cap =
                max_extra_cap.max(candidate.capacity().saturating_sub(baseline.capacity()));
        }
        println!(
            "DEMAND {label} total={} flat={flat} large={large} early_cutoff={early} no_hits={no_hits} all_eligible_hit={all_hits} predicate_transitions={transitions} prefix_exceeds_actual_capacity={cap_short}",
            samples.len()
        );
        println!(
            "REPLAY {label} baseline_capacity={} candidate_capacity={} baseline_growth_events={baseline_growths} candidate_growth_events={candidate_growths} max_extra_candidate_capacity={max_extra_cap} units=u32_elements",
            baseline.capacity(),
            candidate.capacity()
        );
        for (kind, bins) in [("length", lengths), ("prefix", prefixes), ("hits", hits)] {
            println!("HIST {label} {kind} {bins:?}");
            println!(
                "TOTAL {label} {kind} {}",
                bins.into_iter()
                    .enumerate()
                    .map(|(size, count)| size as u64 * count)
                    .sum::<u64>()
            );
        }
        if let Some(path) = output {
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .unwrap();
            let mut file = std::io::BufWriter::new(file);
            file.write_all(b"BFLAT001").unwrap();
            let flat_dirs: Vec<_> = self
                .dirs
                .iter()
                .enumerate()
                .filter_map(|(dir, &state)| {
                    let Dir::Built {
                        base,
                        len,
                        tree_base,
                        ..
                    } = state
                    else {
                        return None;
                    };
                    (len as usize <= FLAT_SWEEP_CEILING).then_some((
                        dir,
                        base as usize,
                        len as usize,
                        tree_base as usize,
                    ))
                })
                .collect();
            file.write_all(&(flat_dirs.len() as u32).to_le_bytes())
                .unwrap();
            for (dir, base, len, tree_base) in flat_dirs {
                file.write_all(&(dir as u32).to_le_bytes()).unwrap();
                file.write_all(&(len as u32).to_le_bytes()).unwrap();
                for i in 0..len {
                    file.write_all(&self.starts[base + i].to_le_bytes())
                        .unwrap();
                    file.write_all(&self.tree[tree_base + i].to_le_bytes())
                        .unwrap();
                    file.write_all(&self.positions[base + i].to_le_bytes())
                        .unwrap();
                }
            }
            file.write_all(&(flat as u32).to_le_bytes()).unwrap();
            for sample in samples {
                let Dir::Built { len, .. } = self.dirs[sample.dir as usize] else {
                    unreachable!()
                };
                if len as usize > FLAT_SWEEP_CEILING {
                    continue;
                }
                file.write_all(&sample.dir.to_le_bytes()).unwrap();
                file.write_all(&sample.lo.to_le_bytes()).unwrap();
                file.write_all(&sample.hi.to_le_bytes()).unwrap();
                file.write_all(&(sample.hits as u32).to_le_bytes()).unwrap();
            }
            file.flush().unwrap();
        }
    }
}
