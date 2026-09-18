"""Isolated serial Pack schedule comparison, with matched carriers and inputs."""
from pathlib import Path
import argparse,hashlib,itertools,json,random
from pack_run import LAB,RESULTS,execute,check_engine_sources

ap=argparse.ArgumentParser();ap.add_argument('--phase',choices=['correctness','smoke','comparison'],required=True)
ap.add_argument('--output',required=True);ap.add_argument('--trials',type=int,default=7)
ap.add_argument('--fanout',type=int,nargs='+',default=[2,4,8]);args=ap.parse_args()
assert args.trials>0 and all(1<=f<=16 for f in args.fanout)
check_engine_sources();build=json.loads((RESULTS/'build-pack-native.json').read_text())
binary=Path(build['binary']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==build['binary_sha256']
for name,sha in build['source_sha256'].items():assert hashlib.sha256((LAB/name).read_bytes()).hexdigest()==sha
common=dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT='slab',
    EVENT_LAB_OCCUPANCY_KERNEL='words',EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',
    EVENT_LAB_RETRACTION_FACTOR='faces',EVENT_LAB_RETRACTION_NORMALIZE='ite',
    EVENT_LAB_RETRACTION_PROJECT='witness',EVENT_LAB_RETRACTION_COMPLETE='local-needed',
    EVENT_LAB_PRODUCT='materialized')
candidates=['range-shannon','range-group','packed512','dense','essential512','retraction512']
if args.phase=='correctness':jobs=[dict(common,verify=True)]
else:
    scenarios=['coup_4290'] if args.phase=='smoke' else ['coup_4290','coup_product_65536']
    fanouts=[4] if args.phase=='smoke' else args.fanout
    jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE='pack',EVENT_LAB_SCENARIO=s,EVENT_PACK_FANOUT=f,
        EVENT_PACK_REVERSE=(i%2)) for i,(c,s,f) in enumerate(itertools.product(candidates,scenarios,fanouts))]
random.Random(20260918).shuffle(jobs)
folder=RESULTS/(Path(args.output).stem+'-logs');folder.mkdir(exist_ok=False)
result=dict(passed=False,phase=args.phase,trials=args.trials,fanouts=args.fanout,build_metadata=build,runs=[],
    runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    execute_sha256=hashlib.sha256((LAB/'pack_run.py').read_bytes()).hexdigest(),
    boundary='Actual Free Join for complete bindings, branch scans and summary joining. Timed queries include validation, reduction, summary planning/images, joining and exact output counts. Original carrier-only timings are not directly comparable.')
for job in jobs:
    run=execute(str(binary),job,len(result['runs']),args.trials,240,folder);result['runs'].append(run)
    (RESULTS/args.output).write_text(json.dumps(result,indent=2)+'\n')
    if run['status']!='passed':raise SystemExit('Pack phase failed: '+run['log'])
result['passed']=True;(RESULTS/args.output).write_text(json.dumps(result,indent=2)+'\n');print('Pack phase passed:',args.phase)
