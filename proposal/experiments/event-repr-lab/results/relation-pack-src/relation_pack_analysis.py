"""Match relational Pack schedules while retaining carrier, role and layout."""
from pathlib import Path
import argparse,hashlib,itertools,json,math,statistics
from prepare import LAB

CANDIDATES=['dense','packed512','essential512','retraction512','range-shannon','range-group']
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def analyze(path):
    data=json.loads(path.read_text());assert data['passed'] and data['phase']=='comparison' and data['trials']>=7
    rows={}
    for run in data['runs']:
        assert run['status']=='passed' and len(run['rows'])==8
        for r in run['rows']:
            assert r['kind']=='relation_pack_free_join' and r['verified']
            for field,env in [('candidate','EVENT_LAB_CANDIDATE'),('width','EVENT_PACK_WIDTH'),('fanout','EVENT_PACK_FANOUT'),('layout','EVENT_LAB_LAYOUT')]:
                assert r[field]==run['job'][env]
            for field in ['fresh_s','warm_s']:
                assert len(r[field])==data['trials'] and all(math.isfinite(v) and v>0 for v in r[field])
            key=tuple(r[k] for k in ['candidate','width','layout','fanout','program','memo','schedule']);assert key not in rows;rows[key]=r
    candidates=data.get('candidates',CANDIDATES);widths=data.get('widths',[4,5]);layouts=data.get('layouts',['face-major','bit-major']);fanouts=data.get('fanouts',[2,8])
    assert 'dense' in candidates and 'face-major' in layouts
    cases=list(itertools.product(candidates,widths,layouts,fanouts,['compose','residual'],[False,True]))
    assert set(rows)=={(*case,s) for case in cases for s in ['complete','factorized']}
    pairs=[]
    for case in cases:
        a,b=(rows[(*case,s)] for s in ['complete','factorized']);ref=rows[('dense',case[1],'face-major',*case[3:],'complete')]
        for r in [a,b]:
            for field in ['checksum','worlds','admissible_worlds','environments','groups','present_groups','represented_bindings']:
                assert r[field]==ref[field],(case,field)
            assert r['groups']==r['present_groups']==16 and r['environments']==2
            assert r['represented_bindings']==16*r['fanout']**2
        f=a['fanout'];assert a['native_emissions']==16*f*f and a['branch_emissions']==a['summary_rows']==a['staged_capacity_bytes']==a['summary_colt_bytes']==0
        assert b['native_emissions']==32*f+16 and b['branch_emissions']==32*f and b['summary_rows']==32
        assert b['staged_capacity_bytes']>=32*f*8
        pair=dict(zip(['candidate','width','layout','fanout','program','memo'],case))
        pair.update(checksum=a['checksum'],represented_bindings=a['represented_bindings'],complete_emissions=a['native_emissions'],factorized_emissions=b['native_emissions'])
        for side,r in [('complete',a),('factorized',b)]:
            pair[side]={field+'_ms':statistics.median(r[field+'_s'])*1000 for field in ['fresh','warm']}
            pair[side].update({field+'_range_ms':[min(r[field+'_s'])*1000,max(r[field+'_s'])*1000] for field in ['fresh','warm']})
            pair[side].update({k:r[k] for k in ['final_nodes','final_bytes_est','retained_input_colt_bytes','staged_capacity_bytes','summary_colt_bytes']})
        for field in ['fresh','warm']:pair[field+'_speedup']=pair['complete'][field+'_ms']/pair['factorized'][field+'_ms']
        pairs.append(pair)
    return dict(passed=True,input=path.name,input_sha256=digest(path),binary_sha256=data['build_metadata']['binary_sha256'],checker_sha256=digest(Path(__file__)),candidates=candidates,widths=widths,layouts=layouts,fanouts=fanouts,
        trials=data['trials'],processes=len(data['runs']),configurations=len(rows),comparisons=pairs,
        boundary='Same typed program, legal environments, layout, carrier and published input roots. Includes native summary planning and role admission. Composition uses union output; residual uses intersection output. No direct absolute comparison with earlier carrier-only or Boolean Pack rounds.')

def markdown(r):
    lines=['# Native relational Pack measurements','',f"{r['trials']}-sample medians in milliseconds. Speedup = complete / factorized.",
        'Every pair holds the carrier, legal support, environment, layout and typed program fixed.','',
        '| Carrier | Width | Layout | Fanout | Program | Memo | Fresh complete | Fresh factored | Speedup | Warm complete | Warm factored | Speedup |',
        '| --- | ---: | --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |']
    for p in r['comparisons']:
        a,b=p['complete'],p['factorized'];lines.append(f"| {p['candidate']} | {p['width']} | {p['layout']} | {p['fanout']} | {p['program']} | {p['memo']} | {a['fresh_ms']:.4f} | {b['fresh_ms']:.4f} | {p['fresh_speedup']:.2f}× | {a['warm_ms']:.4f} | {b['warm_ms']:.4f} | {p['warm_speedup']:.2f}× |")
    lines+=['','## Measurement boundary','',
        f"Carriers: {', '.join(r['candidates'])}. Widths: {r['widths']}; layouts: {', '.join(r['layouts'])}; fanouts: {r['fanouts']}.",
        'two legal state domains selected by one shared environment; sixteen groups.',
        'This is a bounded structured transition/obligation fixture, not a full Coup simulator.','',
        'The complete path evaluates the typed operation for every matching row pair.',
        'The factored path reduces each branch and joins summary rows. Both use actual',
        'Free Join and validate all participating values. Role checks reject Events that',
        'depend on the scratch face. Group presence is separate from relation nonemptiness.','',
        'Query timing includes validation, role admission, reduction, root staging, summary',
        'planning/image/COLT construction, final joining and exact output counts. Input',
        'setup and fresh arena cloning are separate; input COLTs are primed. Warm queries',
        'reuse learned roots/memos but reconstruct summaries. Every fresh trial and the',
        'last warm result are checked pointwise against an independent matrix oracle.','',
        'Retained Event/memo and COLT estimates are separate. Staged capacity sums root',
        'and resolved-role vectors across groups, including short-lived vectors; it is',
        'not peak memory. Maps, images and planner allocations are additional.','',
        f"[Raw samples](results/{r['input']}) · [Matched values and ranges](results/{r['record']})",
        '· [Laws, query certificate and implementation](RELATIONAL-PACK.md)','']
    return '\n'.join(lines)
if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('input');ap.add_argument('--output',required=True);ap.add_argument('--record',required=True);a=ap.parse_args()
    r=analyze(LAB/'results'/a.input);r['record']=a.record;(LAB/'results'/a.record).write_text(json.dumps(r,indent=2)+'\n');(LAB/a.output).write_text(markdown(r))
    print(json.dumps({k:v for k,v in r.items() if k!='comparisons'},indent=2))
