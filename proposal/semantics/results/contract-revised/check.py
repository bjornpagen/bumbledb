"""Recheck the public contract and retained proofs with the pinned installed Lean.

Each invocation creates a new result directory and snapshots exactly the checked
sources. It never overwrites the laboratory's historical evidence or downloads tools.
"""
from pathlib import Path
import argparse, datetime, hashlib, json, re, shutil, subprocess

HERE = Path(__file__).resolve().parent
PROPOSAL = HERE.parent
LAB = PROPOSAL / 'experiments/event-repr-lab'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--label', required=True)
    ap.add_argument('--only-new', action='store_true')
    args = ap.parse_args()
    assert re.fullmatch(r'[a-z0-9-]+', args.label)
    baseline = json.loads((LAB / 'results/lean-check.json').read_text())
    binary = Path(baseline['lean_binary'])
    assert digest(binary) == baseline['lean_binary_sha256']
    version = subprocess.check_output([str(binary), '--version'], text=True).strip()
    assert version == baseline['lean_version']
    output = HERE / 'results' / args.label
    output.mkdir(parents=True, exist_ok=False)
    shutil.copy2(__file__, output / 'check.py')
    expected = {}
    if not args.only_new:
        for row in baseline['files']:
            path = LAB / 'lean' / row['file']
            assert digest(path) == baseline['source_hashes']['lean/' + row['file']]
            expected[path] = list(row['checked_axioms'])
        for path in sorted((LAB / 'join-factorization').glob('*.lean')):
            expected[path] = re.findall(r'^#print axioms ([\w.]+)', path.read_text(), re.M)
    for path in sorted(HERE.glob('*.lean')):
        expected[path] = re.findall(r'^#print axioms ([\w.]+)', path.read_text(), re.M)
    assert all(expected.values())
    hashes = {str(p.relative_to(PROPOSAL)): digest(p) for p in expected}
    for path in expected:
        dest = output / 'sources' / path.relative_to(PROPOSAL)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
    result = dict(passed=False, started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        lean_binary=str(binary), lean_binary_sha256=digest(binary), lean_version=version,
        checker_sha256=digest(Path(__file__)), source_sha256=hashes, files=[],
        scope='new-contract-only' if args.only_new else 'central-plus-pack-plus-public-contract',
        boundary='Denotational semantics with explicit premises. Not Rust refinement, solver correctness, canonical wire proof or native planner correctness.')
    for i, (path, names) in enumerate(expected.items()):
        command = [str(binary), str(path)]
        p = subprocess.run(command, cwd=path.parent, text=True, capture_output=True, timeout=120)
        log = p.stdout + p.stderr
        logfile = f'{i:02}-{path.stem}.log'
        (output / logfile).write_text(log)
        axioms = {}
        for name in names:
            if f"'{name}' does not depend on any axioms" in log:
                axioms[name] = []
            else:
                m = re.search(re.escape(f"'{name}' depends on axioms: [") + r'([^\]]*)\]', log)
                if m:
                    axioms[name] = [v.strip() for v in m.group(1).split(',')]
        permitted = {'propext', 'Quot.sound', 'Classical.choice'}
        passed = p.returncode == 0 and len(axioms) == len(names) and all(set(v) <= permitted for v in axioms.values())
        result['files'].append(dict(source=str(path.relative_to(PROPOSAL)), command=command,
            exit_code=p.returncode, log=logfile, expected=names, axioms=axioms, passed=passed))
        (output / 'check.json').write_text(json.dumps(result, indent=2) + '\n')
        print(path.stem, 'passed' if passed else 'FAILED', len(axioms), 'reports', flush=True)
        if not passed:
            print(log, flush=True)
    assert hashes == {str(p.relative_to(PROPOSAL)): digest(p) for p in expected}, 'Proof source changed during verification'
    result['passed'] = all(row['passed'] for row in result['files'])
    result['reports'] = sum(len(row['axioms']) for row in result['files'])
    result['axiom_free'] = sum(not v for row in result['files'] for v in row['axioms'].values())
    result['finished_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (output / 'check.json').write_text(json.dumps(result, indent=2) + '\n')
    print('Verified', result['reports'], 'reports;', result['axiom_free'], 'axiom-free.', flush=True)
    raise SystemExit(0 if result['passed'] else 1)

if __name__ == '__main__':
    main()
