"""Untimed operation counters. Never run alongside native performance sweeps."""
from pathlib import Path
import hashlib,json,os,subprocess
from prepare import LAB,SCRATCH
folder=LAB/'essential-prototype'
files=[folder/'Cargo.toml',folder/'Cargo.lock',folder/'src/lib.rs',folder/'examples/reuse_work.rs']+[LAB/'src'/n for n in ['essential_raw.rs','essential_store.rs','essential_words.rs','view_product.rs','relations.rs','native.rs','carrier.rs']]
def hashes():return {str(f.relative_to(LAB)):hashlib.sha256(f.read_bytes()).hexdigest() for f in files}
before=hashes();runs=[]
for storage in ['enum','slab']:
 for normalization in ['deferred','source']:
  for reuse in ['off','bounded']:
   env=dict(os.environ,CARGO_TARGET_DIR=str(SCRATCH/'view-target'),EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT=storage,EVENT_LAB_VIEW_KERNEL='words',EVENT_LAB_VIEW_NORMALIZE=normalization,EVENT_LAB_VIEW_REUSE=reuse)
   command=['cargo','run','--offline','--release','--example','reuse_work']
   p=subprocess.run(command,cwd=folder,env=env,text=True,capture_output=True)
   log=f'reuse-work-{storage}-{normalization}-{reuse}.log';(LAB/'results'/log).write_text(p.stdout+'\n'+p.stderr)
   rows=[json.loads(line) for line in p.stdout.splitlines() if line.startswith('{')]
   assert p.returncode==0 and len(rows)==24 and all(r['verified'] for r in rows),(storage,normalization,reuse,p.stderr)
   runs.append(dict(status='passed',storage=storage,normalization=normalization,reuse=reuse,command=command,exit_code=p.returncode,log=log,rows=rows))
assert hashes()==before,'Sources changed during diagnostics'
# Changing the local reader must not change the outer traversal. Storage has
# no effect on these logical counts. Branch-recursive work differs by design.
reference={}
for run in runs:
 for r in run['rows']:
  key=(run['normalization'],r['cutoff'],r['bits'],r['layout'],r['fused'])
  value=(r['pairs'],r['sum'][:3],r['max'][:3],r['sum'][14:])
  assert reference.setdefault(key,value)==value
  assert r['sum'][8]==0
  if run['reuse']=='off':assert r['sum'][10:14]==[0,0,0,0]
  if run['normalization']=='deferred':assert r['sum'][14:]==[0,0,0]
result=dict(passed=True,source_hashes=before,checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),runs=runs,
 fields=['subproblems','memo_hits','local_cubes','borrowed_planes','cofactor_steps','permutation_steps','broadcast_steps','branch_planes','branch_assignments','branch_word_merges','plane_cache_hits','plane_cache_misses','plane_cache_evictions','plane_cache_bytes_peak','canonicalized_views','canonical_cofactor_steps','removed_coordinates'],
 scope='Untimed raw products for unique clover bank-index pairs, each starting from a fresh input clone. Exact materialized IDs checked. Omits native join, grouped union, outer memo and shared result arena; not a query profile or a timing claim.')
(LAB/'results/reuse-work.json').write_text(json.dumps(result,indent=2)+'\n')
print('192 verified diagnostic cases; no timings.')
