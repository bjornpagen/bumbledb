"""Resolve small-fanout warm regressions and reverse order for the leading controls."""
from pathlib import Path
import hashlib,itertools,json,random
from pack_run import LAB,RESULTS,execute,check_engine_sources

check_engine_sources();build=json.loads((RESULTS/'build-pack-native.json').read_text())
binary=Path(build['binary']);assert hashlib.sha256(binary.read_bytes()).hexdigest()==build['binary_sha256']
for name,sha in build['source_sha256'].items():assert hashlib.sha256((LAB/name).read_bytes()).hexdigest()==sha
common=dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_ESSENTIAL_LAYOUT='slab',
    EVENT_LAB_OCCUPANCY_KERNEL='words',EVENT_LAB_PACKED_MAP='local',EVENT_LAB_TRANSFER_IMPORT='words',
    EVENT_LAB_RETRACTION_FACTOR='faces',EVENT_LAB_RETRACTION_NORMALIZE='ite',
    EVENT_LAB_RETRACTION_PROJECT='witness',EVENT_LAB_RETRACTION_COMPLETE='local-needed',EVENT_LAB_PRODUCT='materialized')
# Both repeated fanouts (2,8) ran complete first for these controls in the main
# sweep. Run factorized first here and independently shuffle process order.
candidates=['dense','packed512'];fanouts=[1,2,8]
jobs=[dict(common,EVENT_LAB_CANDIDATE=c,EVENT_LAB_LANE='pack',EVENT_LAB_SCENARIO=s,EVENT_PACK_FANOUT=f,
    EVENT_PACK_REVERSE=1) for c,s,f in itertools.product(candidates,['coup_4290','coup_product_65536'],fanouts)]
random.Random(20260919).shuffle(jobs)
output='pack-native-crossover-raw.json';folder=RESULTS/'pack-native-crossover-logs';folder.mkdir(exist_ok=False)
result=dict(passed=False,phase='comparison',trials=11,fanouts=fanouts,candidates=candidates,build_metadata=build,runs=[],
    runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    execute_sha256=hashlib.sha256((LAB/'pack_run.py').read_bytes()).hexdigest(),
    boundary='Focused follow-up for the small-fanout crossover and large fresh-query gains. Repeated fanouts 2 and 8 reverse schedule order; fanout 1 adds the no-Cartesian-expansion boundary. Exact same binary and query timer.')
for job in jobs:
    run=execute(str(binary),job,len(result['runs']),11,240,folder);result['runs'].append(run)
    (RESULTS/output).write_text(json.dumps(result,indent=2)+'\n')
    if run['status']!='passed':raise SystemExit('Pack crossover failed: '+run['log'])
result['passed']=True;(RESULTS/output).write_text(json.dumps(result,indent=2)+'\n');print('Pack crossover passed.')
