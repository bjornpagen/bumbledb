"""Recompute matched scoped contractions, preserving answer and timing scope."""
from pathlib import Path
import argparse,hashlib,json,statistics as st
from prepare import LAB

KEYS=['kind','support','bits','layout','memo','candidate','essential_layout','product_mode','view_kernel','view_normalize','view_reuse']
FIELDS=[('support','EVENT_LAB_SCOPED_SUPPORT'),('candidate','EVENT_LAB_CANDIDATE'),('layout','EVENT_LAB_LAYOUT'),('essential_layout','EVENT_LAB_ESSENTIAL_LAYOUT'),('product_mode','EVENT_LAB_PRODUCT'),('view_kernel','EVENT_LAB_VIEW_KERNEL'),('view_normalize','EVENT_LAB_VIEW_NORMALIZE'),('view_reuse','EVENT_LAB_VIEW_REUSE')]
KIND='scoped_product_free_join'
def collect(inputs):
 rows={};processes=[];builds=set()
 for name in inputs:
  data=json.loads((LAB/'results'/name).read_text());builds.add(data['build_metadata']['binary_sha256'])
  for run in data['runs']:
   job=run['job'];processes.append(dict(file=name,job=job,status=run['status'],log=run['log']))
   if run['status']!='passed':
    if job.get('EVENT_LAB_LANE')=='scoped-products':
     for key in list(rows):
      case=dict(zip(KEYS,key))
      if all(case[f]==job.get(e) for f,e in FIELDS):del rows[key]
    continue
   for r in run['rows']:
    if r.get('kind')!=KIND:continue
    assert r['verified'] and r['essential_kernel']=='derived'
    for f,e in FIELDS:assert r[f]==job[e],(name,f)
    assert r['rows']==128 and r['groups']==16 and r['bits'] in [12,18]
    assert r['worlds']==1<<r['bits'] and r['checksum']>0
    n=1<<(r['bits']//3)
    population={'full':n**3,'legal-product':(n-3)**3,'copied-face':n*(n-3),'asymmetric':(n-3)*(n-2)**2}[r['support']]
    assert r['admissible_worlds']==population
    assert len(r['fresh_s'])==len(r['warm_s'])==data['trials']
    rows[tuple(r[k] for k in KEYS)]=(r,name)
 assert len(builds)==1,'Do not compare across executables'
 answers={}
 for r,_ in rows.values():
  key=(r['support'],r['bits']);answer=(r['checksum'],r['rows'],r['groups'],r['admissible_worlds'])
  assert answers.setdefault(key,answer)==answer
 pairs=[];missing=[]
 for key,(r,source) in rows.items():
  if r['product_mode']=='materialized':continue
  choices=[('materialized',dict(product_mode='materialized',view_normalize='deferred',view_reuse='off'))]
  if r['view_reuse']=='bounded':choices.append(('reuse',dict(view_reuse='off')))
  if r['view_normalize']=='source':choices.append(('normalization',dict(view_normalize='deferred')))
  if r['product_mode']=='views-inputs':choices.append(('output-order',dict(product_mode='views')))
  for kind,changes in choices:
   base_case=dict(zip(KEYS,key));base_case.update(changes);bk=tuple(base_case[k] for k in KEYS)
   if bk not in rows:missing.append(dict(comparison=kind,case=list(key)));continue
   base,other=rows[bk]
   assert r['input_nodes']==base['input_nodes'] and r['input_storage']==base['input_storage']
   if kind=='reuse':
    for field in ['final_nodes','final_storage','final_bytes_est']:assert r[field]==base[field],(key,field)
   pairs.append(dict(comparison=kind,case=dict(zip(KEYS,key)),baseline_case=base_case,source=source,baseline_source=other,
    phases={p:dict(baseline_median_s=st.median(base[p]),candidate_median_s=st.median(r[p]),baseline_over_candidate=st.median(base[p])/st.median(r[p]),baseline_min_s=min(base[p]),baseline_max_s=max(base[p]),candidate_min_s=min(r[p]),candidate_max_s=max(r[p]),baseline_samples=len(base[p]),candidate_samples=len(r[p])) for p in ['fresh_s','warm_s']},
    baseline_final_nodes=base['final_nodes'],candidate_final_nodes=r['final_nodes'],baseline_final_bytes_est=base['final_bytes_est'],candidate_final_bytes_est=r['final_bytes_est']))
 return rows,processes,next(iter(builds)),pairs,missing

def main():
 ap=argparse.ArgumentParser();ap.add_argument('inputs',nargs='+');ap.add_argument('--output',default='SCOPED-MEASUREMENTS.md');ap.add_argument('--record',default='scoped-comparison.json');args=ap.parse_args()
 rows,processes,binary,pairs,missing=collect(args.inputs)
 result=dict(passed=True,inputs=args.inputs,binary_sha256=binary,cases=len(rows),matched_cases=len(pairs),pairs=pairs,processes=processes,missing_baselines=missing,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),selection='Latest supplied process per exact case; later failure invalidates earlier samples. Resource limits are not answers.')
 (LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
 lines=['# Scoped contraction through Free Join','',
 'Exact staged clipped maps, conjunction and elimination; 16 grouped Event outputs.',
 'Left input rotates bits within each face, right input cycles faces, Y is hidden,',
 'then output Y/Z are swapped. Coupled supports are not called ordinary relation',
 'composition. Every fresh and warm output is checked pointwise outside timing.',
 'Times are milliseconds; resident KB includes outer memos and retained maps,',
 'excluding temporary plane caches and allocator overhead. See [the contract](SCOPED-PRODUCT.md).',
 f'Executable: `{binary}`.','']
 for group in sorted({k[1:5] for k in rows}):
  support,bits,layout,memo=group
  lines += [f'## {support}, {bits} coordinates, {layout}, memo {memo}','',
   '| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |',
   '| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |']
  for key,(r,_) in sorted(rows.items()):
   if key[1:5]!=group:continue
   values=[r['candidate']+'/'+r['essential_layout'],r['product_mode'],r['view_normalize'],r['view_reuse'],f'{1000*st.median(r["fresh_s"]):.3f}',f'{1000*min(r["fresh_s"]):.3f}–{1000*max(r["fresh_s"]):.3f}',f'{1000*st.median(r["warm_s"]):.3f}',f'{r["final_bytes_est"]/1000:.1f}',str(r['final_nodes']),str(len(r['fresh_s']))]
   lines.append('| '+' | '.join(values)+' |')
  lines.append('')
 lines += [f'{len(rows)} cases; {len(pairs)} matched comparisons; {len(processes)} retained processes.','']
 lines += [f'- {r["status"]}: [{r["log"]}](results/{r["log"]}).' for r in processes if r['status']!='passed']
 lines += [f'- [Raw evidence](results/{n}).' for n in args.inputs]
 (LAB/args.output).write_text('\n'.join(lines)+'\n')
 print(f'{len(rows)} cases; {len(pairs)} matched comparisons; {len(missing)} missing baselines.')
if __name__=='__main__':main()
