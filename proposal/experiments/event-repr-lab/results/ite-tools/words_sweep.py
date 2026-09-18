"""Matched local-cube classifiers, resident stores, and constructive controls."""
from pathlib import Path
import argparse, hashlib, json, random
from run import LAB, RESULTS, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--output', required=True)
ap.add_argument('--trials', type=int, default=3)
ap.add_argument('--timeout', type=int, default=180)
ap.add_argument('--seed', type=int, default=20260930)
ap.add_argument('--candidate', nargs='+', choices=['essential64','essential512','packed512','dense-dispatched'],
                default=['essential64','essential512','packed512','dense-dispatched'])
ap.add_argument('--scenario', nargs='+', choices=['coup_4290','coup_product_65536','ordered_65536'],
                default=['coup_4290','coup_product_65536','ordered_65536'])
ap.add_argument('--kernel', nargs='+', choices=['scalar','words'], default=['scalar','words'])
ap.add_argument('--layout', nargs='+', choices=['enum','slab'], default=['enum','slab'])
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
        experiment='Scalar versus borrowed/aligned word occupancy; same exact readout, traversal and executable',
        seed=args.seed,trials=args.trials,timeout_s=args.timeout,
        build_metadata=metadata,runs=records),indent=2)+'\n')
common = dict(EVENT_LAB_ESSENTIAL_KERNEL='derived',EVENT_LAB_PACKED_MAP='local',
              EVENT_LAB_TRANSFER_IMPORT='words',EVENT_LAB_IDENTITY='native')
for storage in args.layout:
    for kernel in args.kernel:
        job = dict(common,verify=True,EVENT_LAB_ESSENTIAL_LAYOUT=storage,EVENT_LAB_OCCUPANCY_KERNEL=kernel)
        record = execute(str(binary),job,len(records),args.trials,args.timeout,folder)
        records.append(record); save()
        if record['status'] != 'passed': raise SystemExit('Verification failed: '+record['log'])
jobs = []
for candidate in args.candidate:
    for storage in args.layout if candidate.startswith('essential') else ['enum']:
        for scenario in args.scenario:
            for classifier in args.classifier:
                for kernel in args.kernel if candidate.startswith('essential') and classifier == 'direct' else ['scalar']:
                    jobs.append(dict(common,EVENT_LAB_LANE='classify',EVENT_LAB_CANDIDATE=candidate,
                                     EVENT_LAB_SCENARIO=scenario,EVENT_LAB_CLASSIFY=classifier,
                                     EVENT_LAB_ESSENTIAL_LAYOUT=storage,EVENT_LAB_OCCUPANCY_KERNEL=kernel))
random.Random(args.seed).shuffle(jobs)
for job in jobs:
    record = execute(str(binary),job,len(records),args.trials,args.timeout,folder)
    records.append(record); save()
    if record['status'] == 'failed': raise SystemExit('Correctness failure: '+record['log'])
print('Saved '+str(RESULTS/args.output),flush=True)
