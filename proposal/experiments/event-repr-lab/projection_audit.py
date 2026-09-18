"""Replay the dependency-directed projection evidence, without native work."""
from pathlib import Path
import hashlib
import json
from prepare import LAB
from projection_comparison import collect, KEYS


def verify():
    def read(name): return json.loads((LAB/'results'/name).read_text())
    build = read('build-projection.json')
    tools = read('projection-tools/manifest.json')
    for name,digest in tools.items():
        assert hashlib.sha256((LAB/'results/projection-tools'/name).read_bytes()).hexdigest() == digest
    checks = {}
    for name in ['projection-witness-slab-check.json','projection-joint-slab-ite-check.json',
                 'projection-active-slab-ite-check.json','projection-witness-enum-ite-check.json',
                 'projection-witness-slab-staged-check.json']:
        r = read(name)
        assert r['passed'] and r['lab_sources'] == build['lab_sources']
        rows = [json.loads(line.split('EVENT_LAB ',1)[1])
                for line in (LAB/'results'/r['log']).read_text().splitlines() if 'EVENT_LAB {' in line]
        gate_checks = [r for r in rows if r.get('kind') == 'projection_gate_verification']
        assert len(gate_checks) == 6
        assert all(r['passed'] and r['policies'] == 3 and r['product_cases'] == 49152
                   and r['coupled_cases'] == 2304 for r in gate_checks)
        checks[name] = dict(decoder_cutoff_combinations=6,product_results_per_combination=49152,
                           coupled_results_per_combination=2304,policies=3)
    comparison = read('projection-comparison.json')
    assert comparison['checker_sha256'] == tools['projection_comparison.py']
    rows, processes, binary, pairs = collect(comparison['inputs'])
    assert comparison['passed'] and comparison['semantic_failures'] == comparison['incomplete_processes'] == 0
    assert comparison['cases'] == len(rows) == 264 and comparison['matched_cases'] == len(pairs) == 816
    assert comparison['pairs'] == pairs and comparison['processes'] == processes
    assert comparison['binary_sha256'] == binary == build['binary_sha256']
    assert len(processes) == 74 and all(p['status'] == 'passed' for p in processes)
    for name in comparison['inputs']:
        r = read(name)
        assert r['build_metadata'] == build and r['checker_sha256'] == tools['projection_sweep.py']
    identity_pairs = 0
    for pair in pairs:
        if pair['comparison'] not in ['projection','projection-strength']: continue
        a = rows[tuple(pair['case'][k] for k in KEYS)][0]
        b = rows[tuple(pair['baseline_case'][k] for k in KEYS)][0]
        # Full support and certified prefix suffix readouts bypass these gates.
        if a['domain'] == 'full' or (a['candidate'].startswith('prefix') and a['readout_family'].startswith('suffix')):
            for field in ['input_nodes','final_nodes','input_bytes_est','final_bytes_est','input_storage','final_storage','outer_memo_bytes']:
                assert a[field] == b[field], field
            identity_pairs += 1
    assert identity_pairs == 84
    diagnostic_cases = {}
    diagnostics = {}
    for gates in ['joint','active','witness']:
        name = f'projection-{gates}-diagnostics.json'
        r = read(name)
        assert r['build_metadata'] == build and r['checker_sha256'] == tools['projection_diagnostics.py']
        assert len(r['runs']) == 1 and r['runs'][0]['status'] == 'passed'
        run = r['runs'][0]
        assert run['job']['EVENT_LAB_REACHABILITY'] == '1' and run['job']['EVENT_LAB_RETRACTION_PROJECT'] == gates
        count = 0
        for row in run['rows']:
            if row.get('kind') != 'readout_reachability': continue
            count += 1
            g = row['graphs']; w = row['width']; key = (w,row['memo'])
            base = diagnostic_cases.setdefault(key,g)
            for field in ['inputs','outputs','output_masks','output_records']:
                assert g[field] == base[field], (gates,key,field)
            assert len(g['output_masks']) == len(g['output_records']) == 64
            allowed = ((1 << (2*w))-1) | (1 << (3*w))
            for mask,size in zip(g['output_masks'],g['output_records']):
                assert mask & ~allowed == 0
                d = bin(mask).count('1')
                assert size <= (1 if d <= 9 else (1 << (d-8))-1)
            assert g['outputs']['records'] <= g['live_union']['records'] <= g['arena_records']
        assert count == 4
        diagnostics[gates] = dict(processes=1,censuses=count)
    acceptance = read('projection-acceptance.json')
    assert acceptance['build_metadata'] == build
    assert acceptance['checker_sha256'] == tools['projection_acceptance.py']
    assert len(acceptance['runs']) == 32
    accepted = {}
    for run in acceptance['runs']:
        assert run['status'] == 'passed'
        assert run['job']['EVENT_LAB_RETRACTION_PROJECT'] in ['active','witness']
        for row in run['rows']:
            assert row['verified']
            kind = row['kind']
            if kind == 'owned_free_join': assert row['output_roots'] == 80 and all(n>0 for n in row['novel_publication_classes'])
            elif kind == 'symbolic_relation_free_join': assert row['new_classes']>0 and len(row['output_counts']) == 5
            else: raise AssertionError(kind)
            accepted[kind] = accepted.get(kind,0)+1
    assert accepted == {'owned_free_join':96,'symbolic_relation_free_join':48}
    return dict(shared_checks=checks,tools=len(tools),cases=len(rows),matched_cases=len(pairs),
                processes=len(processes),storage_identity_pairs=identity_pairs,
                diagnostics=diagnostics,acceptance_processes=32,acceptance_rows=accepted)


if __name__ == '__main__': print(json.dumps(verify(),indent=2))
