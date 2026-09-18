"""Compare matched layouts; reject changed normal forms or mismatched binaries."""
from pathlib import Path
import argparse, hashlib, json, statistics
from prepare import LAB

KEYS = ['kind','candidate','scenario','shape','bits','states','memo','classifier',
        'predicate','parameter_groups','layout','target_candidate','target_layout',
        'packed_map','transfer_import','identity_mode','essential_kernel']
LOGICAL = ['records','tables','logical_words']
OUTPUTS = ['checksum','admissible_worlds','rows','accepted','histogram',
           'output_counts','output_packet_bytes','input_roots','output_roots',
           'signature_cache_entries','evidence_admissible_worlds','examples_beta','examples_mean',
           'closure_iterations','new_classes']

def collect(inputs):
    rows, processes, builds = {}, [], set()
    for name in inputs:
        data = json.loads((LAB/'results'/name).read_text())
        builds.add(data['build_metadata']['binary_sha256'])
        for run in data['runs']:
            processes.append(dict(file=name,job=run['job'],status=run['status'],
                                  max_rss_bytes=run['max_rss_bytes'],log=run['log']))
            for row in run['rows']:
                if row.get('kind') == 'verification' or 'verified' not in row: continue
                assert run['status'] == 'passed' and row['verified']
                assert row['essential_layout'] == run['job']['EVENT_LAB_ESSENTIAL_LAYOUT']
                assert row['essential_kernel'] == 'derived'
                for field in ['input_storage','final_storage']:
                    s = row[field]
                    if s is None: continue
                    assert s['layout'] == row['essential_layout']
                    assert s['total_bytes_est'] == sum(s[k] for k in
                        ['record_bytes','table_bytes','interner_bytes','metadata_bytes','cache_bytes'])
                if row['kind'] == 'signature_free_join' and row['classifier'] == 'direct':
                    assert row['input_storage'] == row['final_storage']
                    assert row['input_bytes_est'] == row['final_bytes_est']
                    assert row['input_nodes'] == row['final_nodes']
                key = tuple(row.get(k) for k in KEYS)+(row['essential_layout'],)
                rows[key] = (row,name,run['max_rss_bytes'])
    assert len(builds) == 1, 'One executable per matched comparison.'
    pairs = []
    for key,(slab,source,rss) in rows.items():
        if key[-1] != 'slab': continue
        enum,enum_source,enum_rss = rows[key[:-1]+('enum',)]
        for field in OUTPUTS:
            assert slab.get(field) == enum.get(field), (key,field)
        for field in ['input_storage','final_storage']:
            a,b = enum[field],slab[field]
            assert (a is None) == (b is None)
            if a is not None:
                assert all(a[k] == b[k] for k in LOGICAL), (key,field,a,b)
        phases = {k:dict(enum_median_s=statistics.median(enum[k]),
                         slab_median_s=statistics.median(slab[k]),
                         enum_min_s=min(enum[k]),enum_max_s=max(enum[k]),
                         slab_min_s=min(slab[k]),slab_max_s=max(slab[k]),
                         enum_samples=len(enum[k]),slab_samples=len(slab[k]),
                         enum_over_slab=statistics.median(enum[k])/statistics.median(slab[k]))
                  for k in slab if k.endswith('_s') and isinstance(slab[k],list)
                  and slab[k] and all(isinstance(x,(int,float)) for x in slab[k])}
        pairs.append(dict(case=dict(zip(KEYS,key[:-1])),
                          enum_source=enum_source,slab_source=source,
                          enum_rss=enum_rss,slab_rss=rss,phases=phases,
                          enum_final_bytes_est=enum.get('final_arena_bytes_est',enum.get('final_bytes_est')),
                          slab_final_bytes_est=slab.get('final_arena_bytes_est',slab.get('final_bytes_est')),
                          enum_storage=enum['final_storage'],slab_storage=slab['final_storage']))
    return rows,processes,next(iter(builds)),pairs

