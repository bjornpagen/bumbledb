"""Serial matched mapped-product strategies through the native executor."""
from pathlib import Path
import argparse,hashlib,json,random
from run import LAB,RESULTS,execute,check_engine_sources

ap=argparse.ArgumentParser()
ap.add_argument('--output',required=True)
ap.add_argument('--trials',type=int,default=5)
ap.add_argument('--seed',type=int,default=20261201)
ap.add_argument('--timeout',type=int,default=180)
ap.add_argument('--candidate',nargs='+',choices=['essential64','essential512','packed512','dense-dispatched'],
    default=['essential64','essential512','packed512','dense-dispatched'])
ap.add_argument('--layout',nargs='+',choices=['face-major','bit-major','pair-major'],
    default=['face-major','bit-major','pair-major'])
ap.add_argument('--storage',nargs='+',choices=['enum','slab'],default=['enum','slab'])
ap.add_argument('--lane',nargs='+',choices=['relations','products','symbolic-relations','owned'],default=['relations','products'])
ap.add_argument('--mode',nargs='+',choices=['materialized','views','views-inputs'],default=['materialized','views','views-inputs'])
ap.add_argument('--view-kernel',nargs='+',choices=['assignments','words'],default=['assignments','words'])
args=ap.parse_args()
assert args.trials>0
check_engine_sources()
metadata=json.loads((RESULTS/'build.json').read_text())
binary=Path(metadata['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest()==metadata['binary_sha256']
for name,digest in metadata['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest()==digest,name
folder=RESULTS/(Path(args.output).stem+'-logs');folder.mkdir(exist_ok=False)
(folder/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
runs=[]
def save():
    (RESULTS/args.output).write_text(json.dumps(dict(
        experiment='Materialized operands versus mapped views; same canonical relation outputs',
        build_metadata=metadata,trials=args.trials,timeout_s=args.timeout,seed=args.seed,runs=runs),indent=2)+'\n')
common=dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_OCCUPANCY_KERNEL='words',
    EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',EVENT_LAB_IDENTITY='native')
for storage in args.storage:
  for kernel in args.view_kernel:
    for mode in args.mode:
        job=dict(common,verify=True,EVENT_LAB_VIEW_KERNEL=kernel,EVENT_LAB_ESSENTIAL_LAYOUT=storage,EVENT_LAB_PRODUCT=mode)
        r=execute(str(binary),job,len(runs),args.trials,args.timeout,folder);runs.append(r);save()
        if r['status']!='passed':raise SystemExit('Verification failed: '+r['log'])
jobs=[]
for candidate in args.candidate:
    for storage in args.storage if candidate.startswith('essential') else ['enum']:
        for mode in args.mode if candidate.startswith('essential') else ['materialized']:
          for kernel in args.view_kernel if mode != 'materialized' else ['assignments']:
            for lane in args.lane:
                if lane=='symbolic-relations' and candidate=='dense-dispatched':continue
                for layout in ['bit-major'] if lane=='symbolic-relations' else args.layout:
                    jobs.append(dict(common,EVENT_LAB_VIEW_KERNEL=kernel,EVENT_LAB_CANDIDATE=candidate,EVENT_LAB_LANE=lane,
                        EVENT_LAB_ESSENTIAL_LAYOUT=storage,EVENT_LAB_PRODUCT=mode,EVENT_LAB_LAYOUT=layout))
random.Random(args.seed).shuffle(jobs)
for job in jobs:
    r=execute(str(binary),job,len(runs),args.trials,args.timeout,folder);runs.append(r);save()
    if r['status']=='failed':raise SystemExit('Correctness failed: '+r['log'])
print('Saved '+str(RESULTS/args.output),flush=True)
