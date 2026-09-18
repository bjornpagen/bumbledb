"""Isolate storage layout in one native executable, with unchanged algebra."""
from pathlib import Path
import argparse, hashlib, json, random
from run import LAB, RESULTS, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--output', required=True)
ap.add_argument('--trials', type=int, default=5)
ap.add_argument('--timeout', type=int, default=180)
ap.add_argument('--seed', type=int, default=20260927)
ap.add_argument('--candidate', nargs='+',
                choices=['essential64','essential512','packed64','packed512','dense-dispatched'],
                default=['essential64','essential512','packed64','packed512','dense-dispatched'])
ap.add_argument('--lane', nargs='+', choices=['join','classify','owned','laws','symbolic-relations'],
                default=['join','classify','owned','laws','symbolic-relations'])
ap.add_argument('--scenario', nargs='+', choices=['coup_4290','coup_product_65536'],
                default=['coup_4290','coup_product_65536'])
ap.add_argument('--classifier', nargs='+', choices=['cells','direct'], default=['cells','direct'])
args = ap.parse_args()
assert args.trials > 0
check_engine_sources()
metadata = json.loads((RESULTS/'build.json').read_text())
binary = Path(metadata['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == metadata['binary_sha256']
for name,digest in metadata['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest, name
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
(folder/'build.json').write_text(json.dumps(metadata,indent=2)+'\n')
records = []
def save():
    (RESULTS/args.output).write_text(json.dumps(dict(
        experiment='Enum keys versus exact fingerprint/slab store; same normal form, algorithms and native executable',
        seed=args.seed, trials=args.trials, timeout_s=args.timeout,
        build_metadata=metadata, runs=records),indent=2)+'\n')
common = dict(EVENT_LAB_ESSENTIAL_KERNEL='derived', EVENT_LAB_PACKED_MAP='local',
              EVENT_LAB_TRANSFER_IMPORT='words', EVENT_LAB_IDENTITY='native',
              EVENT_LAB_LAYOUT='bit-major')
for storage in ['enum','slab']:
    record = execute(str(binary), dict(common,verify=True,EVENT_LAB_ESSENTIAL_LAYOUT=storage),
                     len(records),args.trials,args.timeout,folder)
    records.append(record); save()
    if record['status'] != 'passed': raise SystemExit('Native verification failed: '+record['log'])
jobs = []
for candidate in args.candidate:
    for storage in ['enum','slab'] if candidate.startswith('essential') else ['enum']:
        for lane in args.lane:
            if lane == 'symbolic-relations' and candidate == 'dense-dispatched': continue
            for scenario in args.scenario if lane in ['join','classify'] else ['']:
                for classifier in args.classifier if lane == 'classify' else ['']:
                    job = dict(common,EVENT_LAB_ESSENTIAL_LAYOUT=storage,
                               EVENT_LAB_CANDIDATE=candidate,EVENT_LAB_LANE=lane)
                    if scenario: job['EVENT_LAB_SCENARIO'] = scenario
                    if classifier: job['EVENT_LAB_CLASSIFY'] = classifier
                    jobs.append(job)
random.Random(args.seed).shuffle(jobs)
for job in jobs:
    record = execute(str(binary),job,len(records),args.trials,args.timeout,folder)
    records.append(record); save()
    if record['status'] == 'failed': raise SystemExit('Correctness failure: '+record['log'])
print('Saved '+str(RESULTS/args.output),flush=True)
