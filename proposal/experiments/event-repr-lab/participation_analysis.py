"""Check participation measurements against independent row counts and raw logs."""
from pathlib import Path
import argparse, hashlib, json, math, statistics
from prepare import LAB

RESULTS = LAB / 'results'
SCHEDULES = ['complete', 'staged', 'index-right', 'index-small']

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def analyze(path):
    data = json.loads(path.read_text())
    assert data['passed'] and data['phase'] in ['smoke', 'comparison', 'repeat']
    assert data['runner_sha256'] == digest(LAB / 'participation_sweep.py')
    assert data['execute_sha256'] == digest(LAB / 'pack_run.py')
    comparisons = []
    for run in data['runs']:
        assert run['status'] == 'passed'
        raw = [json.loads(line.split('EVENT_LAB ', 1)[1])
            for line in (RESULTS / run['log']).read_text().splitlines() if 'EVENT_LAB ' in line]
        assert len(raw) == len(run['rows']) == 16
        job = run['job']
        order = job['EVENT_PACK_ORDER']
        sequence = SCHEDULES[order:] + SCHEDULES[:order]
        assert [r['schedule'] for r in raw] == [s for s in sequence for _ in range(2)] * 2
        f, shape = job['EVENT_PACK_FANOUT'], job['EVENT_PACK_SHAPE']
        if shape == 'balanced':
            counts, key_counts, union_keys, bindings = [16*f, 16*f], [16, 16], 16, 16*f*f
        elif shape == 'skew':
            counts, key_counts, union_keys, bindings = [16*f+12, 48*f+4], [16, 16], 16, 64*f
        else:
            assert shape == 'unmatched'
            counts, key_counts, union_keys, bindings = [32*f, 80*f], [32, 32], 48, 16*f*f
        groups = 48 if shape == 'unmatched' else 16
        keyed = {}
        for r, enriched in zip(raw, run['rows']):
            assert all(enriched[k] == v for k, v in r.items())
            assert r['kind'] == 'participation_free_join' and r['verified']
            for field, env in [('candidate','EVENT_LAB_CANDIDATE'), ('layout','EVENT_LAB_LAYOUT'),
                    ('width','EVENT_PACK_WIDTH'), ('fanout','EVENT_PACK_FANOUT'),
                    ('shape','EVENT_PACK_SHAPE'), ('schedule_order','EVENT_PACK_ORDER')]:
                assert r[field] == job[env]
            assert r['input_rows'] == counts and r['groups'] == groups and r['present_groups'] == 16
            assert r['represented_bindings'] == bindings
            s = r['schedule']
            if s == 'complete':
                assert r['native_emissions'] == bindings
                for field in ['branch_emissions','summary_rows','map_entries','map_payload_bytes','staged_capacity_bytes']:
                    assert r[field] == 0
                assert r['roster_branch'] == -1
            else:
                assert r['summary_rows'] == 32
                first = -1 if s == 'staged' else 0 if s == 'index-small' and counts[0] < counts[1] else 1
                assert r['roster_branch'] == first
                assert r['branch_emissions'] == sum(counts) + (counts[first] if first >= 0 else 0)
                assert r['native_emissions'] == r['branch_emissions'] + 16
                assert r['map_entries'] == (union_keys if s == 'staged' else key_counts[first])
                if first >= 0:
                    assert r['staged_capacity_bytes'] == 0
            for timing in ['fresh_s', 'warm_s']:
                assert len(r[timing]) == data['trials']
                assert all(math.isfinite(v) and v > 0 for v in r[timing])
            key = (r['program'], r['memo'], s)
            assert key not in keyed
            keyed[key] = r
        for p in ['compose', 'residual']:
            for memo in [False, True]:
                staged = keyed[p, memo, 'staged']
                rows = [keyed[p, memo, s] for s in SCHEDULES]
                for field in ['checksum','represented_bindings','worlds','admissible_worlds','present_groups']:
                    assert len({r[field] for r in rows}) == 1
                item = {k: staged[k] for k in ['candidate','layout','width','fanout','shape','program','memo','input_rows','represented_bindings','checksum']}
                item['schedules'] = {}
                for r in rows:
                    metrics = {k:r[k] for k in ['native_emissions','branch_emissions','staged_capacity_bytes','map_entries','map_payload_bytes','summary_colt_bytes','retained_input_colt_bytes','input_bytes_est','final_bytes_est','roster_branch']}
                    for mode in ['fresh','warm']:
                        metrics[mode+'_median_ms'] = 1000*statistics.median(r[mode+'_s'])
                        metrics[mode+'_speedup_over_staged'] = statistics.median(staged[mode+'_s'])/statistics.median(r[mode+'_s'])
                    item['schedules'][r['schedule']] = metrics
                comparisons.append(item)
    summary = {}
    for s in ['index-right','index-small']:
        summary[s] = {}
        for mode in ['fresh','warm']:
            gains = [p['schedules'][s][mode+'_speedup_over_staged'] for p in comparisons]
            summary[s][mode] = dict(faster=sum(v>1 for v in gains), slower=sum(v<1 for v in gains),
                min=min(gains), median=statistics.median(gains), max=max(gains))
    return dict(passed=True, input=path.name, processes=len(data['runs']), configurations=16*len(data['runs']),
        checker_sha256=digest(Path(__file__)), trials=data['trials'], comparisons=comparisons, summary=summary,
        boundary='Medians of same-binary schedules. Maps exclude allocator/tree overhead; root capacities are cumulative across groups, not peak live bytes. Neither metric measures total peak allocation.')

if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('input')
    ap.add_argument('--output', required=True)
    args = ap.parse_args()
    result = analyze(RESULTS / args.input)
    output = RESULTS / args.output
    assert not output.exists(), 'Preserve previous analysis.'
    output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k != 'comparisons'}, indent=2))
