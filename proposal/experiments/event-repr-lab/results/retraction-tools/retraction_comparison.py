"""Compare completion choices using only matched rows from one native executable."""
from pathlib import Path
import argparse,hashlib,json,statistics as st
from legal_comparison import collect,KEYS
from prepare import LAB
ap=argparse.ArgumentParser();ap.add_argument('inputs',nargs='+');ap.add_argument('--output',default='RETRACTION-MEASUREMENTS.md');ap.add_argument('--record',default='retraction-comparison.json');args=ap.parse_args()
rows,processes,binary,within,missing=collect(args.inputs)
pairs=[]
for key,(r,source) in rows.items():
 if not r['candidate'].startswith('retraction'):continue
 changes=[dict(candidate=r['candidate'].replace('retraction','essential'))]
 for control in ['packed512','dense-dispatched']:
  changes.append(dict(candidate=control,essential_layout='enum',product_mode='materialized',view_normalize='deferred',view_reuse='off',support_gates='materialized'))
 for change in changes:
  case=dict(zip(KEYS,key));case.update(change);bk=tuple(case[k] for k in KEYS)
  if bk not in rows:continue
  base,other=rows[bk]
  assert (r['checksum'],r['admissible_worlds'],r['outputs'],r['rows'])==(base['checksum'],base['admissible_worlds'],base['outputs'],base['rows'])
  pairs.append(dict(case=dict(zip(KEYS,key)),baseline_case=case,source=source,baseline_source=other,phases={p:dict(candidate_median_s=st.median(r[p]),baseline_median_s=st.median(base[p]),baseline_over_candidate=st.median(base[p])/st.median(r[p]),candidate_min_s=min(r[p]),candidate_max_s=max(r[p]),baseline_min_s=min(base[p]),baseline_max_s=max(base[p]),candidate_samples=len(r[p]),baseline_samples=len(base[p])) for p in ['build_s','support_s','import_s','admission_s','fresh_s','warm_s']},candidate_final_nodes=r['final_nodes'],baseline_final_nodes=base['final_nodes'],candidate_final_bytes_est=r['final_bytes_est'],baseline_final_bytes_est=base['final_bytes_est']))
result=dict(passed=True,inputs=args.inputs,binary_sha256=binary,cases=len(rows),matched_cases=len(pairs),pairs=pairs,within_representation_pairs=within,processes=processes,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),row_checker_sha256=hashlib.sha256((LAB/'legal_comparison.py').read_bytes()).hexdigest(),selection='Latest supplied process per exact case; retain raw samples including outliers. One executable only.')
(LAB/'results'/args.record).write_text(json.dumps(result,indent=2)+'\n')
lines=['# Fixed-decoder completion: native Free Join comparison','','Construction is measured separately. Fresh and warm queries include exact legal','population checksums over all 80 outputs. Every output is also checked pointwise','outside timing. Retained KB includes construction residue and owner caches; temporary','observation memo and allocator overhead are excluded. No raw alias counts are used.',f'Executable: `{binary}`.','']
for group in sorted({k[1:5] for k in rows}):
 domain,bits,layout,memo=group
 lines += [f'## {domain}, {bits} coordinates, {layout}, memo {memo}','','| Carrier/store | Product | Gates | Build ms | Fresh ms | Min–max | Warm ms | Final KB | Nodes | Samples |','| --- | --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: |']
 for key,(r,_) in sorted(rows.items()):
  if key[1:5]!=group:continue
  lines.append('| '+' | '.join([r['candidate']+'/'+r['essential_layout'],r['product_mode'],r['support_gates'],f'{1e3*st.median(r["build_s"]):.3f}',f'{1e3*st.median(r["fresh_s"]):.3f}',f'{1e3*min(r["fresh_s"]):.3f}–{1e3*max(r["fresh_s"]):.3f}',f'{1e3*st.median(r["warm_s"]):.3f}',f'{r["final_bytes_est"]/1e3:.1f}',str(r['final_nodes']),str(len(r['fresh_s']))])+' |')
 lines.append('')
lines += [f'{len(rows)} configurations; {len(pairs)} matched completion/control comparisons; {len(processes)} retained processes.','']
lines += [f'- [Raw evidence](results/{name}).' for name in args.inputs]
(LAB/args.output).write_text('\n'.join(lines)+'\n');print(json.dumps({k:result[k] for k in ['passed','cases','matched_cases']}))
