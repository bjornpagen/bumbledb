"""Validate the direct-ITE round without executing native work."""
from pathlib import Path
import hashlib
import json
from prepare import LAB
from completion_comparison import collect, KEYS


def verify():
    def read(name): return json.loads((LAB/'results'/name).read_text())
    build = read('build-ite.json')
    tools = read('ite-tools/manifest.json')
    for name, digest in tools.items():
        assert hashlib.sha256((LAB/'results/ite-tools'/name).read_bytes()).hexdigest() == digest
    reading = read('ite-reading.json')
    for name, digest in reading['files'].items():
        assert hashlib.sha256((LAB.parents[2]/name).read_bytes()).hexdigest() == digest
    checks = {}
    for name in ['ite-slab-check.json','ite-staged-slab-derived-check.json',
                 'ite-ite-enum-derived-check.json','ite-equal-enum-derived-check.json',
                 'ite-ite-slab-scalar-check.json']:
        r = read(name)
        assert r['passed'] and r['lab_sources'] == build['lab_sources']
        rows = [json.loads(l.split('EVENT_LAB ',1)[1])
                for l in (LAB/'results'/r['log']).read_text().splitlines() if 'EVENT_LAB {' in l]
        small = [r for r in rows if r['kind'] == 'ite_small_verification']
        scattered = [r for r in rows if r['kind'] == 'ite_scattered_verification']
        normalization = [r for r in rows if r['kind'] == 'retraction_verification']
        assert sorted((r['cutoff'],r['cases']) for r in small) == [(1,8192),(2,8192),(6,8192),(9,8192)]
        assert sorted((r['cutoff'],r['cases'],r['worlds_per_case']) for r in scattered) == [(6,96,4096),(9,96,4096)]
        assert len(normalization) == 6 and all(r['normalization_inputs'] == 160 for r in normalization)
        assert all(r.get('passed',r.get('verified',False)) for r in rows)
        checks[name] = dict(small_triples=32768,scattered_cases=192,worlds_per_scattered_case=4096,
                           decoder_cutoff_cases=6,normalization_inputs_per_case=160)
    comparison = read('ite-comparison.json')
    assert comparison['checker_sha256'] == tools['completion_comparison.py']
    rows, processes, binary, pairs = collect(comparison['inputs'])
    assert comparison['cases'] == len(rows) and comparison['matched_cases'] == len(pairs)
    assert comparison['pairs'] == pairs and comparison['processes'] == processes
    assert comparison['binary_sha256'] == binary == build['binary_sha256']
    assert comparison['semantic_failures'] == 0
    assert comparison['incomplete_processes'] == sum(p['status'] != 'passed' for p in processes)
    assert comparison['passed'] == all(p['status'] == 'passed' for p in processes)
    caps = [p for p in processes if p['status'] != 'passed']
    assert len(caps) == 4
    for cap in caps:
        assert cap['status'] == 'resource_cap' and cap['job']['EVENT_LAB_RETRACTION_NORMALIZE'] == 'staged'
        peer = dict(cap['job'],EVENT_LAB_RETRACTION_NORMALIZE='ite')
        assert any(p['job'] == peer and p['status'] == 'passed' for p in processes)
    full_pairs = 0
    for pair in pairs:
        if pair['comparison'] != 'normalization' or pair['case']['domain'] != 'full': continue
        a = rows[tuple(pair['case'][k] for k in KEYS)][0]
        b = rows[tuple(pair['baseline_case'][k] for k in KEYS)][0]
        for field in ['input_nodes','final_nodes','input_bytes_est','final_bytes_est','input_storage','final_storage','outer_memo_bytes']:
            assert a[field] == b[field]
        full_pairs += 1
    assert full_pairs == 16
    for name in comparison['inputs']:
        r = read(name)
        assert r['build_metadata'] == build and r['checker_sha256'] == tools['completion_sweep.py']
    diagnostics = {}
    for name in ['ite-first-diagnostics.json','ite-cap64-diagnostics.json','ite-prefix64-diagnostics.json','ite-min512-diagnostics.json']:
        r = read(name)
        assert r['build_metadata'] == build and r['checker_sha256'] == tools['ite_diagnostics.py']
        cases = {}
        caps = []
        census_count = 0
        for run in r['runs']:
            job = run['job']
            assert job['EVENT_LAB_REACHABILITY'] == '1'
            assert run['status'] in ['passed','resource_cap']
            if run['status'] == 'resource_cap':
                log = (LAB/'results'/run['log']).read_text()
                assert 'readout::verify_batch' in log and 'RegionOps>::import' in log
                caps.append(job)
            for row in run['rows']:
                if row.get('kind') != 'readout_reachability': continue
                g = row['graphs']; w = row['width']; census_count += 1
                key = tuple(job[k] for k in ['EVENT_LAB_CANDIDATE','EVENT_LAB_READOUT_FAMILY','EVENT_LAB_LAYOUT'])+(w,row['memo'])
                base = cases.setdefault(key,g)
                for field in ['inputs','outputs','infrastructure','live_union','output_masks','output_records']:
                    assert g[field] == base[field], (key,field)
                assert len(g['output_masks']) == len(g['output_records']) == 64
                allowed = ((1 << (2*w))-1) | (1 << (3*w))
                cutoff = 9 if row['candidate'].endswith('512') else 6
                for mask,size in zip(g['output_masks'],g['output_records']):
                    assert mask & ~allowed == 0
                    d = bin(mask).count('1')
                    assert size <= (1 if d <= cutoff else (1 << (d-cutoff+1))-1)
                assert g['live_union']['records'] <= g['arena_records']
        diagnostics[name] = dict(processes=len(r['runs']),censuses=census_count,
                                  caps=len(caps),phase='any caps occur in untimed canonical reimport')
    acceptance = read('ite-acceptance.json')
    assert acceptance['build_metadata'] == build
    assert acceptance['checker_sha256'] == tools['completion_acceptance.py']
    assert len(acceptance['runs']) == 32
    accepted = {}
    for run in acceptance['runs']:
        assert run['status'] == 'passed'
        for row in run['rows']:
            assert row['verified']
            kind = row['kind']
            if kind == 'owned_free_join': assert row['output_roots'] == 80 and all(n>0 for n in row['novel_publication_classes'])
            elif kind == 'symbolic_relation_free_join': assert row['new_classes']>0 and len(row['output_counts']) == 5
            else: raise AssertionError(kind)
            accepted[kind] = accepted.get(kind,0)+1
    assert accepted == {'owned_free_join':96,'symbolic_relation_free_join':48}
    initial = read('ite-lean-initial/lean-check.json')
    assert not initial['passed']
    assert "unexpected token 'variable'" in (LAB/'results/ite-lean-initial/lean-check.log').read_text()
    assert initial['source_hashes']['lean/DirectConditional.lean'] == hashlib.sha256((LAB/'results/ite-lean-initial/DirectConditional.lean').read_bytes()).hexdigest()
    assert initial['verifier_sha256'] == hashlib.sha256((LAB/'results/lean-v17/verify_lean.py').read_bytes()).hexdigest()
    return dict(shared_checks=checks,former_cap_cases_completed=4,full_support_storage_identity_pairs=full_pairs,tools=len(tools),cases=len(rows),matched_cases=len(pairs),
                processes=len(processes),incomplete_processes=comparison['incomplete_processes'],
                diagnostics=diagnostics,acceptance_processes=32,acceptance_rows=accepted)


if __name__ == '__main__':
    print(json.dumps(verify(),indent=2))
