#!/usr/bin/env python3
"""Audit every fixed Q2-vs-Q1 timing sample; preserve tails, clock flags and adverse cases."""
import argparse
import json
import math
import shutil
import subprocess
from diagnostics import Phase, ROUND, SOURCE, digest, load
from q2_experiment import fingerprint, prior_identities
from q2_time_experiment import TREE, dependencies, CASES

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('timing', type=int)
parser.add_argument('attempt', type=int)
args = parser.parse_args()
path = ROUND/f'q2-timing-{args.timing}'
state = load(path/'STATE.json')
assert state['status'] == 'ORDINARY-Q2-TIMING-COMPLETE-REVIEW-REQUIRED'
assert state['processes_completed'] == 72 and state['distributions'] == 432
assert len(state['steps']) == 72 and all(s['status'] == 'PASS' for s in state['steps'])
assert fingerprint() == state['source_fingerprint']
assert prior_identities() == state['prior_identities']
assert dependencies() == state['dependencies']
assert not subprocess.check_output(['git', '-C', TREE, 'status', '--porcelain'])
assert subprocess.check_output(['git', '-C', TREE, 'rev-parse', 'HEAD']).decode().strip() == SOURCE
assert all(digest(path/name) == sha for name, sha in state['input_hashes'].items())
assert all(digest(e['path']) == e['sha256'] for e in state['executables'].values())

cases = CASES
order = ['A0', 'B0', 'B1', 'A1']
kinds = ['prepare', 'first', 'combined', 'second', 'same_target', 'alternating']
metrics = ['min', 'p50', 'p90', 'p95', 'p99', 'max', 'mean_ns']
assert state['cases'] == [list(c) for c in cases] and state['order'] == order
phase = Phase(f'q2-timing-review-{args.attempt}')
shutil.copy2(__file__, phase.path/'review.py')
phase.state.update(diagnostic_only=False, timing=str(path),
                   source_fingerprint=state['source_fingerprint'],
                   prior_identities=state['prior_identities'],
                   inputs=state['input_hashes'], dependencies=dependencies())
