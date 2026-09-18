"""Matched completion/count-kernel controls, serial native jobs."""
from pathlib import Path
import argparse,hashlib,json,random
from run import LAB,RESULTS,execute,check_engine_sources

ap=argparse.ArgumentParser()
ap.add_argument('--output',required=True)
ap.add_argument('--trials',type=int,default=3)
ap.add_argument('--seed',type=int,default=20260918)
ap.add_argument('--timeout',type=int,default=180)
ap.add_argument('--candidate',nargs='+',choices=['retraction64','retraction512','essential64','essential512','packed512','dense-dispatched'],default=['essential64','essential512','packed512','dense-dispatched'])
ap.add_argument('--layout',nargs='+',choices=['face-major','bit-major','pair-major'],default=['face-major','bit-major'])
ap.add_argument('--storage',nargs='+',choices=['enum','slab'],default=['enum','slab'])
ap.add_argument('--domain',nargs='+',choices=['full','below','holes','fibred'],default=['full','below','holes','fibred'])
ap.add_argument('--gates',nargs='+',choices=['gated','certified'],default=['gated','certified'])
ap.add_argument('--mode',nargs='+',choices=['materialized','views','views-inputs'],default=['materialized','views','views-inputs'])
ap.add_argument('--normalize',nargs='+',choices=['deferred','source'],default=['source'])
ap.add_argument('--reuse',nargs='+',choices=['off','bounded'],default=['bounded'])
ap.add_argument('--count-kernel',nargs='+',choices=['scalar','words'],default=['words'])
args=ap.parse_args();assert args.trials>0
check_engine_sources()
metadata=json.loads((RESULTS/'build.json').read_text());binary=Path(metadata['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest()==metadata['binary_sha256']
for name,digest in metadata['lab_sources'].items():assert hashlib.sha256((LAB/name).read_bytes()).hexdigest()==digest,name
folder=RESULTS/(Path(args.output).stem+'-logs');folder.mkdir(exist_ok=False)
(folder/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
runs=[]
def save():
 (RESULTS/args.output).write_text(json.dumps(dict(experiment='Checked domain/role admission, actual Free Join group union/intersection, closure, converse, residual, May and nonvacuous Must; 80 complete outputs',build_metadata=metadata,trials=args.trials,timeout_s=args.timeout,seed=args.seed,runs=runs),indent=2)+'\n')
common=dict(EVENT_LAB_RETRACTION_COUNT='words',EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_OCCUPANCY_KERNEL='words',EVENT_LAB_VIEW_KERNEL='words',EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',EVENT_LAB_IDENTITY='native')
def modes(selected):
 for mode in selected:
  for normalization in args.normalize if mode!='materialized' else ['deferred']:
   for reuse in args.reuse if mode!='materialized' else ['off']:
    for gates in args.gates if mode!='materialized' else ['gated']:
     yield dict(EVENT_LAB_PRODUCT=mode,EVENT_LAB_VIEW_NORMALIZE=normalization,EVENT_LAB_VIEW_REUSE=reuse,EVENT_LAB_SUPPORT_GATES=gates)
verification_choices={}
for choice in modes(args.mode):
 verification_choices.setdefault(tuple((k,v) for k,v in choice.items() if k!='EVENT_LAB_SUPPORT_GATES'),choice)
# Semantic checks exercise both certificate paths independently of this job setting.
for storage in args.storage:
 for choice in verification_choices.values():
  r=execute(str(binary),dict(common,**choice,verify=True,EVENT_LAB_ESSENTIAL_LAYOUT=storage),len(runs),args.trials,args.timeout,folder)
  runs.append(r);save()
  if r['status']!='passed':raise SystemExit('Verification failed: '+r['log'])
jobs=[]
for candidate in args.candidate:
 for storage in args.storage if candidate.startswith(('essential','retraction')) else ['enum']:
  for choice in modes(args.mode if candidate.startswith(('essential','retraction')) else ['materialized']):
   for layout in args.layout:
    for domain in args.domain:
     for count in args.count_kernel if candidate.startswith('retraction') else ['words']:
      job=dict(common,**choice,EVENT_LAB_CANDIDATE=candidate,EVENT_LAB_LANE='legal-relations',EVENT_LAB_ESSENTIAL_LAYOUT=storage,EVENT_LAB_LAYOUT=layout,EVENT_LAB_LEGAL_DOMAIN=domain)
      job['EVENT_LAB_RETRACTION_COUNT']=count; jobs.append(job)
random.Random(args.seed).shuffle(jobs)
for job in jobs:
 r=execute(str(binary),job,len(runs),args.trials,args.timeout,folder);runs.append(r);save()
 if r['status']=='failed':raise SystemExit('Correctness failed: '+r['log'])
print('Saved '+str(RESULTS/args.output),flush=True)
