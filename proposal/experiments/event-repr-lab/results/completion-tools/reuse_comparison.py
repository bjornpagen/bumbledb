"""Recompute matched native comparisons; normalization and cache are separate."""
from pathlib import Path
import argparse,hashlib,json,statistics as st
from prepare import LAB

KEYS=['kind','bits','layout','memo','candidate','essential_layout','product_mode','view_kernel','view_normalize','view_reuse']
KINDS={'relations':'relations_free_join','products':'product_free_join'}
FIELDS=[('layout','EVENT_LAB_LAYOUT'),('product_mode','EVENT_LAB_PRODUCT'),('essential_layout','EVENT_LAB_ESSENTIAL_LAYOUT'),
 ('view_kernel','EVENT_LAB_VIEW_KERNEL'),('view_normalize','EVENT_LAB_VIEW_NORMALIZE'),('view_reuse','EVENT_LAB_VIEW_REUSE')]
def collect(inputs):
 rows,processes,builds={},[],set()
 for name in inputs:
  data=json.loads((LAB/'results'/name).read_text());builds.add(data['build_metadata']['binary_sha256'])
  for run in data['runs']:
   job=run['job'];processes.append(dict(file=name,job=job,status=run['status'],log=run['log']))
   if run['status']!='passed':
    for key in list(rows):
     case=dict(zip(KEYS,key))
     if (case['kind']==KINDS.get(job.get('EVENT_LAB_LANE')) and case['candidate']==job.get('EVENT_LAB_CANDIDATE')
         and all(case[f]==job.get(e) for f,e in FIELDS)):
      del rows[key]
    continue
   for row in run['rows']:
    if row.get('kind') not in KINDS.values():continue
    assert row['verified'] and row['essential_kernel']=='derived'
    for field,env in FIELDS:assert row[field]==job[env],(name,field)
    assert row['groups']==16 and row['rows']==128 and row['bits'] in [12,18]
    assert row['worlds']==1<<row['bits']
    assert len(row['fresh_s'])==len(row['warm_s'])==data['trials']
    for field in ['input_storage','final_storage']:
     s=row[field]
     if s is not None:
      assert s['layout']==row['essential_layout']
      assert s['total_bytes_est']==sum(s[k] for k in ['record_bytes','table_bytes','interner_bytes','metadata_bytes','cache_bytes'])
    rows[tuple(row[k] for k in KEYS)]=(row,name)
 assert len(builds)==1,'Do not merge different executables'
 answers={}
 for row,_ in rows.values():
  key=(row['kind'],row['bits']);answer=(row['checksum'],row['rows'],row['groups'],row.get('closure_iterations'))
  assert answers.setdefault(key,answer)==answer
 pairs=[];missing=[]
 for key,(row,source) in rows.items():
  if row['product_mode']=='materialized':continue
  choices=[('materialized',key[:6]+('materialized',row['view_kernel'],'deferred','off'))]
  if row['view_reuse']=='bounded':choices.append(('reuse',key[:-1]+('off',)))
  if row['view_normalize']=='source':choices.append(('normalization',key[:8]+('deferred',row['view_reuse'])))
  if row['product_mode']=='views-inputs':choices.append(('output-order',key[:6]+('views',)+key[7:]))
  for kind,baseline in choices:
   if baseline not in rows:missing.append(dict(comparison=kind,case=list(key)));continue
   base,other=rows[baseline]
   assert row['input_nodes']==base['input_nodes'] and row['input_storage']==base['input_storage']
   if kind=='reuse':
    for field in ['final_nodes','final_storage','final_bytes_est']:assert row[field]==base[field],(key,field)
   pairs.append(dict(comparison=kind,case=dict(zip(KEYS,key)),source=source,baseline_source=other,
    baseline_case=dict(zip(KEYS,baseline)),
    phases={p:dict(baseline_median_s=st.median(base[p]),candidate_median_s=st.median(row[p]),
     baseline_over_candidate=st.median(base[p])/st.median(row[p]),baseline_min_s=min(base[p]),baseline_max_s=max(base[p]),
     candidate_min_s=min(row[p]),candidate_max_s=max(row[p]),baseline_samples=len(base[p]),candidate_samples=len(row[p])) for p in ['fresh_s','warm_s']},
    baseline_final_nodes=base['final_nodes'],candidate_final_nodes=row['final_nodes'],
    baseline_final_bytes_est=base['final_bytes_est'],candidate_final_bytes_est=row['final_bytes_est']))
 return rows,processes,next(iter(builds)),pairs,missing

def main():
 ap=argparse.ArgumentParser();ap.add_argument('inputs',nargs='+');ap.add_argument('--output',default='REUSE-MEASUREMENTS.md');ap.add_argument('--record',default='reuse-comparison.json')
 args=ap.parse_args();rows,processes,binary,pairs,missing=collect(args.inputs)
 result=dict(passed=True,inputs=args.inputs,binary_sha256=binary,cases=len(rows),matched_cases=len(pairs),processes=processes,pairs=pairs,missing_baselines=missing,
 checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),selection='Latest supplied process per exact case; later failures invalidate earlier timings. Resource limits and timeouts are retained, never mathematical answers.')
 (LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
 lines=['# Source normalization and bounded plane reuse through Free Join','',
  'Same executable and output contracts. Source normalization reduces pinned operands',
  'in the existing canonical arena before product memo lookup. Bounded reuse caches',
  'local aligned planes within one product; recomputation remains the control.',
  'Reuse pairs must retain exactly identical final nodes, caches and arena bytes.',
  'Resident KB estimates omit the bounded temporary plane cache, other temporary',
  'workspace and allocator overhead; see [the contract and bounds](REUSE.md).',
  'Times are milliseconds. Query includes actual Free Join, canonical construction',
  'and exact count. Input construction, cloning, setup and bitset validation are outside.',
  'The 16-output product lane and 80-output relation program stay separate.',f'Executable: `{binary}`.','']
 for group in sorted({key[:4] for key in rows}):
  kind,bits,layout,memo=group
  lines += [f'## {kind}, {bits} coordinates, {layout}, memo {memo}','',
   '| Carrier/store | Strategy | Normalization | Reuse | Fresh | Min–max | Warm | Final KB | Nodes | Samples |',
   '| --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |']
  for key,(r,_) in sorted(rows.items()):
   if key[:4]!=group:continue
   values=[r['candidate']+'/'+r['essential_layout'],r['product_mode'],r['view_normalize'],r['view_reuse'],
    f'{1000*st.median(r["fresh_s"]):.3f}',f'{1000*min(r["fresh_s"]):.3f}–{1000*max(r["fresh_s"]):.3f}',
    f'{1000*st.median(r["warm_s"]):.3f}',f'{r["final_bytes_est"]/1000:.1f}',str(r['final_nodes']),str(len(r['fresh_s']))]
   lines.append('| '+' | '.join(values)+' |')
  lines.append('')
 lines += [f'{len(rows)} cases; {len(pairs)} matched comparisons; {len(processes)} retained processes.','']
 for r in processes:
  if r['status']!='passed':lines.append(f'- {r["status"]}: [{r["log"]}](results/{r["log"]}).')
 lines += [f'- [Raw evidence](results/{n}).' for n in args.inputs]
 (LAB/args.output).write_text('\n'.join(lines)+'\n')
 print(f'{len(rows)} cases; {len(pairs)} matched comparisons; {len(missing)} missing baselines.')
if __name__=='__main__':main()
