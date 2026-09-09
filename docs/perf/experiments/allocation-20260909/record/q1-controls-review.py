#!/usr/bin/env python3
"""Check every broader owner/counter window, including adverse growth costs."""
import argparse
import ast
import re
import shutil
from diagnostics import Phase, ROUND, digest, load

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('baseline',type=int)
parser.add_argument('candidate',type=int)
parser.add_argument('attempt',type=int)
args=parser.parse_args()
paths={key:ROUND/f'q1-controls-{key}-{attempt}' for key,attempt in [('baseline',args.baseline),('candidate',args.candidate)]}
states={key:load(path/'STATE.json') for key,path in paths.items()}
assert all(state['status']=='BROADER-CONTROLS-COMPLETE-REVIEW-REQUIRED' for state in states.values())
assert states['baseline']['dependencies']==states['candidate']['dependencies']
assert states['baseline']['input_hashes']==states['candidate']['input_hashes']
assert states['baseline']['binary_sha256']!=states['candidate']['binary_sha256']
assert all(not state['engine_artifact']['fresh'] for state in states.values())
phase=Phase(f'q1-controls-review-{args.attempt}');shutil.copy2(__file__,phase.path/'review.py')
def counters(raw):
    rows=re.findall(r'ALLOC (\S+) AllocWindow \{ ([^}]+) \}',raw)
    result={label:tuple(map(int,re.findall(r': (\d+)',data))) for label,data in rows}
    assert len(result)==len(rows)==328 and all(len(c)==4 for c in result.values())
    return result
def maps(raw):
    rows=re.findall(r'^MAP (\S+) role=(\S+) rows=(\d+) arity=(\d+) owners=(\[.*?\]) bytes=(\d+)$',raw,re.M)
    result={}
    for label,role,count,arity,owners,size in rows:
        owners=ast.literal_eval(owners);size=int(size)
        assert len(owners)==5 and all(l<=cap for l,cap,_ in owners)
        assert sum(cap*width for _,cap,width in owners)==size
        result[(label,role)]=(int(count),int(arity),owners,size)
    assert len(result)==len(rows)
    return result
try:
    raw={key:(path/'controls.log').read_text() for key,path in paths.items()}
    alloc={key:counters(text) for key,text in raw.items()}
    table={key:maps(text) for key,text in raw.items()}
    assert alloc['baseline'].keys()==alloc['candidate'].keys()
    assert table['baseline'].keys()==table['candidate'].keys()
    unchanged={}
    for prefix in ['ANSWER','PROJECTION','SET','AGGREGATE','DENSE','FOLDS','AGGOWNERS','PASS']:
        rows={key:re.findall(r'^'+prefix+r' .*$',text,re.M) for key,text in raw.items()}
        assert rows['baseline']==rows['candidate'],prefix
        unchanged[prefix]=len(rows['baseline'])
    retained={key:{} for key in table}
    for key,tables in table.items():
        for (label,role),(count,arity,owners,size) in tables.items():
            window=label.split('/')[0]
            retained[key][window]=retained[key].get(window,0)+size
            if key=='candidate':
                old=table['baseline'][(label,role)]
                assert (count,arity)==old[:2]
                assert [w for _,_,w in owners]==[w for _,_,w in old[2]]
                if ':prepare/' in label:assert size==0
                if ':release/' in label:assert size==0
    cases={label.rsplit(':',1)[0] for label in alloc['baseline']}
    assert len(cases)==41
    windows={};summary={}
    for case in sorted(cases):
        total_delta=[0]*4;metadata=None
        for kind in ('prepare','cold','warm','small','return','release','refill','warm_refill'):
            label=f'{case}:{kind}'
            a,b=alloc['baseline'][label],alloc['candidate'][label]
            change=tuple(y-x for x,y in zip(a,b))
            total_delta=[x+y for x,y in zip(total_delta,change)]
            map_a=retained['baseline'].get(label,0);map_b=retained['candidate'].get(label,0)
            residual=total_delta[2]-total_delta[3]-(map_b-map_a)
            if kind=='prepare':metadata=residual
            # Every difference in retained requested-minus-freed bytes is
            # explained by observed map owners plus unchanged plan metadata.
            assert residual==metadata,(label,residual,metadata)
            if kind in ('warm','small','return','refill','warm_refill'):
                assert change==(0,0,0,0),(label,change)
            windows[label]=dict(baseline=a,candidate=b,delta=change,map_baseline=map_a,map_candidate=map_b)
        prepare=windows[case+':prepare'];cold=windows[case+':cold']
        joint=tuple(a+b for a,b in zip(prepare['delta'],cold['delta']))
        summary[case]=dict(prepare_delta=prepare['delta'],cold_delta=cold['delta'],combined_delta=joint,
            retained_map_baseline=cold['map_baseline'],retained_map_candidate=cold['map_candidate'],metadata_bytes_delta=metadata)
        print(f'{case} prepare_delta={prepare["delta"]} cold_delta={cold["delta"]} combined_delta={joint} retained_maps={cold["map_baseline"]}->{cold["map_candidate"]} metadata_delta={metadata}')
    phase.state.update(baseline=str(paths['baseline']),candidate=str(paths['candidate']),
        source_fingerprint=states['candidate']['ordinary_source'],dependencies=states['candidate']['dependencies'],
        log_hashes={key:digest(path/'controls.log') for key,path in paths.items()},
        map_records=len(table['baseline']),unchanged_records=unchanged,windows=windows,summary=summary,
        limitations='Requested layouts and retained sink/map owners, not RSS/peak/speed. All adverse growth costs preserved. No timing acceptance.')
except BaseException:
    phase.finish('INCOMPLETE');raise
else:phase.finish('ALL-BROADER-OWNERS-AND-GROWTH-TRADEOFFS-VERIFIED')
