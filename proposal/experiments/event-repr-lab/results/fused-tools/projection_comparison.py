"""Compare joint, active and witness gates without changing the readout query."""
from pathlib import Path
import argparse
import hashlib
import json
import statistics as stats
from prepare import LAB

KEYS = ['domain', 'width', 'bits', 'readout_family', 'readout_faces', 'layout', 'memo',
        'candidate', 'essential_layout', 'retraction_count', 'retraction_factor', 'retraction_normalize', 'retraction_project']
FIELDS = [('domain', 'EVENT_LAB_LEGAL_DOMAIN'), ('readout_family', 'EVENT_LAB_READOUT_FAMILY'),
          ('readout_faces', 'EVENT_LAB_READOUT_FACES'), ('layout', 'EVENT_LAB_LAYOUT'),
          ('candidate', 'EVENT_LAB_CANDIDATE'), ('essential_layout', 'EVENT_LAB_ESSENTIAL_LAYOUT'),
          ('retraction_count', 'EVENT_LAB_RETRACTION_COUNT'), ('retraction_factor', 'EVENT_LAB_RETRACTION_FACTOR'),
          ('retraction_normalize', 'EVENT_LAB_RETRACTION_NORMALIZE'), ('retraction_project', 'EVENT_LAB_RETRACTION_PROJECT')]
PHASES = ['build_s', 'support_s', 'import_s', 'admission_s', 'join_only_s',
          'fresh_s', 'fresh_group_s', 'fresh_readout_s', 'fresh_compute_s', 'fresh_count_s',
          'warm_s', 'warm_group_s', 'warm_readout_s', 'warm_compute_s', 'warm_count_s']


