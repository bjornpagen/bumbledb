"""Reverse-order repeat for dense/packed ranking and the warm small-fanout boundary."""
from pathlib import Path
import hashlib,itertools,json,random
from pack_run import LAB,RESULTS,execute,check_engine_sources
check_engine_sources();build=json.loads((RESULTS/'build-relation-pack.json').read_text());binary=Path(build['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest()==build['binary_sha256']
for n,sha in build['source_sha256'].items():assert hashlib.sha256((LAB/n).read_bytes()).hexdigest()==sha
common=dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT='slab',EVENT_LAB_OCCUPANCY_KERNEL='words',
    EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',EVENT_LAB_RETRACTION_FACTOR='faces',
    EVENT_LAB_RETRACTION_NORMALIZE='ite',EVENT_LAB_RETRACTION_PROJECT='witness',EVENT_LAB_RETRACTION_COMPLETE='local-needed',EVENT_LAB_PRODUCT='materialized')
candidates=['dense','packed512'];widths=[5];layouts=['face-major','bit-major'];fanouts=[2,8]
jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE='relation-pack',EVENT_PACK_WIDTH=w,
    EVENT_LAB_LAYOUT=l,EVENT_PACK_FANOUT=f,EVENT_PACK_REVERSE=1 if f==2 else 0)
    for c,w,l,f in itertools.product(candidates,widths,layouts,fanouts)]
random.Random(20260921).shuffle(jobs)
output='relation-pack-repeat.json';folder=RESULTS/'relation-pack-repeat-logs';folder.mkdir(exist_ok=False)
result=dict(passed=False,phase='comparison',trials=11,candidates=candidates,widths=widths,layouts=layouts,fanouts=fanouts,
    build_metadata=build,runs=[],runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    execute_sha256=hashlib.sha256((LAB/'pack_run.py').read_bytes()).hexdigest(),
    boundary='Independent process order and reversed schedule order relative to main sweep. Focus: dense/packed ranking at width five and the small warm crossover. Same executable and timer.')
for job in jobs:
    run=execute(str(binary),job,len(result['runs']),11,240,folder);result['runs'].append(run)
    (RESULTS/output).write_text(json.dumps(result,indent=2)+'\n')
    if run['status']!='passed':raise SystemExit('Relational repeat failed: '+run['log'])
result['passed']=True;(RESULTS/output).write_text(json.dumps(result,indent=2)+'\n');print('Relational Pack repeat passed.')
