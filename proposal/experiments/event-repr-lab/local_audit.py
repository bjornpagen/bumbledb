"""Replay local-completion evidence, preserving previous immutable rounds."""
from pathlib import Path
import hashlib
import json
from prepare import LAB
from local_comparison import collect, KEYS

CHECKS = ['local-first-slab-check.json', 'local-enum-ite-local-check.json',
          'local-slab-staged-local-needed-check.json', 'local-enum-equal-needed-check.json']


def verify():
    def read(name): return json.loads((LAB/'results'/name).read_text())
    build = read('build-local.json')
    tools = read('local-tools/manifest.json')
    for name, digest in tools.items():
        assert hashlib.sha256((LAB/'results/local-tools'/name).read_bytes()).hexdigest() == digest
    checks = {}
    for name in CHECKS:
        r = read(name)
        assert r['passed'] and r['lab_sources'] == build['lab_sources']
        rows = [json.loads(line.split('EVENT_LAB ', 1)[1])
                for line in (LAB/'results'/r['log']).read_text().splitlines() if 'EVENT_LAB {' in line]
        selected = [r for r in rows if r.get('kind') == 'local_completion_verification']
        assert len(selected) == 6
        assert all(r['passed'] and r['policies'] == 3 and r['strategies'] == 4
                   and r['product_cases'] == 196608 and r['coupled_cases'] == 9216
                   and r['substitution_cases'] == 65536 for r in selected)
        checks[name] = dict(decoder_cutoff_combinations=6, product_results_per_combination=196608,
                           coupled_results_per_combination=9216, substitution_results_per_combination=65536,
                           gate_policies=3, completion_strategies=4)
    comparison = read('local-comparison.json')
    assert comparison['checker_sha256'] == tools['local_comparison.py']
    rows, processes, binary, pairs = collect(comparison['inputs'])
    assert comparison['passed'] and comparison['semantic_failures'] == comparison['incomplete_processes'] == 0
    assert comparison['cases'] == len(rows) == 368 and comparison['matched_cases'] == len(pairs) == 1472
    assert comparison['pairs'] == pairs and comparison['processes'] == processes
    assert comparison['binary_sha256'] == binary == build['binary_sha256']
    assert len(processes) == 110 and all(p['status'] == 'passed' for p in processes)
    for name in comparison['inputs']:
        r = read(name)
        assert r['build_metadata'] == build and r['checker_sha256'] == tools['local_sweep.py']
    identity_pairs = 0
    for pair in pairs:
        if pair['comparison'] not in ['completion', 'locality', 'traversal']: continue
        a = rows[tuple(pair['case'][k] for k in KEYS)][0]
        b = rows[tuple(pair['baseline_case'][k] for k in KEYS)][0]
        bypass = a['domain'] == 'full' or (a['candidate'].startswith('prefix') and a['readout_family'].startswith('suffix'))
        fallback = a['retraction_project'] != 'witness' and (
            (a['retraction_complete'], b['retraction_complete']) in [('local', 'full'), ('local-needed', 'needed')])
        if bypass or fallback:
            for field in ['input_nodes', 'final_nodes', 'input_bytes_est', 'final_bytes_est',
                          'input_storage', 'final_storage', 'outer_memo_bytes']:
                assert a[field] == b[field], (pair['case'], field)
            identity_pairs += 1
    assert identity_pairs == 152
    diagnostic_cases, diagnostics = {}, {}
    for completion in ['full', 'needed', 'local', 'local-needed']:
        r = read(f'local-{completion}-diagnostics.json')
        assert r['build_metadata'] == build and r['checker_sha256'] == tools['local_diagnostics.py']
        assert len(r['runs']) == 1 and r['runs'][0]['status'] == 'passed'
        run = r['runs'][0]
        assert run['job']['EVENT_LAB_REACHABILITY'] == '1'
        assert run['job']['EVENT_LAB_RETRACTION_COMPLETE'] == completion
        count = 0
        for row in run['rows']:
            if row.get('kind') != 'readout_reachability': continue
            count += 1
            g = row['graphs']; w = row['width']; key = (w, row['memo'])
            base = diagnostic_cases.setdefault(key, g)
            for field in ['inputs', 'outputs', 'output_masks', 'output_records', 'infrastructure', 'live_union']:
                assert g[field] == base[field], (completion, key, field)
            assert len(g['output_masks']) == len(g['output_records']) == 64
            assert g['outputs']['records'] <= g['live_union']['records'] <= g['arena_records']
        assert count == 4
        diagnostics[completion] = dict(processes=1, censuses=count)
    acceptance = read('local-acceptance.json')
    assert acceptance['build_metadata'] == build
    assert acceptance['checker_sha256'] == tools['local_acceptance.py']
    assert len(acceptance['runs']) == 32
    accepted = {}
    for run in acceptance['runs']:
        assert run['status'] == 'passed'
        assert run['job']['EVENT_LAB_RETRACTION_COMPLETE'] in ['local', 'local-needed']
        for row in run['rows']:
            assert row['verified']; kind = row['kind']
            if kind == 'owned_free_join': assert row['output_roots'] == 80 and all(n > 0 for n in row['novel_publication_classes'])
            elif kind == 'symbolic_relation_free_join': assert row['new_classes'] > 0 and len(row['output_counts']) == 5
            else: raise AssertionError(kind)
            accepted[kind] = accepted.get(kind, 0) + 1
    assert accepted == {'owned_free_join':96, 'symbolic_relation_free_join':48}
    return dict(shared_checks=checks, tools=len(tools), cases=len(rows), matched_cases=len(pairs),
                processes=len(processes), storage_identity_pairs=identity_pairs,
                diagnostics=diagnostics, acceptance_processes=32, acceptance_rows=accepted)


if __name__ == '__main__': print(json.dumps(verify(), indent=2))
