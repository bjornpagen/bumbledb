//! After residual compaction, probe each negated occurrence per surviving
//! binding. A hit rejects the binding. The negated trie holds all its key
//! variables at one level: this checks existence, not a continuation to emit.
use super::filter::resolve_points;
use super::{
    AntiProbeForm, AntiProbeSpec, BatchBuffers, BatchRows, Colt, Counters, PREFETCH_WIDTH_FLOOR,
    SourceLayout, grow_scratch,
};
use crate::work::WorkError;

pub(super) fn anti_probe_pass<R: BatchRows, C: Counters>(
    specs: &[AntiProbeSpec],
    layout: &SourceLayout,
    rows: &R,
    buf: &mut BatchBuffers,
    colts: &mut [Colt],
    node_idx: usize,
    counters: &mut C,
) -> Result<(), WorkError> {
    for (a_idx, spec) in specs.iter().enumerate() {
        if buf.survivors.is_empty() {
            return Ok(());
        }
        let n = buf.survivors.len();
        let point_sources = &layout.anti_point_sources[a_idx];

        match &spec.form {
            AntiProbeForm::Gate if spec.point_parts.is_empty() => {
                let start = colts[spec.occ].start();
                let hit = colts[spec.occ].key_count(start).magnitude() > 0;
                for _ in 0..n {
                    counters.anti_probe(node_idx, hit);
                }
                if hit {
                    buf.survivors.clear();
                }
            }
            AntiProbeForm::Gate => {
                let start = colts[spec.occ].start();
                grow_scratch(&mut buf.mask, n);
                for k in 0..n {
                    let element = buf.survivors[k] as usize;
                    resolve_points(rows, element, point_sources, &mut buf.point_checks);
                    let hit = colts[spec.occ].any_position_matches(start, &buf.point_checks);
                    counters.anti_probe(node_idx, hit);
                    buf.mask[k] = u8::from(!hit);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut buf.survivors, &buf.mask);
            }
            AntiProbeForm::Keyed { key_words, .. } => {
                let sources = &layout.anti_sources[a_idx];
                let start = colts[spec.occ].start();
                let probe = colts[spec.occ].prepare_probe(start, 0)?;

                let kw = key_words.get();
                grow_scratch(&mut buf.hashes, n);
                {
                    let probe_keys = &mut buf.probe_keys[..n * kw];
                    let hashes = &mut buf.hashes[..n];
                    for (k, &element) in buf.survivors.iter().enumerate() {
                        let key = &mut probe_keys[k * kw..(k + 1) * kw];
                        for (word, &source) in key.iter_mut().zip(sources) {
                            *word = rows.word(element as usize, source, 0);
                        }
                        hashes[k] = crate::exec::colt::hash_key(key);
                    }
                }

                if n >= PREFETCH_WIDTH_FLOOR {
                    probe.prefetch_batch(&buf.hashes[..n], !spec.point_parts.is_empty());
                }

                grow_scratch(&mut buf.mask, n);
                {
                    let probe_keys = &buf.probe_keys[..n * kw];
                    let hashes = &buf.hashes[..n];
                    let mask = &mut buf.mask[..n];
                    if spec.point_parts.is_empty() {
                        for k in 0..n {
                            let hit = probe.contains_prehashed_width::<0>(
                                &probe_keys[k * kw..(k + 1) * kw],
                                hashes[k],
                            );
                            counters.anti_probe(node_idx, hit);
                            mask[k] = u8::from(!hit);
                        }
                    } else {
                        for k in 0..n {
                            let element = buf.survivors[k] as usize;
                            let child = probe.get_prehashed_width::<0>(
                                &probe_keys[k * kw..(k + 1) * kw],
                                hashes[k],
                            );
                            let hit = child.is_some_and(|child| {
                                resolve_points(rows, element, point_sources, &mut buf.point_checks);
                                probe.any_position_matches(child, &buf.point_checks)
                            });
                            counters.anti_probe(node_idx, hit);
                            mask[k] = u8::from(!hit);
                        }
                    }
                }
                crate::exec::kernel::compact_u32_by_mask(&mut buf.survivors, &buf.mask);
            }
        }
    }
    Ok(())
}
