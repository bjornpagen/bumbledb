"""Serial native admission/timing; require the matching isolated range build."""
from pathlib import Path
import argparse,hashlib,itertools,json,random
from run import LAB,RESULTS,execute,check_engine_sources

ap=argparse.ArgumentParser()
ap.add_argument('--phase',choices=['correctness','admission','join','algebra'],required=True)
ap.add_argument('--output',required=True)
ap.add_argument('--trials',type=int,default=7)
args=ap.parse_args();assert args.trials>0
check_engine_sources();build=json.loads((RESULTS/'build-range-native.json').read_text())
binary=Path(build['binary']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==build['binary_sha256']
for name,sha in build['source_sha256'].items():assert hashlib.sha256((LAB/name).read_bytes()).hexdigest()==sha,name
common=dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT='slab',
    EVENT_LAB_OCCUPANCY_KERNEL='words',EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',
    EVENT_LAB_RETRACTION_FACTOR='faces',EVENT_LAB_RETRACTION_NORMALIZE='ite',
    EVENT_LAB_RETRACTION_PROJECT='witness',EVENT_LAB_RETRACTION_COMPLETE='local-needed',
    EVENT_LAB_PRODUCT='materialized')
ranges=['range-shannon','range-step','range-group']
if args.phase=='correctness':jobs=[dict(common,verify=True)]
elif args.phase=='admission':
    jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE=l,EVENT_LAB_LAYOUT='bit-major')
        for c,l in itertools.product(ranges,['owned','symbolic-relations','transport','laws'])]
elif args.phase=='join':
    jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE='join',EVENT_LAB_SCENARIO=s)
        for c,s in itertools.product(ranges+['packed512','dense','essential512','retraction512'],['coup_4290','coup_product_65536'])]
else:
    jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE=l,EVENT_LAB_LAYOUT=o)
        for c,l,o in itertools.product(ranges+['packed512','essential512','retraction512'],['legal-relations','readouts'],['bit-major','face-major'])]
random.Random(20260918).shuffle(jobs)
folder=RESULTS/(Path(args.output).stem+'-logs');folder.mkdir(exist_ok=False)
result=dict(passed=False,phase=args.phase,trials=args.trials,build_metadata=build,runs=[],
    runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    execute_sha256=hashlib.sha256((LAB/'run.py').read_bytes()).hexdigest(),
    boundary='Serial native Free Join, same executable. Preparation excluded; query timing includes exact output counts. Admission/correctness trials are not a latency ranking.')
for job in jobs:
    run=execute(str(binary),job,len(result['runs']),args.trials,240,folder)
    result['runs'].append(run);(RESULTS/args.output).write_text(json.dumps(result,indent=2)+'\n')
    if run['status']!='passed':raise SystemExit('Native range phase failed: '+run['log'])
result['passed']=True;(RESULTS/args.output).write_text(json.dumps(result,indent=2)+'\n')
print('Native range phase passed:',args.phase)
