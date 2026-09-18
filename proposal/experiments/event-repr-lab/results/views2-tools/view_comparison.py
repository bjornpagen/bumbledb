"""Keep full relation programs and per-binding products as different outputs."""
from pathlib import Path
import argparse,hashlib,json,statistics as st
from prepare import LAB

KEYS=['kind','bits','layout','memo','candidate','essential_layout','product_mode']
def collect(inputs):
    rows,processes,builds={},[],set()
    for name in inputs:
        data=json.loads((LAB/'results'/name).read_text());builds.add(data['build_metadata']['binary_sha256'])
        for run in data['runs']:
            processes.append(dict(file=name,job=run['job'],status=run['status'],log=run['log']))
            if run['status']!='passed':
                job=run['job']
                kind={'relations':'relations_free_join','products':'product_free_join'}.get(job.get('EVENT_LAB_LANE'))
                # A later failed attempt invalidates older timings for its job;
                # it cannot disappear behind a previous successful process.
                for key in list(rows):
                    if (key[0]==kind and key[2]==job.get('EVENT_LAB_LAYOUT')
                        and key[4]==job.get('EVENT_LAB_CANDIDATE')
                        and key[5]==job.get('EVENT_LAB_ESSENTIAL_LAYOUT')
                        and key[6]==job.get('EVENT_LAB_PRODUCT')):
                        del rows[key]
                continue
            for row in run['rows']:
                if row.get('kind') not in ['relations_free_join','product_free_join']:continue
                assert row['verified']
                for field,env in [('layout','EVENT_LAB_LAYOUT'),('product_mode','EVENT_LAB_PRODUCT'),('essential_layout','EVENT_LAB_ESSENTIAL_LAYOUT')]:
                    assert row[field]==run['job'][env],(name,field)
                assert row['essential_kernel']=='derived'
                assert row['bits'] in [12,18] and row['worlds']==1<<row['bits']
                assert row['groups']==16 and row['rows']==128
                assert len(row['fresh_s'])==data['trials'] and len(row['warm_s'])==data['trials']
                for field in ['input_storage','final_storage']:
                    storage=row[field]
                    if storage is not None:
                        assert storage['layout']==row['essential_layout']
                        assert storage['total_bytes_est']==sum(storage[k] for k in
                            ['record_bytes','table_bytes','interner_bytes','metadata_bytes','cache_bytes'])
                rows[tuple(row[k] for k in KEYS)]=(row,name)
    assert len(builds)==1,'Do not merge different executables.'
    answers={}
    for row,_ in rows.values():
        key=(row['kind'],row['bits'])
        answer=(row['checksum'],row['rows'],row['groups'],row.get('closure_iterations'))
        assert answers.setdefault(key,answer)==answer,(key,answer)
    pairs=[];missing=[]
    for key,(view,source) in rows.items():
        if view['product_mode']!='views':continue
        baseline=rows.get(key[:-1]+('materialized',))
        if baseline is None:missing.append(list(key));continue
        materialized,other=baseline
        assert view['input_nodes']==materialized['input_nodes']
        assert view['input_storage']==materialized['input_storage']
        pairs.append(dict(case=dict(zip(KEYS,key)),view_source=source,materialized_source=other,
            phases={p:dict(materialized_median_s=st.median(materialized[p]),views_median_s=st.median(view[p]),
                materialized_over_views=st.median(materialized[p])/st.median(view[p]),
                materialized_min_s=min(materialized[p]),materialized_max_s=max(materialized[p]),
                views_min_s=min(view[p]),views_max_s=max(view[p]),
                materialized_samples=len(materialized[p]),views_samples=len(view[p])) for p in ['fresh_s','warm_s']},
            materialized_final_nodes=materialized['final_nodes'],views_final_nodes=view['final_nodes'],
            materialized_final_bytes_est=materialized['final_bytes_est'],views_final_bytes_est=view['final_bytes_est']))
    return rows,processes,next(iter(builds)),pairs,missing

def main():
    ap=argparse.ArgumentParser();ap.add_argument('inputs',nargs='+')
    ap.add_argument('--output',default='VIEW-MEASUREMENTS.md');ap.add_argument('--record',default='view-comparison.json')
    args=ap.parse_args();rows,processes,binary,pairs,missing=collect(args.inputs)
    result=dict(checked=True,inputs=args.inputs,binary_sha256=binary,cases=len(rows),matched_cases=len(pairs),
        processes=processes,pairs=pairs,missing_baselines=missing,
        checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        selection='Latest supplied process per exact case; a later failed job invalidates earlier timings. Resource refusals/timeouts are retained; never mathematical answers.')
    (LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
    lines=['# Mapped products through Free Join','',
        'One executable; latest supplied process per exact case. Times are',
        'milliseconds; KB means 1,000 bytes. Full relation programs and per-binding',
        'products have different output contracts and remain separate. Query time',
        'includes native join, canonical result construction and exact count readout.',
        'Construction, cloning, native setup and full bitset verification are outside.',
        'Resident byte estimates include arenas and outer operation caches but omit',
        'temporary traversal storage and allocator overhead. Failed/resource-limited',
        'processes do not supply timings. See [the kernel contract](VIEW-PRODUCT.md).','',
        f'Executable SHA-256: `{binary}`.','']
    groups=sorted({tuple(r[k] for k in KEYS[:4]) for r,_ in rows.values()})
    for kind,bits,layout,memo in groups:
        selected=[r for r,_ in rows.values() if tuple(r[k] for k in KEYS[:4])==(kind,bits,layout,memo)]
        lines += [f'## {kind}, {bits} coordinates, {layout}, memo {"on" if memo else "off"}','',
            '| Carrier/store | Product | Join only | Fresh | Fresh min–max | Warm | Input KB | Final KB | Final nodes | Samples |',
            '| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |']
        for r in sorted(selected,key=lambda x:(x['candidate'],x['essential_layout'],x['product_mode'])):
            label=r['candidate']+('/'+r['essential_layout'] if r['candidate'].startswith('essential') else '')
            values=[label,r['product_mode'],f'{1000*st.median(r["join_only_s"]):.3f}',
                f'{1000*st.median(r["fresh_s"]):.3f}',f'{1000*min(r["fresh_s"]):.3f}–{1000*max(r["fresh_s"]):.3f}',
                f'{1000*st.median(r["warm_s"]):.3f}',f'{r["input_bytes_est"]/1000:.1f}',
                f'{r["final_bytes_est"]/1000:.1f}',str(r['final_nodes']),str(len(r['fresh_s']))]
            lines.append('| '+' | '.join(values)+' |')
        lines.append('')
    lines += ['## Evidence','',f'{len(rows)} distinct cases, {len(pairs)} matched strategy pairs, {len(processes)} retained processes.','']
    for p in processes:
        if p['status']!='passed':lines.append(f'- {p["status"]}: [{p["log"]}](results/{p["log"]}).')
    lines += [f'- [Raw evidence](results/{n}).' for n in args.inputs]
    (LAB/args.output).write_text('\n'.join(lines)+'\n')
    print(f'Wrote {args.output}: {len(rows)} cases, {len(pairs)} matched pairs, {len(missing)} missing baselines.')
if __name__=='__main__':main()
