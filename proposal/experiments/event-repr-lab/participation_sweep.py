"""Serial participation-index checks and bounded, same-binary comparisons."""
from pathlib import Path
import argparse, hashlib, itertools, json, random
from pack_run import LAB, RESULTS, execute, check_engine_sources

ap = argparse.ArgumentParser()
ap.add_argument('--phase', choices=['correctness', 'smoke', 'comparison', 'repeat'], required=True)
ap.add_argument('--output', required=True)
ap.add_argument('--trials', type=int, default=7)
args = ap.parse_args()
assert args.trials > 0
check_engine_sources()
build = json.loads((RESULTS / 'build-participation.json').read_text())
binary = Path(build['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest() == build['binary_sha256']
for name, sha in build['source_sha256'].items():
    assert hashlib.sha256((LAB / name).read_bytes()).hexdigest() == sha
common = dict(EVENT_LAB_ESSENTIAL_KERNEL='derived', EVENT_LAB_ESSENTIAL_LAYOUT='slab',
    EVENT_LAB_OCCUPANCY_KERNEL='words', EVENT_LAB_PACKED_MAP='local',
    EVENT_LAB_TRANSFER_IMPORT='words', EVENT_LAB_RETRACTION_FACTOR='faces',
    EVENT_LAB_RETRACTION_NORMALIZE='ite', EVENT_LAB_RETRACTION_PROJECT='witness',
    EVENT_LAB_RETRACTION_COMPLETE='local-needed', EVENT_LAB_PRODUCT='materialized')
if args.phase == 'correctness':
    jobs = [dict(common, verify=True)]
else:
    candidates = ['dense', 'packed512']
    widths, layouts, fanouts, shapes = [5], ['face-major', 'bit-major'], [2, 8], ['balanced', 'skew', 'unmatched']
    if args.phase == 'smoke':
        candidates += ['essential512', 'retraction512', 'range-shannon', 'range-group']
        widths, layouts, fanouts, shapes = [4], ['bit-major'], [4], ['balanced']
    jobs = [dict(common, EVENT_LAB_CANDIDATE=c, EVENT_LAB_LANE='relation-pack',
        EVENT_PACK_WIDTH=w, EVENT_LAB_LAYOUT=l, EVENT_PACK_FANOUT=f,
        EVENT_PACK_SHAPE=s, EVENT_PACK_ORDER=(i + (2 if args.phase == 'repeat' else 0)) % 4)
        for i, (c, w, l, f, s) in enumerate(itertools.product(candidates, widths, layouts, fanouts, shapes))]
seed = 20260924 if args.phase == 'repeat' else 20260923
random.Random(seed).shuffle(jobs)
folder = RESULTS / (Path(args.output).stem + '-logs')
folder.mkdir(exist_ok=False)
output = RESULTS / args.output
assert not output.exists(), 'Preserve prior evidence; choose a new output name.'
result = dict(passed=False, phase=args.phase, trials=args.trials, seed=seed,
    build_metadata=build, runs=[], runner_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    execute_sha256=hashlib.sha256((LAB / 'pack_run.py').read_bytes()).hexdigest(),
    boundary='Four schedules, serial processes. Input setup/arena cloning are separate; query timing includes validation, folds, summary planning and native joining. Root capacities and map payloads are censuses, not peak allocated memory.')
for job in jobs:
    run = execute(str(binary), job, len(result['runs']), args.trials, 240, folder)
    result['runs'].append(run)
    output.write_text(json.dumps(result, indent=2) + '\n')
    if run['status'] != 'passed':
        raise SystemExit('Participation phase failed: ' + run['log'])
result['passed'] = True
output.write_text(json.dumps(result, indent=2) + '\n')
print('Participation phase passed:', args.phase, len(jobs), 'processes', flush=True)