def case(r):
    xs = [r.get('scenario') or str(r.get('bits',''))+' bits']
    if r.get('shape'): xs.append(r['shape'])
    if 'memo' in r: xs.append('memo '+('on' if r['memo'] else 'off'))
    if r.get('classifier'): xs += [r['classifier'],r['predicate']]
    if r.get('parameter_groups'): xs.append(str(r['parameter_groups'])+' parameter groups')
    if r.get('target_candidate'): xs += ['→ '+r['target_candidate'],r['target_layout']]
    return ', '.join(xs)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('inputs',nargs='+')
    ap.add_argument('--output',default='SLAB-MEASUREMENTS.md')
    ap.add_argument('--record',default='slab-comparison.json')
    args = ap.parse_args()
    rows,processes,binary,pairs = collect(args.inputs)
    report = dict(passed=True,binary_sha256=binary,inputs=args.inputs,
                  checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  cases=len(rows),matched_layout_cases=len(pairs),
                  processes=processes,pairs=pairs,
                  rule='Latest supplied process per exact case, never fastest selection; paired logical counts and outputs must match.')
    (LAB/'results'/args.record).write_text(json.dumps(report,indent=2)+'\n')
    lines = ['# Essential storage: one executable, two exact layouts','',
        'Generated by `slab_comparison.py`; latest supplied process per exact case.',
        'Both layouts use the derived algorithms, identical cutoff and normal form.',
        'All paired input/final record, table and logical-word counts must match.',
        'The slab removes duplicated resident table buffers and stores sixteen-byte',
        'records; full comparisons still resolve every fingerprint collision.',
        'This is a bundled physical storage change, including the interner shape.',
        'It does not isolate the contribution of each allocation or hash probe.',
        'Times are milliseconds. Speed ratio is enum median / slab median; greater',
        'than one favors slab. Samples are within-process observations, not formal',
        'confidence intervals. Close differences need independent process repeats.',
        'Memory is retained capacity, with hash tuple + control-byte estimates.',
        'It excludes allocator overhead and temporary scratch; process peak RSS',
        'also contains the executor, oracle, input copies and test runtime.',
        'Storage counters are sampled after cloning and outside all timed queries.',
        'The table uses the existing lane estimate of retained bytes for both layouts',
        'and controls; this includes external algebra memos when that lane charges',
        'them. Split carrier components in the JSON exclude those external memos.',
        'See [contracts and acceptance criteria](SLABS.md).','',
        f'Executable SHA-256: `{binary}`.','']
    kinds = [('free_join','Coup Boolean query','fresh_s'),
             ('owned_free_join','Owned full relation query','owned_compute_s'),
             ('law_free_join','Shared-parameter observation after relation query','fresh_s'),
             ('symbolic_relation_free_join','Symbolic full counter relation algebra','owned_compute_s'),
             ('signature_free_join','Relationship filtering in Free Join','fresh_s')]
    for kind,title,phase in kinds:
        selected = [p for p in pairs if p['case']['kind'] == kind]
        if not selected: continue
        selected.sort(key=lambda p: (case({k:v for k,v in p['case'].items() if v is not None}),p['case']['candidate']))
        lines += ['## '+title,'',f'Primary phase: `{phase}`. Full phase medians are in the [comparison record](results/{args.record}).','',
                  '| Case | Carrier | Enum ms | Slab ms | Ratio | Enum KB | Slab KB | Records / tables / words | Samples enum / slab |',
                  '| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: |']
        for p in selected:
            r = {k:v for k,v in p['case'].items() if v is not None}
            phase_data = p['phases'][phase]
            a,b = p['enum_storage'],p['slab_storage']
            storage = [f"{p['enum_final_bytes_est']/1000:.1f}",f"{p['slab_final_bytes_est']/1000:.1f}",
                       ' / '.join(str(b[k]) for k in LOGICAL) if a else 'destination is a control']
            cells = [case(r),r['candidate'],f"{phase_data['enum_median_s']*1000:.3f}",
                     f"{phase_data['slab_median_s']*1000:.3f}",f"{phase_data['enum_over_slab']:.2f}",*storage,
                     f"{phase_data['enum_samples']} / {phase_data['slab_samples']}"]
            lines.append('| '+' | '.join(cells)+' |')
        lines += ['', '### Same-binary controls','',
                  '| Case | Carrier | Primary ms | Warm ms | Retained KB | Samples |',
                  '| --- | --- | ---: | ---: | ---: | ---: |']
        controls = [r for r,_,_ in rows.values() if r['kind'] == kind and not r['candidate'].startswith('essential')]
        controls.sort(key=lambda r: (case(r),r['candidate']))
        for r in controls:
            memory = r.get('final_arena_bytes_est',r.get('final_bytes_est'))
            cells = [case(r),r['candidate'],f'{statistics.median(r[phase])*1000:.3f}',
                     f'{statistics.median(r["warm_s"])*1000:.3f}' if 'warm_s' in r else '—',
                     f'{memory/1000:.1f}',str(len(r[phase]))]
            lines.append('| '+' | '.join(cells)+' |')
        lines.append('')
    lines += ['## Evidence','',f'{len(rows)} cases, {len(pairs)} layout pairs, {len(processes)} serial processes.','']
    lines += [f'- [Raw samples and process logs](results/{name}).' for name in args.inputs]
    lines.append('')
    (LAB/args.output).write_text('\n'.join(lines))
    print(f'Wrote {args.output}: {len(rows)} cases; {len(pairs)} paired normal-form checks.')

if __name__ == '__main__': main()
