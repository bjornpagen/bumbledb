"""Native ownership/symbolic acceptance for both completion-normalization paths."""
from pathlib import Path
import argparse
import hashlib
import json
import random
from run import LAB, RESULTS, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--output', required=True)
ap.add_argument('--trials', type=int, default=1)
ap.add_argument('--normalize', nargs='+', choices=['staged','equal'], default=['staged','equal'])
args = ap.parse_args()
check_engine_sources()
build = json.loads((RESULTS/'build.json').read_text())
binary = Path(build['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == build['binary_sha256']
for name, digest in build['lab_sources'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest
folder = RESULTS/(Path(args.output).stem+'-logs')
folder.mkdir(exist_ok=False)
common = dict(EVENT_LAB_RETRACTION_COUNT='words', EVENT_LAB_RETRACTION_FACTOR='faces',
              EVENT_LAB_ESSENTIAL_KERNEL='derived', EVENT_LAB_ESSENTIAL_LAYOUT='slab',
              EVENT_LAB_OCCUPANCY_KERNEL='words', EVENT_LAB_VIEW_KERNEL='words',
              EVENT_LAB_VIEW_REUSE='bounded', EVENT_LAB_VIEW_NORMALIZE='source',
              EVENT_LAB_PACKED_MAP='local', EVENT_LAB_TRANSFER_IMPORT='words',
              EVENT_LAB_IDENTITY='native', EVENT_LAB_LAYOUT='bit-major')
jobs = [dict(common, EVENT_LAB_CANDIDATE=candidate, EVENT_LAB_LANE=lane,
             EVENT_LAB_PRODUCT=product, EVENT_LAB_RETRACTION_NORMALIZE=mode)
        for candidate in ['prefix64','prefix512','retraction64','retraction512']
        for lane in ['owned','symbolic-relations']
        for product in ['materialized','views']
        for mode in args.normalize]
random.Random(20260927).shuffle(jobs)
runs = []
for job in jobs:
    result = execute(str(binary), job, len(runs), args.trials, 240, folder)
    runs.append(result)
    (RESULTS/args.output).write_text(json.dumps(dict(
        build_metadata=build, trials=args.trials, runs=runs,
        checker_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()), indent=2)+'\n')
    if result['status'] != 'passed':
        raise SystemExit('Native completion acceptance failed: '+result['log'])
print('Native completion acceptance passed.')
