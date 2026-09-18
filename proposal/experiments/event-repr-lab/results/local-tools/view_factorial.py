"""Separate local plane construction from the choice to fuse output renaming."""
from pathlib import Path
import argparse,hashlib,json,statistics as st
from prepare import LAB

KEYS=['kind','bits','layout','memo','candidate','essential_layout','product_mode','view_kernel']
KINDS={'relations':'relations_free_join','products':'product_free_join'}
def collect(inputs):
    rows,processes,builds={},[],set()
    for name in inputs:
        data=json.loads((LAB/'results'/name).read_text());builds.add(data['build_metadata']['binary_sha256'])
        for run in data['runs']:
            job=run['job'];processes.append(dict(file=name,job=job,status=run['status'],log=run['log']))
            if run['status']!='passed':
                for key in list(rows):
                    if (key[0]==KINDS.get(job.get('EVENT_LAB_LANE')) and key[2]==job.get('EVENT_LAB_LAYOUT')
                        and key[4]==job.get('EVENT_LAB_CANDIDATE') and key[5]==job.get('EVENT_LAB_ESSENTIAL_LAYOUT')
                        and key[6]==job.get('EVENT_LAB_PRODUCT') and key[7]==job.get('EVENT_LAB_VIEW_KERNEL')):
                        del rows[key]
                continue
            for row in run['rows']:
                if row.get('kind') not in KINDS.values():continue
                assert row['verified'] and row['essential_kernel']=='derived'
                for field,env in [('layout','EVENT_LAB_LAYOUT'),('product_mode','EVENT_LAB_PRODUCT'),
                                  ('essential_layout','EVENT_LAB_ESSENTIAL_LAYOUT'),('view_kernel','EVENT_LAB_VIEW_KERNEL')]:
                    assert row[field]==job[env],(name,field)
                assert row['groups']==16 and row['rows']==128
                assert row['bits'] in [12,18] and row['worlds']==1<<row['bits']
                assert len(row['fresh_s'])==len(row['warm_s'])==data['trials']
                for field in ['input_storage','final_storage']:
                    s=row[field]
                    if s is not None:
                        assert s['layout']==row['essential_layout']
                        assert s['total_bytes_est']==sum(s[k] for k in ['record_bytes','table_bytes','interner_bytes','metadata_bytes','cache_bytes'])
                rows[tuple(row[k] for k in KEYS)]=(row,name)
    assert len(builds)==1,'Only compare strategies in the same executable'
    answers={}
    for row,_ in rows.values():
        key=(row['kind'],row['bits']);answer=(row['checksum'],row['rows'],row['groups'],row.get('closure_iterations'))
        assert answers.setdefault(key,answer)==answer
    pairs=[];missing=[]
    for key,(row,source) in rows.items():
        if row['product_mode']=='materialized':continue
        comparisons=[('materialized',key[:6]+('materialized','assignments'))]
        if row['view_kernel']=='words':comparisons.append(('local-kernel',key[:7]+('assignments',)))
        if row['product_mode']=='views-inputs':comparisons.append(('output-order',key[:6]+('views',row['view_kernel'])))
        for kind,basekey in comparisons:
            if basekey not in rows:missing.append(dict(comparison=kind,case=list(key)));continue
            base,other=rows[basekey]
            assert row['input_nodes']==base['input_nodes'] and row['input_storage']==base['input_storage']
            if kind=='local-kernel':
                for field in ['final_nodes','final_storage','final_bytes_est']:
                    assert row[field]==base[field],(key,field)
            pairs.append(dict(comparison=kind,case=dict(zip(KEYS,key)),source=source,baseline_source=other,
                baseline_strategy=base['product_mode']+'/'+base['view_kernel'],
                phases={p:dict(baseline_median_s=st.median(base[p]),candidate_median_s=st.median(row[p]),
                    baseline_over_candidate=st.median(base[p])/st.median(row[p]),
                    baseline_min_s=min(base[p]),baseline_max_s=max(base[p]),
                    candidate_min_s=min(row[p]),candidate_max_s=max(row[p]),
                    baseline_samples=len(base[p]),candidate_samples=len(row[p])) for p in ['fresh_s','warm_s']},
                baseline_final_nodes=base['final_nodes'],candidate_final_nodes=row['final_nodes'],
                baseline_final_bytes_est=base['final_bytes_est'],candidate_final_bytes_est=row['final_bytes_est']))
    return rows,processes,next(iter(builds)),pairs,missing

def main():
    ap=argparse.ArgumentParser();ap.add_argument('inputs',nargs='+')
    ap.add_argument('--output',default='VIEW-FACTORIAL.md');ap.add_argument('--record',default='view-factorial.json')
    args=ap.parse_args();rows,processes,binary,pairs,missing=collect(args.inputs)
    result=dict(passed=True,inputs=args.inputs,binary_sha256=binary,cases=len(rows),matched_cases=len(pairs),
        processes=processes,pairs=pairs,missing_baselines=missing,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        selection='Latest supplied process per exact case; later failures invalidate earlier timings. Refusals and timeouts never count as answers.')
    (LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
    lines=['# Mapped product factorial experiment','',
        'Same executable and answers. Each process uses one local readout kernel',
        'and one product strategy. `views` fuses output renaming; `views-inputs`',
        'renames the canonical output after projection in the working coordinate order.',
        'Local word/assignment pairs must retain exactly identical nodes, cache',
        'and arena byte estimates. Resident bytes omit transient workspace and allocator overhead.',
        'Times are milliseconds; KB means 1,000 bytes. Query includes actual Free Join,',
        'canonical construction, and exact count. Fixture construction, cloning, setup',
        'and full bitset verification are outside. These two query kinds return different outputs.',
        f'Executable: `{binary}`.','']
    for group in sorted({key[:4] for key in rows}):
        kind,bits,layout,memo=group
        lines += [f'## {kind}, {bits} coordinates, {layout}, memo {memo}','',
            '| Carrier/store | Strategy | Local reader | Fresh | Min–max | Warm | Final KB | Nodes | Samples |',
            '| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |']
        for key,(r,_) in sorted(rows.items()):
            if key[:4]!=group:continue
            values=[r['candidate']+'/'+r['essential_layout'],r['product_mode'],r['view_kernel'],
                f'{1000*st.median(r["fresh_s"]):.3f}',f'{1000*min(r["fresh_s"]):.3f}–{1000*max(r["fresh_s"]):.3f}',
                f'{1000*st.median(r["warm_s"]):.3f}',f'{r["final_bytes_est"]/1000:.1f}',str(r['final_nodes']),str(len(r['fresh_s']))]
            lines.append('| '+' | '.join(values)+' |')
        lines.append('')
    lines += [f'{len(rows)} cases; {len(pairs)} matched comparisons; {len(processes)} retained processes.','']
    for p in processes:
        if p['status']!='passed':lines.append(f'- {p["status"]}: [{p["log"]}](results/{p["log"]}).')
    lines += [f'- [Raw evidence](results/{n}).' for n in args.inputs]
    (LAB/args.output).write_text('\n'.join(lines)+'\n')
    print(f'{len(rows)} cases; {len(pairs)} matched comparisons; {len(missing)} missing baselines.')
if __name__=='__main__':main()
