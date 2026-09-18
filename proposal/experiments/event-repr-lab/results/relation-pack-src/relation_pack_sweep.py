"""Serial native relational Pack correctness and matched schedule measurements."""
from pathlib import Path
import argparse,hashlib,itertools,json,random
from pack_run import LAB,RESULTS,execute,check_engine_sources

ap=argparse.ArgumentParser();ap.add_argument('--phase',choices=['correctness','smoke','comparison'],required=True)
ap.add_argument('--output',required=True);ap.add_argument('--trials',type=int,default=7)
args=ap.parse_args();assert args.trials>0
check_engine_sources();build=json.loads((RESULTS/'build-relation-pack.json').read_text());binary=Path(build['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest()==build['binary_sha256']
for name,sha in build['source_sha256'].items():assert hashlib.sha256((LAB/name).read_bytes()).hexdigest()==sha
common=dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT='slab',
    EVENT_LAB_OCCUPANCY_KERNEL='words',EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',
    EVENT_LAB_RETRACTION_FACTOR='faces',EVENT_LAB_RETRACTION_NORMALIZE='ite',EVENT_LAB_RETRACTION_PROJECT='witness',
    EVENT_LAB_RETRACTION_COMPLETE='local-needed',EVENT_LAB_PRODUCT='materialized')
candidates=['dense','packed512','essential512','retraction512','range-shannon','range-group']
if args.phase=='correctness':jobs=[dict(common,verify=True)]
else:
    widths=[4] if args.phase=='smoke' else [4,5];layouts=['bit-major'] if args.phase=='smoke' else ['face-major','bit-major']
    fanouts=[4] if args.phase=='smoke' else [2,8]
    jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE='relation-pack',EVENT_PACK_WIDTH=w,
        EVENT_LAB_LAYOUT=l,EVENT_PACK_FANOUT=f,EVENT_PACK_REVERSE=i%2)
        for i,(c,w,l,f) in enumerate(itertools.product(candidates,widths,layouts,fanouts))]
random.Random(20260920).shuffle(jobs)
folder=RESULTS/(Path(args.output).stem+'-logs');folder.mkdir(exist_ok=False)
result=dict(passed=False,phase=args.phase,trials=args.trials,build_metadata=build,runs=[],
    runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    execute_sha256=hashlib.sha256((LAB/'pack_run.py').read_bytes()).hexdigest(),
    boundary='Typed composition and left residual with shared environments and checked legal faces. Actual Free Join branch scans and summary join. Query charges participating validation, role admission, staging, summary planning/images, Event operations and exact output counts.')
for job in jobs:
    run=execute(str(binary),job,len(result['runs']),args.trials,240,folder);result['runs'].append(run)
    (RESULTS/args.output).write_text(json.dumps(result,indent=2)+'\n')
    if run['status']!='passed':raise SystemExit('Relational Pack phase failed: '+run['log'])
result['passed']=True;(RESULTS/args.output).write_text(json.dumps(result,indent=2)+'\n');print('Relational Pack phase passed:',args.phase)