phase.save()
rows, metadata, stamps, logs = {}, {}, {}, {}
distribution_lines = ['case\tkind\tlabel\tn\tdistinct\tsum_ns\t' + '\t'.join(metrics) + '\tclock_flag\tmax_indices']
clock_lines = ['case\tlabel\tbracket\tpre\tpost\tthreshold\tflagged\tretried']
comparison_lines = ['case\trows\tkind\tmetric\tA0\tB0\tB1\tA1\tB_over_A_geomean\tB0_over_A0\tB1_over_A1\tA1_over_A0\tB1_over_B0']
try:
    for family, draw in cases:
        case = f'{family}-{draw}'
        for label in order:
            name = f'{case}-{label}'
            raw_text = (path/f'{name}.log').read_text()
            logs[name] = digest(path/f'{name}.log')
            assert 'scheduler boost: user-interactive QoS verified (not hard P-core affinity)' in raw_text
            records = [json.loads(line) for line in raw_text.splitlines() if line.startswith('{')]
            assert len(records) == 7
            meta, *blocks = records
            metadata.setdefault(case, meta)
            assert metadata[case] == meta
            assert meta['family'] == family and meta['draw'] == draw
            assert meta['alternate'] == (draw+1) % meta['sql_draws']
            assert meta['warmups'] == 8 and meta['warm_samples'] == 320
            assert [b['kind'] for b in blocks] == kinds
            assert all(b['ghz'] == blocks[0]['ghz'] for b in blocks[:4])
            assert all(c >= p+f for p, f, c in zip(*(b['raw_ns'] for b in blocks[:3])))
            for block in blocks:
                kind, raw, ghz = block['kind'], block['raw_ns'], block['ghz']
                assert block['batch'] == 1 and len(raw) == (16 if kind in kinds[:4] else 320)
                assert all(isinstance(v, int) and v > 0 for v in raw)
                values = sorted(raw)
                computed = dict(min=values[0], max=values[-1], mean_ns=sum(values)//len(values))
                for percentile in [50, 90, 95, 99]:
                    computed[f'p{percentile}'] = values[(percentile*len(values)+99)//100-1]
                assert block['stats'] == computed, (name, kind, computed, block['stats'])
                assert all(math.isfinite(ghz[k]) and ghz[k] > 0 for k in ['pre', 'post', 'threshold'])
                assert ghz['contaminated'] == (min(ghz['pre'], ghz['post']) < ghz['threshold'])
                assert not ghz['retried']
                rows[case, label, kind] = block
                max_indices = [i for i, v in enumerate(raw) if v == values[-1]]
                distribution_lines.append('\t'.join(map(str, [case, kind, label, len(raw), len(set(raw)), sum(raw),
                    *(computed[m] for m in metrics), ghz['contaminated'], max_indices])))
                if kind in ['prepare', 'same_target', 'alternating']:
                    bracket = 'cold' if kind == 'prepare' else kind
                    stamps[f'{name}/{bracket}'] = ghz
                    clock_lines.append('\t'.join(map(str, [case, label, bracket,
                        ghz['pre'], ghz['post'], ghz['threshold'], ghz['contaminated'], ghz['retried']])))
    assert len(rows) == 432 and len(stamps) == 216 and len(logs) == 72
    comparisons, tails, rollover = {}, {}, {}
    for family, draw in cases:
        case = f'{family}-{draw}'
        comparisons[case] = dict(metadata=metadata[case], kinds={})
        for kind in kinds:
            group = {label: rows[case, label, kind] for label in order}
            summaries = {label: b['stats'] for label, b in group.items()}
            ratios = {}
            for metric in metrics:
                a0, b0, b1, a1 = (summaries[l][metric] for l in order)
                ratios[metric] = dict(geomean=math.sqrt((b0/a0)*(b1/a1)),
                    forward=b0/a0, backward=b1/a1, baseline_drift=a1/a0, candidate_drift=b1/b0)
                comparison_lines.append('\t'.join(map(str, [case, metadata[case]['answers'], kind, metric,
                    a0, b0, b1, a1, *(ratios[metric][k] for k in ['geomean', 'forward', 'backward', 'baseline_drift', 'candidate_drift'])])))
            comparisons[case]['kinds'][kind] = dict(stats=summaries, ratios=ratios,
                clocks={label: b['ghz'] for label, b in group.items()})
            if kind in kinds[4:]:
                for label, block in group.items():
                    raw = block['raw_ns']
                    # Never remove these samples; index summaries only aid inspection.
                    top = sorted(enumerate(raw), key=lambda iv: (-iv[1], iv[0]))[:8]
                    tails[f'{case}/{label}/{kind}'] = dict(top_index_ns=top,
                        maximum_over_p50=max(raw)/block['stats']['p50'],
                        adjacent_windows=[dict(index=i, values=raw[max(0, i-2):min(len(raw), i+3)],
                                               first_index=max(0, i-2)) for i, _ in top[:3]])
                    if family == 'saved_triangle' and draw != 3 and kind == 'same_target':
                        index = state['rollover_probe']['same_target_index']
                        rollover[f'{case}/{label}'] = dict(index=index, nanoseconds=raw[index],
                            over_p50=raw[index]/block['stats']['p50'],
                            rank_descending=1+sum(v > raw[index] for v in raw),
                            window_first=index-3, window=raw[index-3:index+4])
        print('CASE', case, 'rows', metadata[case]['answers'])
        for kind in kinds:
            detail = comparisons[case]['kinds'][kind]
            print('  ', kind, 'p50', {l: s['p50'] for l, s in detail['stats'].items()},
                  'B/A p50/mean/p99/max', {m: round(detail['ratios'][m]['geomean'], 6) for m in ['p50', 'mean_ns', 'p99', 'max']},
                  'flags', {l: g['contaminated'] for l, g in detail['clocks'].items()})
    (phase.path/'distributions.tsv').write_text('\n'.join(distribution_lines)+'\n')
    (phase.path/'clocks.tsv').write_text('\n'.join(clock_lines)+'\n')
    (phase.path/'comparisons.tsv').write_text('\n'.join(comparison_lines)+'\n')
    (phase.path/'warm-tails.json').write_text(json.dumps(tails, indent=2)+'\n')
    flagged = {name: g for name, g in stamps.items() if g['contaminated']}
    phase.state.update(log_hashes=logs, distributions=len(rows), clock_brackets=len(stamps),
        flagged_clocks=flagged, clock_minimum=min(min(g['pre'], g['post']) for g in stamps.values()),
        clock_maximum=max(max(g['pre'], g['post']) for g in stamps.values()),
        total_raw_values=sum(len(b['raw_ns']) for b in rows.values()),
        comparisons=comparisons, rollover=rollover,
        output_hashes={name:digest(phase.path/name) for name in
            ['distributions.tsv', 'clocks.tsv', 'comparisons.tsv', 'warm-tails.json']},
        limitations='Descriptive per-case ABBA only, all raw values retained without retries or filtering. Shared macOS QoS is not hard P-core affinity. Endpoint clock flags cannot rule out mid-window interference. Reopened DB is not an OS-cold-cache claim. Index248 is a source-model prediction, not direct rollover attribution. No RSS/Pi qualification, full trace, full suite or automatic candidate acceptance.')
    assert fingerprint() == state['source_fingerprint'] and prior_identities() == state['prior_identities']
    assert dependencies() == state['dependencies']
    phase.save()
    print('CLOCKS', len(stamps), 'flagged', len(flagged), 'raw values', phase.state['total_raw_values'])
    print('PREDECLARED-ROLLOVER', json.dumps(rollover, sort_keys=True))
except BaseException:
    phase.finish('INCOMPLETE'); raise
else:
    phase.finish('ALL-ORDINARY-Q2-DISTRIBUTIONS-VERIFIED-INTERPRETATION-REQUIRED')
