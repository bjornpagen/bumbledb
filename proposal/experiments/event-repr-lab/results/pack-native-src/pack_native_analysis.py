"""Compare schedules only after checking identical native queries and outputs."""
from pathlib import Path
import argparse, hashlib, itertools, json, math, statistics
from prepare import LAB

CANDIDATES = ['dense', 'packed512', 'essential512', 'retraction512', 'range-shannon', 'range-group']
SCENARIOS = ['coup_4290', 'coup_product_65536']

def digest(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def analyze(path):
    data = json.loads(path.read_text())
    assert data['passed'] and data['phase'] == 'comparison' and data['trials'] >= 7
    rows = {}
    for run in data['runs']:
        assert run['status'] == 'passed' and len(run['rows']) == 4
        for row in run['rows']:
            assert row['kind'] == 'factorized_pack_free_join' and row['verified']
            for field, env in [('candidate','EVENT_LAB_CANDIDATE'), ('scenario','EVENT_LAB_SCENARIO'), ('fanout','EVENT_PACK_FANOUT')]:
                assert row[field] == run['job'][env]
            for field in ['fresh_s', 'warm_s']:
                assert len(row[field]) == data['trials']
                assert all(math.isfinite(v) and v > 0 for v in row[field])
            key = tuple(row[k] for k in ['candidate','scenario','fanout','memo','schedule'])
            assert key not in rows
            rows[key] = row
    candidates=data.get('candidates',CANDIDATES)
    assert 'dense' in candidates and set(candidates)<=set(CANDIDATES)
    cases = list(itertools.product(candidates, SCENARIOS, data['fanouts'], [False, True]))
    assert set(rows) == {(*case,s) for case in cases for s in ['complete','factorized']}
    pairs = []
    for case in cases:
        complete, factorized = (rows[(*case,s)] for s in ['complete','factorized'])
        ref = rows[('dense',*case[1:],'complete')]
        g, f = complete['groups'], complete['fanout']
        for r in [complete,factorized]:
            for field in ['checksum','worlds','admissible_worlds','groups','present_groups','represented_bindings']:
                assert r[field] == ref[field], (case,field)
            assert r['groups'] == r['present_groups'] == 64
            assert r['represented_bindings'] == g*f**3
        assert complete['native_emissions'] == g*f**3
        assert all(complete[k] == 0 for k in ['branch_emissions','summary_rows','summary_colt_bytes','staged_id_capacity_bytes'])
        assert factorized['branch_emissions'] == 3*g*f
        assert factorized['summary_rows'] == 3*g
        assert factorized['native_emissions'] == 3*g*f+g
        assert factorized['staged_id_capacity_bytes'] >= 3*g*f*8
        pair = dict(zip(['candidate','scenario','fanout','memo'],case))
        pair.update(represented_bindings=g*f**3, complete_emissions=g*f**3, factorized_emissions=3*g*f+g)
        for side, r in [('complete',complete),('factorized',factorized)]:
            pair[side] = {field+'_ms':statistics.median(r[field+'_s'])*1000 for field in ['fresh','warm']}
            pair[side].update({field+'_range_ms':[min(r[field+'_s'])*1000,max(r[field+'_s'])*1000] for field in ['fresh','warm']})
            for field in ['final_nodes','final_bytes_est','retained_input_colt_bytes','summary_colt_bytes','staged_id_capacity_bytes']:
                pair[side][field] = r[field]
        for field in ['fresh','warm']:
            pair[field+'_speedup'] = pair['complete'][field+'_ms']/pair['factorized'][field+'_ms']
        pairs.append(pair)
    return dict(passed=True,input=path.name,input_sha256=digest(path),binary_sha256=data['build_metadata']['binary_sha256'],
        checker_sha256=digest(Path(__file__)),trials=data['trials'],processes=len(data['runs']),configurations=len(rows),
        comparisons=pairs,
        boundary='Complete/factorized schedules from one binary; carrier and inputs held fixed. Both validate participating published roots. Factorized query includes staging, summary planning/images and exact counts. Estimates are not peak allocation measurements.')

def markdown(result):
    lines = ['# Native Pack factorization measurements','',
        'Same executable, input Events, scalar rows and outputs. Each row holds the carrier',
        'fixed and compares complete bindings with certified branch reductions.',
        f"Times are {result['trials']}-sample medians in milliseconds; speedup is complete / factorized.",
        'Raw samples and min/max ranges remain in the linked results.','',
        '| Carrier | Coup presentation | Fanout | Memo | Fresh complete | Fresh factored | Speedup | Warm complete | Warm factored | Speedup |',
        '| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |']
    for p in result['comparisons']:
        a,b=p['complete'],p['factorized']
        lines.append(f"| {p['candidate']} | {p['scenario']} | {p['fanout']} | {p['memo']} | {a['fresh_ms']:.4f} | {b['fresh_ms']:.4f} | {p['fresh_speedup']:.2f}× | {a['warm_ms']:.4f} | {b['warm_ms']:.4f} | {p['warm_speedup']:.2f}× |")
    lines += ['', '## Execution and measurement contract','',
        'For 64 groups and branch fanout f, the complete schedule emits 64f³ bindings.',
        'The factored schedule scans 192f branch rows and emits 64 final bindings;',
        'its 192 summary rows retain group presence independently of Event nonemptiness.',
        'Both report the same original 64f³ binding count. The summaries are rebuilt',
        'on every query, including warm queries.','',
        'Input-image setup and cloning the input-only Event arena are outside query timing.',
        'Validation, Event operations, staged root storage, summary planning/image/COLT',
        'construction, final Free Join and exact output cardinalities are inside.',
        'The input COLTs are primed for both schedules; fresh means a fresh Event arena.',
        'Do not compare these absolute times with the earlier carrier-only harness.','',
        'Memory fields distinguish retained Event/memo estimates, input COLT retention,',
        'summary COLT retention and staged root-vector capacities. They are not peak',
        'query allocation totals: map nodes, relation images and planner temporaries',
        'are not included in those estimates. Process RSS spans both schedules.','',
        'This fixture uses the proved Cartesian clover query. It does not establish a',
        'general optimizer or license the same rewrite on triangle joins. Correctness',
        'also covers absent branches, present empty results, duplicates and faults.',
        'Timing fixtures use populated groups and published valid roots.','',
        f"[Raw samples](results/{result['input']}) · [Matched comparison](results/{result['record']})",
        '· [Rewrite and evidence](FACTORIZED-PACK.md)','']
    return '\n'.join(lines)

if __name__ == '__main__':
    ap=argparse.ArgumentParser();ap.add_argument('input');ap.add_argument('--output',required=True);ap.add_argument('--record',required=True)
    args=ap.parse_args();result=analyze(LAB/'results'/args.input);result['record']=args.record
    (LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
    (LAB/args.output).write_text(markdown(result))
    print(json.dumps({k:v for k,v in result.items() if k!='comparisons'},indent=2))
