//! After residual compaction, probe each negated occurrence per surviving
//! binding. A hit rejects the binding. The negated trie holds all its key
//! variables at one level: this checks existence, not a continuation to emit.
use super::{
    AntiProbeForm, AntiProbeSpec, Colt, Counters, PREFETCH_WIDTH_FLOOR, PointSource, Source,
    grow_scratch,
};
use crate::work::WorkError;

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
    arity: usize,
    colts: &mut [Colt],
    entry_keys: &[u64],
    survivors: &mut Vec<u32>,
    probe_keys: &mut [u64],
    hashes: &mut Vec<u64>,
    mask: &mut Vec<u8>,
    anti_sources: &[Vec<Source>],
    point_checks: &mut Vec<(usize, usize, u64)>,
    anti_point_sources: &[Vec<PointSource>],
    read_slot: impl Fn(usize, usize) -> u64,
    counters: &mut C,
) -> Result<(), WorkError> {
    for (a_idx, spec) in specs.iter().enumerate() {
        if survivors.is_empty() {
            return Ok(());
        }
        let n = survivors.len();

        let point_sources = &anti_point_sources[a_idx];

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
                    for &(start_col, end_col, src, dense) in point_sources {
                        let point = match src {
                            Source::Batch(base) => entry_keys[element * arity + base],
                            Source::Slot(slot) => read_slot(element, slot),
                        };
                        // The dense finite-probe guard:
                        // a nonfinite point satisfies no membership, so
                        // the negated atom's conjunction has no witness.
                        let point = if dense {
                            crate::image::view::dense_probe_word(point)
                        } else {
                            point
                        };
                        point_checks.push((start_col, end_col, point));
                    }
                    let hit = colts[spec.occ].any_position_matches(start, point_checks);
                    counters.anti_probe(node_idx, hit);
                    mask[k] = u8::from(!hit);
                }
                crate::exec::kernel::compact_u32_by_mask(survivors, mask);
            }
            AntiProbeForm::Keyed { key_words, .. } => {
                let sources = &anti_sources[a_idx];
                let start = colts[spec.occ].start();
                let probe = colts[spec.occ].prepare_probe(start, 0)?;

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

                if n >= PREFETCH_WIDTH_FLOOR {
                    probe.prefetch_batch(&hashes[..n], !spec.point_parts.is_empty());
                }

                grow_scratch(mask, n);
                {
                    let probe_keys = &probe_keys[..n * kw];
                    let hashes = &hashes[..n];
                    let mask = &mut mask[..n];
                    if spec.point_parts.is_empty() {
                        for k in 0..n {
                            let hit = probe.contains_prehashed_width::<0>(
                                &probe_keys[k * kw..(k + 1) * kw],
                                hashes[k],
                            );
                            counters.anti_probe(node_idx, hit);
                            mask[k] = u8::from(!hit);
                        }
                        crate::exec::kernel::compact_u32_by_mask(survivors, mask);
                        continue;
                    }
                    for k in 0..n {
                        let element = usize::try_from(survivors[k]).expect("batch fits usize");
                        let child = probe
                            .get_prehashed_width::<0>(&probe_keys[k * kw..(k + 1) * kw], hashes[k]);
                        let hit = match child {
                            None => false,
                            Some(child) => {
                                point_checks.clear();
                                for &(start_col, end_col, src, dense) in point_sources {
                                    let point = match src {
                                        Source::Batch(base) => entry_keys[element * arity + base],
                                        Source::Slot(slot) => read_slot(element, slot),
                                    };
                                    // The dense finite-probe guard.
                                    let point = if dense {
                                        crate::image::view::dense_probe_word(point)
                                    } else {
                                        point
                                    };
                                    point_checks.push((start_col, end_col, point));
                                }
                                probe.any_position_matches(child, point_checks)
                            }
                        };
                        counters.anti_probe(node_idx, hit);
                        mask[k] = u8::from(!hit);
                    }
                }
                crate::exec::kernel::compact_u32_by_mask(survivors, mask);
            }
        }
    }
    Ok(())
}
