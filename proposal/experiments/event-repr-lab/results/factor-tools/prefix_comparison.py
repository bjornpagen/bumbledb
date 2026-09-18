"""Compare completion choices using only matched rows from one native executable."""
from pathlib import Path
import argparse,hashlib,json,statistics as st
from prepare import LAB
KEYS=['kind','domain','bits','layout','memo','candidate','essential_layout','product_mode','view_kernel','view_normalize','view_reuse','support_gates','retraction_count']
FIELDS=[('domain','EVENT_LAB_LEGAL_DOMAIN'),('candidate','EVENT_LAB_CANDIDATE'),('layout','EVENT_LAB_LAYOUT'),('essential_layout','EVENT_LAB_ESSENTIAL_LAYOUT'),('product_mode','EVENT_LAB_PRODUCT'),('view_kernel','EVENT_LAB_VIEW_KERNEL'),('view_normalize','EVENT_LAB_VIEW_NORMALIZE'),('view_reuse','EVENT_LAB_VIEW_REUSE'),('retraction_count','EVENT_LAB_RETRACTION_COUNT')]
KIND='legal_relation_free_join'
def collect(inputs):
 rows={};processes=[];builds=set()
 for name in inputs:
  data=json.loads((LAB/'results'/name).read_text());builds.add(data['build_metadata']['binary_sha256'])
  for run in data['runs']:
   job=run['job'];processes.append(dict(file=name,job=job,status=run['status'],log=run['log']))
   if run['status']!='passed':
    if job.get('EVENT_LAB_LANE')=='legal-relations':
     for key in list(rows):
      case=dict(zip(KEYS,key))
      gates=job.get('EVENT_LAB_SUPPORT_GATES') if job.get('EVENT_LAB_PRODUCT')!='materialized' else 'materialized'
      if case['support_gates']==gates and all(case[f]==job.get(e) for f,e in FIELDS):del rows[key]
    continue
   for r in run['rows']:
    if r.get('kind')!=KIND:continue
    assert r['verified'] and r['essential_kernel']=='derived'
    for f,e in FIELDS:assert r[f]==job[e],(name,f)
    assert r['rows']==128 and r['groups']==16 and r['outputs']==80 and r['width'] in [4,6]
    gates=job['EVENT_LAB_SUPPORT_GATES'] if r['product_mode']!='materialized' else 'materialized'
    assert r['support_gates']==gates
    assert r['bits']==3*r['width']+(r['domain']=='fibred')
    assert r['worlds']==1<<r['bits'] and r['checksum']>0
    n=1<<r['width'];holes=sum((v+1)%3!=0 for v in range(n-1))
    population={'full':n**3,'below':(n-3)**3,'holes':holes**3,'fibred':(n-3)**3+holes**3}[r['domain']]
    assert r['environments']==(2 if r['domain']=='fibred' else 1)
    assert r['admissible_worlds']==population
    assert len(r['fresh_s'])==len(r['warm_s'])==data['trials']
    rows[tuple(r[k] for k in KEYS)]=(r,name)
 assert len(builds)==1,'Do not compare across executables'
 answers={}
 for r,_ in rows.values():
  key=(r['domain'],r['bits']);answer=(r['checksum'],r['rows'],r['groups'],r['admissible_worlds'])
  assert answers.setdefault(key,answer)==answer
 pairs=[];missing=[]
 for key,(r,source) in rows.items():
  choices=[]
  if r['product_mode']!='materialized':
   choices.append(('materialized',dict(product_mode='materialized',view_normalize='deferred',view_reuse='off',support_gates='materialized')))
   if r['support_gates']=='certified':choices.append(('certification',dict(support_gates='gated')))
   if r['view_normalize']=='source':choices.append(('normalization',dict(view_normalize='deferred')))
   if r['product_mode']=='views-inputs':choices.append(('output-order',dict(product_mode='views')))
  if r['candidate'].startswith('essential'):
   if r['essential_layout']=='slab':choices.append(('storage',dict(essential_layout='enum')))
   for control in ['packed512','dense-dispatched']:
    choices.append(('control-'+control,dict(candidate=control,essential_layout='enum',product_mode='materialized',view_normalize='deferred',view_reuse='off',support_gates='materialized')))
  for kind,changes in choices:
   base_case=dict(zip(KEYS,key));base_case.update(changes);bk=tuple(base_case[k] for k in KEYS)
   if bk not in rows:missing.append(dict(comparison=kind,case=list(key)));continue
   base,other=rows[bk]
   if kind=='storage':
    assert r['input_nodes']==base['input_nodes'] and r['final_nodes']==base['final_nodes']
   elif not kind.startswith('control-'):
    assert r['input_nodes']==base['input_nodes'] and r['input_storage']==base['input_storage']
   pairs.append(dict(comparison=kind,case=dict(zip(KEYS,key)),baseline_case=base_case,source=source,baseline_source=other,
    phases={p:dict(baseline_median_s=st.median(base[p]),candidate_median_s=st.median(r[p]),baseline_over_candidate=st.median(base[p])/st.median(r[p]),baseline_min_s=min(base[p]),baseline_max_s=max(base[p]),candidate_min_s=min(r[p]),candidate_max_s=max(r[p]),baseline_samples=len(base[p]),candidate_samples=len(r[p])) for p in ['fresh_s','warm_s']},
    baseline_final_nodes=base['final_nodes'],candidate_final_nodes=r['final_nodes'],baseline_final_bytes_est=base['final_bytes_est'],candidate_final_bytes_est=r['final_bytes_est']))
 return rows,processes,next(iter(builds)),pairs,missing


