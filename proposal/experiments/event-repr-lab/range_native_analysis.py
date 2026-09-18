"""Matched native comparisons; never compare these times with raw replay counts."""
from pathlib import Path
import argparse,hashlib,itertools,json,statistics
from prepare import LAB

ap=argparse.ArgumentParser();ap.add_argument('--input',required=True);ap.add_argument('--output',required=True);args=ap.parse_args()
data=json.loads((LAB/'results'/args.input).read_text());assert data['phase']=='join' and data['passed']
assert len(data['runs'])==14
candidates=['range-shannon','range-step','range-group','packed512','dense','essential512','retraction512']
cases=list(itertools.product(['coup_4290','coup_product_65536'],['triangle','clover'],[False,True]))
rows={}
for run in data['runs']:
    assert run['status']=='passed' and len(run['rows'])==4
    for row in run['rows']:
        assert row['kind']=='free_join' and row['verified']
        key=(row['candidate'],row['scenario'],row['shape'],row['memo']);assert key not in rows
        for field in ['fresh_s','warm_s','build_s','join_only_s']:
            assert len(row[field])==data['trials'] and all(t>=0 for t in row[field])
        rows[key]=row
assert set(rows)=={(c,*case) for c,case in itertools.product(candidates,cases)}
metrics={}
for key,r in rows.items():
    metrics[key]={**{field.replace('_s','_ms'):statistics.median(r[field])*1000 for field in ['fresh_s','warm_s','build_s','join_only_s']},
        **{field:r[field] for field in ['input_nodes','final_nodes','input_bytes_est','final_bytes_est','rows','checksum']}}
comparisons=[]
for case in cases:
    reference=rows[('range-shannon',*case)]
    for c in candidates:
        row=rows[(c,*case)]
        for k in ['worlds','admissible_worlds','rows','groups','checksum']:assert row[k]==reference[k],(case,c,k)
    for baseline in ['range-shannon','range-step','packed512','dense','essential512','retraction512']:
        group=metrics[('range-group',*case)];control=metrics[(baseline,*case)]
        comparisons.append(dict(scenario=case[0],shape=case[1],memo=case[2],baseline=baseline,
            group=group,control=control,fresh_ratio=group['fresh_ms']/control['fresh_ms'],warm_ratio=group['warm_ms']/control['warm_ms']))
out=dict(passed=True,cases=len(cases),configurations=len(rows),processes=len(data['runs']),trials=data['trials'],
    binary_sha256=data['build_metadata']['binary_sha256'],input=args.input,input_sha256=hashlib.sha256((LAB/'results'/args.input).read_bytes()).hexdigest(),
    checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),comparisons=comparisons,
    boundary='Ratios are grouped/control medians in the same native binary. Includes exact grouped-output counting; no universal workload ranking.')
(LAB/'results'/args.output).write_text(json.dumps(out,indent=2)+'\n')
for c in comparisons:
    if c['shape']=='clover' and c['memo']:
        print(c['scenario'],c['baseline'],'group/control ms',round(c['group']['fresh_ms'],3),round(c['control']['fresh_ms'],3),'ratio',round(c['fresh_ratio'],3))