def collect(inputs):
    rows, processes, builds = {}, [], set()
    for name in inputs:
        data = json.loads((LAB/'results'/name).read_text())
        builds.add(data['build_metadata']['binary_sha256'])
        for run in data['runs']:
            job = run['job']
            assert job.get('EVENT_LAB_RETRACTION_EXECUTE','staged') == 'staged', 'Use the direct-projection collector'
            assert job.get('EVENT_LAB_RETRACTION_COMPLETE','full') == 'full', 'Use the completion-strategy collector'
            assert job.get('EVENT_LAB_REACHABILITY','0') == '0', 'Diagnostic runs cannot contribute comparison rows'
            processes.append(dict(file=name, job=job, status=run['status'], log=run['log']))
            if run['status'] != 'passed':
                if job.get('EVENT_LAB_LANE') == 'readouts':
                    for key in list(rows):
                        r = rows[key][0]
                        if all(r[field] == job.get(env) for field, env in FIELDS):
                            del rows[key]
                continue
            for r in run['rows']:
                if r.get('kind') != 'readout_free_join':
                    continue
                assert r['verified'] and r['essential_kernel'] == 'derived'
                assert r['retraction_project'] in ['joint','active','witness']
                assert r['retraction_normalize'] in ['staged', 'equal', 'ite']
                if not r['candidate'].startswith(('prefix','retraction')):
                    assert r['retraction_normalize'] == 'staged' and r['retraction_project'] == 'joint'
                assert r['requested_product_mode'] == 'materialized'
                for field, env in FIELDS:
                    assert r[field] == job[env], (name, field)
                assert r['rows'] == 128 and r['groups'] == 16 and r['outputs'] == 64
                width = r['width']
                assert width in (4, 6) and r['bits'] == 3*width+(r['domain'] == 'fibred')
                assert r['worlds'] == 1 << r['bits'] and r['checksum'] > 0
                local = {'suffix-1': 1, 'suffix-half': (1 << (width//2))-1,
                         'whole': (1 << width)-1, 'non-suffix': 1 << (width-1)}[r['readout_family']]
                hidden = {'x': local, 'y': local << width, 'xy': local | (local << width)}[r['readout_faces']]
                assert r['hidden'] == hidden
                size = 1 << width
                holes = sum((v+1) % 3 != 0 for v in range(size-1))
                population = {'full': size**3, 'below': (size-3)**3, 'holes': holes**3,
                              'fibred': (size-3)**3+holes**3}[r['domain']]
                assert r['admissible_worlds'] == population
                assert r['environments'] == (2 if r['domain'] == 'fibred' else 1)
                assert 0 <= r['changed_possible_groups'] <= 16
                assert 0 <= r['changed_guaranteed_groups'] <= 16
                assert r['changed_possible_groups']+r['changed_guaranteed_groups'] > 0
                assert 0 <= r['nonconstant_readouts'] <= 48
                assert all(len(r[phase]) == data['trials'] for phase in PHASES)
                rows[tuple(r[k] for k in KEYS)] = (r, name)
    assert len(builds) == 1, 'Do not mix executables'
    assert rows, 'No successful readout query rows'
    answers = {}
    for r, _ in rows.values():
        key = (r['domain'], r['bits'], r['readout_family'], r['readout_faces'])
        answer = tuple(r[k] for k in ['checksum', 'changed_possible_groups', 'changed_guaranteed_groups', 'nonconstant_readouts'])
        assert answers.setdefault(key, answer) == answer
    pairs = []
    for key, (r, source) in rows.items():
        choices = []
        if r['retraction_normalize'] in ['equal','ite']:
            choices.append(('normalization', dict(retraction_normalize='staged')))
        if r['retraction_project'] != 'joint':
            choices.append(('projection',dict(retraction_project='joint')))
        if r['retraction_project'] == 'witness':
            choices.append(('projection-strength',dict(retraction_project='active')))
        candidate = r['candidate']
        if candidate.startswith('prefix'):
            choices.append(('decoder', dict(candidate=candidate.replace('prefix', 'retraction'))))
        if candidate.startswith(('prefix', 'retraction')):
            suffix = candidate.removeprefix('prefix').removeprefix('retraction')
            choices.append(('anchor', dict(candidate='essential'+suffix, retraction_factor='joint', retraction_normalize='staged',retraction_project='joint')))
            if r['retraction_factor'] == 'faces':
                choices.append(('count', dict(retraction_factor='joint')))
        if candidate.startswith(('prefix', 'retraction', 'essential')):
            if candidate.endswith('512'):
                choices.append(('cutoff', dict(candidate=candidate[:-3]+'64')))
            if r['essential_layout'] == 'slab':
                choices.append(('storage', dict(essential_layout='enum')))
            for control in ['packed512', 'dense-dispatched']:
                choices.append(('control', dict(candidate=control, essential_layout='enum', retraction_factor='joint', retraction_normalize='staged',retraction_project='joint')))
        if r['layout'] == 'bit-major':
            choices.append(('order', dict(layout='face-major')))
        for comparison, changes in choices:
            base_case = dict(zip(KEYS, key))
            base_case.update(changes)
            other_key = tuple(base_case[k] for k in KEYS)
            if other_key not in rows:
                continue
            base, other_source = rows[other_key]
            if comparison in ['projection','projection-strength']:
                for field in ['input_nodes','input_bytes_est','input_storage']:
                    assert r[field] == base[field], field
            if comparison == 'count':
                for field in ['input_nodes', 'final_nodes', 'input_bytes_est', 'final_bytes_est', 'input_storage', 'final_storage', 'outer_memo_bytes']:
                    assert r[field] == base[field], field
            if comparison == 'storage':
                assert r['input_nodes'] == base['input_nodes'] and r['final_nodes'] == base['final_nodes']
            phases = {}
            for phase in PHASES:
                am, bm = stats.median(r[phase]), stats.median(base[phase])
                phases[phase] = dict(candidate_median_s=am, baseline_median_s=bm,
                                     baseline_over_candidate=bm/am,
                                     candidate_min_s=min(r[phase]), candidate_max_s=max(r[phase]),
                                     baseline_min_s=min(base[phase]), baseline_max_s=max(base[phase]),
                                     candidate_samples=len(r[phase]), baseline_samples=len(base[phase]))
            pairs.append(dict(comparison=comparison, case=dict(zip(KEYS, key)), baseline_case=base_case,
                              source=source, baseline_source=other_source, phases=phases,
                              candidate_final_bytes_est=r['final_bytes_est'], baseline_final_bytes_est=base['final_bytes_est']))
    return rows, processes, next(iter(builds)), pairs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('inputs', nargs='+')
    parser.add_argument('--output', default='PROJECTION-GATE-MEASUREMENTS.md')
    parser.add_argument('--record', default='projection-comparison.json')
    args = parser.parse_args()
    rows, processes, binary, pairs = collect(args.inputs)
    record = dict(passed=all(p['status'] == 'passed' for p in processes),
                  semantic_failures=sum(p['status'] == 'failed' for p in processes),
                  incomplete_processes=sum(p['status'] != 'passed' for p in processes),
                  inputs=args.inputs, binary_sha256=binary, cases=len(rows), matched_cases=len(pairs),
                  pairs=pairs, processes=processes, checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  selection='Latest supplied successful process per exact case, invalidated by a later failed/capped case. Retain all raw samples. Never compare different hidden masks as the same query.')
    (LAB/'results'/args.record).write_text(json.dumps(record, indent=2)+'\n')
    lines = ['# Same-binary dependency-directed projection after Free Join', '',
             'Every query returns 64 Events: original, Possible, Guaranteed and Ambiguous',
             'for sixteen groups. Fresh/warm totals include exact original-support counts.',
             'Every returned Event is checked pointwise and canonically reimported outside',
             'timing. Construction is separate. Bytes estimate retained carrier/cache memory.',
             'They exclude allocator overhead, verification clones, result vectors and the join engine.',
             f'Executable: `{binary}`.', '',
             'Only entirely successful processes contribute timing rows. Partial rows from',
             'capped or failed processes remain in the raw evidence but cannot supply a',
             'complete-query comparison. A later incomplete process invalidates an older',
             'successful process for that exact job; no failed result is a fast answer.', '']
    incomplete = [p for p in processes if p['status'] != 'passed']
    if incomplete:
        lines += ['## Incomplete processes', '',
                  '| Carrier | Normalization | Domain | Readout | Order | Status | Log |',
                  '| --- | --- | --- | --- | --- | --- | --- |']
        for p in incomplete:
            j = p['job']
            cells = [j.get('EVENT_LAB_CANDIDATE', 'verification'),
                     j.get('EVENT_LAB_RETRACTION_NORMALIZE', '')+'/'+j.get('EVENT_LAB_RETRACTION_PROJECT',''),
                     j.get('EVENT_LAB_LEGAL_DOMAIN', ''),
                     j.get('EVENT_LAB_READOUT_FAMILY', '')+'/'+j.get('EVENT_LAB_READOUT_FACES', ''),
                     j.get('EVENT_LAB_LAYOUT', ''), p['status'],
                     f'[Retained log](results/{p["log"]})']
            lines.append('| '+' | '.join(cells)+' |')
        lines += ['']
    for group in sorted({key[:7] for key in rows}):
        domain, width, bits, family, faces, layout, memo = group
        lines += [f'## {domain}, width {width}, {family}/{faces}, {layout}, memo {memo}', '',
                  '| Carrier/store/mode | Factor | Build ms | Fresh ms | Group ms | Readout ms | Count ms | Fresh range | Warm ms | KB | Nodes | Samples |',
                  '| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |']
        for key, (r, _) in sorted(rows.items()):
            if key[:7] != group:
                continue
            values = [r['candidate']+'/'+r['essential_layout']+'/'+r['retraction_normalize']+'/'+r['retraction_project'], r['retraction_factor']]
            values += [f'{1000*stats.median(r[p]):.3f}' for p in ['build_s', 'fresh_s', 'fresh_group_s', 'fresh_readout_s', 'fresh_count_s']]
            values += [f'{1000*min(r["fresh_s"]):.3f}–{1000*max(r["fresh_s"]):.3f}',
                       f'{1000*stats.median(r["warm_s"]):.3f}', f'{r["final_bytes_est"]/1000:.1f}',
                       str(r['final_nodes']), str(len(r['fresh_s']))]
            lines.append('| '+' | '.join(values)+' |')
        example = next(r for key, (r, _) in rows.items() if key[:7] == group)
        lines += ['', f'Groups changed by Possible: {example["changed_possible_groups"]}/16; by Guaranteed: {example["changed_guaranteed_groups"]}/16. Nonconstant readout outputs: {example["nonconstant_readouts"]}/48.', '']
    lines += [f'{len(rows)} configurations; {len(pairs)} matched comparisons; {len(processes)} retained processes.', '']
    lines += [f'- [Raw evidence](results/{name}).' for name in args.inputs]
    (LAB/args.output).write_text('\n'.join(lines)+'\n')
    print(json.dumps({k: record[k] for k in ['passed', 'cases', 'matched_cases']}))


if __name__ == '__main__':
    main()