def main():
 ap=argparse.ArgumentParser();ap.add_argument('inputs',nargs='+');ap.add_argument('--output',default='PREFIX-MEASUREMENTS.md');ap.add_argument('--record',default='prefix-comparison.json');args=ap.parse_args()
 rows,processes,binary,within,missing=collect(args.inputs)
 pairs=[]
 for key,(r,source) in rows.items():
  if not r['candidate'].startswith('prefix'):continue
  changes=[dict(candidate=r['candidate'].replace('prefix',base),retraction_count='words') for base in ['retraction','essential']]
  if r['retraction_count']=='words':changes.append(dict(retraction_count='scalar'))
  for control in ['packed512','dense-dispatched']:
   changes.append(dict(candidate=control,retraction_count='words',essential_layout='enum',product_mode='materialized',view_normalize='deferred',view_reuse='off',support_gates='materialized'))
  for change in changes:
   case=dict(zip(KEYS,key));case.update(change);bk=tuple(case[k] for k in KEYS)
   if bk not in rows:continue
   base,other=rows[bk]
   if change==dict(retraction_count='scalar'):
    for field in ['input_nodes','final_nodes','input_storage','final_storage','input_bytes_est','final_bytes_est']:assert r[field]==base[field],field
   assert (r['checksum'],r['admissible_worlds'],r['outputs'],r['rows'])==(base['checksum'],base['admissible_worlds'],base['outputs'],base['rows'])
   pairs.append(dict(case=dict(zip(KEYS,key)),baseline_case=case,source=source,baseline_source=other,phases={p:dict(candidate_median_s=st.median(r[p]),baseline_median_s=st.median(base[p]),baseline_over_candidate=st.median(base[p])/st.median(r[p]),candidate_min_s=min(r[p]),candidate_max_s=max(r[p]),baseline_min_s=min(base[p]),baseline_max_s=max(base[p]),candidate_samples=len(r[p]),baseline_samples=len(base[p])) for p in ['build_s','support_s','import_s','admission_s','fresh_s','warm_s','fresh_compute_s','fresh_count_s','warm_compute_s','warm_count_s']},candidate_final_nodes=r['final_nodes'],baseline_final_nodes=base['final_nodes'],candidate_final_bytes_est=r['final_bytes_est'],baseline_final_bytes_est=base['final_bytes_est']))
 result=dict(passed=True,inputs=args.inputs,binary_sha256=binary,cases=len(rows),matched_cases=len(pairs),pairs=pairs,within_representation_pairs=within,processes=processes,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),selection='Latest supplied process per exact case; retain raw samples including outliers. One executable only.')
 (LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
 lines=['# Prefix-preserving completion: native Free Join comparison','','Construction is measured separately. Fresh and warm queries include exact legal','population checksums over all 80 outputs. Every output is also checked pointwise','outside timing. Retained KB includes construction residue and owner caches; temporary','observation memo and allocator overhead are excluded. No raw alias counts are used.',f'Executable: `{binary}`.','']
 for group in sorted({k[1:5] for k in rows}):
  domain,bits,layout,memo=group
  lines += [f'## {domain}, {bits} coordinates, {layout}, memo {memo}','','| Carrier/store | Product | Gates | Count | Build ms | Fresh ms | Compute ms | Observe ms | Min–max | Warm ms | Final KB | Nodes | Samples |','| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |']
  for key,(r,_) in sorted(rows.items()):
   if key[1:5]!=group:continue
   lines.append('| '+' | '.join([r['candidate']+'/'+r['essential_layout'],r['product_mode'],r['support_gates'],r['retraction_count'],f'{1e3*st.median(r["build_s"]):.3f}',f'{1e3*st.median(r["fresh_s"]):.3f}',f'{1e3*st.median(r["fresh_compute_s"]):.3f}',f'{1e3*st.median(r["fresh_count_s"]):.3f}',f'{1e3*min(r["fresh_s"]):.3f}–{1e3*max(r["fresh_s"]):.3f}',f'{1e3*st.median(r["warm_s"]):.3f}',f'{r["final_bytes_est"]/1e3:.1f}',str(r['final_nodes']),str(len(r['fresh_s']))])+' |')
  lines.append('')
 lines += [f'{len(rows)} configurations; {len(pairs)} matched completion/control comparisons; {len(processes)} retained processes.','']
 lines += [f'- [Raw evidence](results/{name}).' for name in args.inputs]
 (LAB/args.output).write_text('\n'.join(lines)+'\n');print(json.dumps({k:result[k] for k in ['passed','cases','matched_cases']}))

if __name__=='__main__':main()
