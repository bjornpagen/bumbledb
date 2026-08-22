//! node probes its negated occurrence per surviving binding; a hit
//! rejects the binding — the inverted polarity of a positive probe miss,
//! compacted through the same survivor cursor-write. Existence, not
//! continuation: the negated occurrence's trie schema is one probe level
//! holding all its variables, so a single `get`-style confirmation
//! decides the probe — an anti-probe never iterates a leaf. The one
//! The anti-probe pass: after residual compaction, each anti-probe attached to the

use super::{
    AntiProbeForm, AntiProbeSpec, Colt, Counters, JoinPhase, PREFETCH_WIDTH_FLOOR, Source,
    grow_scratch, word_base,
};

#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)] 

#[expect(
    clippy::too_many_lines,
    reason = "the linear table or protocol is clearer kept together"
)] 

pub(super) fn anti_probe_pass<C: Counters>(
    specs: &[AntiProbeSpec],
    node_idx: usize,
    cover_vars: &[crate::ir::VarId],
    var_widths: &[(crate::ir::VarId, usize)],
    arity: usize,
    colts: &mut [Colt],
    entry_keys: &[u64],
    survivors: &mut Vec<u32>,
    probe_keys: &mut [u64],
    hashes: &mut Vec<u64>,
    mask: &mut Vec<u8>,
    anti_sources: &mut [Vec<Source>],
    point_checks: &mut Vec<(usize, usize, u64)>,
    point_sources: &mut Vec<(usize, usize, Source)>,
    read_slot: impl Fn(usize, usize) -> u64,
    counters: &mut C,
) {
    let width_of = |var: crate::ir::VarId| -> usize {
        var_widths
            .iter()
            .find(|(v, _)| *v == var)
            .expect("plans bind every variable")
            .1
    };
    for (a_idx, spec) in specs.iter().enumerate() {
        if survivors.is_empty() {
            return;
        }
        let n = survivors.len();

        point_sources.clear();
        for (start_col, end_col, var, slot) in &spec.point_parts {
            let src =
                word_base(cover_vars, *var, width_of).map_or(Source::Slot(*slot), Source::Batch);
            point_sources.push((*start_col, *end_col, src));
        }

        match &spec.form {
            AntiProbeForm::Gate if spec.point_parts.is_empty() => {
                let start = colts[spec.occ].start();
                let hit = colts[spec.occ].key_count(start).magnitude() > 0;
                for _ in 0..n {
                    counters.anti_probe(node_idx, hit);
                }
                if hit {
                    survivors.clear();
                }
            }
            AntiProbeForm::Gate => {

                let start = colts[spec.occ].start();
                grow_scratch(mask, n);
                for k in 0..n {
                    let element = usize::try_from(survivors[k]).expect("batch fits usize");
                    point_checks.clear();
                    for &(start_col, end_col, src) in point_sources.iter() {
                        let point = match src {
                            Source::Batch(base) => entry_keys[element * arity + base],
                            Source::Slot(slot) => read_slot(element, slot),
                        };
                        point_checks.push((start_col, end_col, point));
                    }
                    let hit = colts[spec.occ].any_position_matches(start, point_checks);
                    counters.anti_probe(node_idx, hit);
                    mask[k] = u8::from(!hit);
                }
                crate::exec::kernel::compact_u32_by_mask(survivors, mask);
            }
            AntiProbeForm::Keyed { parts, key_words } => {

                let sources = &mut anti_sources[a_idx];
                sources.clear();
                for (var, slot, width) in parts {
                    match word_base(cover_vars, *var, width_of) {
                        Some(base) => {
                            for offset in 0..*width {
                                sources.push(Source::Batch(base + offset));
                            }
                        }
                        None => {
                            for offset in 0..*width {
                                sources.push(Source::Slot(slot + offset));
                            }
                        }
                    }
                }
                debug_assert_eq!(sources.len(), key_words.get(), "key widths add up");

                counters.phase_start(node_idx, JoinPhase::Force);
                let start = colts[spec.occ].start();
                colts[spec.occ].ensure_forced(start, 0);
                counters.phase_end(node_idx, JoinPhase::Force);

                counters.phase_start(node_idx, JoinPhase::Hash);
                let kw = key_words.get();
                grow_scratch(hashes, n);
                {
                    let probe_keys = &mut probe_keys[..n * kw];
                    let hashes = &mut hashes[..n];
                    for (k, &e) in survivors.iter().enumerate() {
                        let element = usize::try_from(e).expect("batch fits usize");
                        for (word, source) in sources.iter().enumerate() {
                            probe_keys[k * kw + word] = match *source {
                                Source::Batch(col) => entry_keys[element * arity + col],
                                Source::Slot(slot) => read_slot(element, slot),
                            };
                        }
                        hashes[k] = crate::exec::colt::hash_key(&probe_keys[k * kw..(k + 1) * kw]);
                    }
                }
                counters.phase_end(node_idx, JoinPhase::Hash);

                if n >= PREFETCH_WIDTH_FLOOR {
                    crate::obs::event(
                        crate::obs::names::PREFETCH_PASS,
                        crate::obs::TraceArgs::Pair(
                            n as u64,
                            colts[spec.occ].probe_footprint_bytes() as u64,
                        ),
                    );
                    for &hash in &hashes[..n] {
                        colts[spec.occ].prefetch_bucket(start, hash);
                    }
                }

                counters.phase_start(node_idx, JoinPhase::Probe);
                grow_scratch(mask, n);
                {
                    let probe_keys = &probe_keys[..n * kw];
                    let hashes = &hashes[..n];
                    let mask = &mut mask[..n];
                    for k in 0..n {
                        let element = usize::try_from(survivors[k]).expect("batch fits usize");
                        let child = colts[spec.occ].get_prehashed(
                            start,
                            0,
                            &probe_keys[k * kw..(k + 1) * kw],
                            hashes[k],
                        );
                        let hit = match child {
                            None => false,
                            Some(_) if spec.point_parts.is_empty() => true,
                            Some(child) => {
                                point_checks.clear();
                                for &(start_col, end_col, src) in point_sources.iter() {
                                    let point = match src {
                                        Source::Batch(base) => entry_keys[element * arity + base],
                                        Source::Slot(slot) => read_slot(element, slot),
                                    };
                                    point_checks.push((start_col, end_col, point));
                                }
                                colts[spec.occ].any_position_matches(child, point_checks)
                            }
                        };
                        counters.anti_probe(node_idx, hit);
                        mask[k] = u8::from(!hit);
                    }
                }
                crate::exec::kernel::compact_u32_by_mask(survivors, mask);
                counters.phase_end(node_idx, JoinPhase::Probe);
            }
        }

    }
}
