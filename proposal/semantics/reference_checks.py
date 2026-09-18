"""Retain fresh exact reference results without overwriting historical evidence."""
from pathlib import Path
import hashlib, json, shutil, subprocess, sys

HERE = Path(__file__).resolve().parent
PROPOSAL = HERE.parent
OUTPUT = HERE / 'results/reference-checks'
SCRIPTS = ['representation-checks.py', 'algebra-checks.py', 'coup/query-checks.py',
    'coup/event-checks.py', 'event-checks.py', 'process-checks.py', 'spec-checks.py']
OUTPUT.mkdir(parents=True, exist_ok=False)
digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
sources = {name:digest(PROPOSAL/name) for name in SCRIPTS}
for name in SCRIPTS:
    target = OUTPUT / 'sources' / name
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(PROPOSAL/name, target)
shutil.copy2(__file__, OUTPUT/'reference_checks.py')
result = dict(passed=False, python=sys.version, checker_sha256=digest(Path(__file__)),
    source_sha256=sources, checks=[],
    boundary='Exact finite reference examples and historical source subtheories. No native engine build, benchmark, model request or general solver verification.')
for i, name in enumerate(SCRIPTS):
    command = [sys.executable, str(PROPOSAL/name)]
    p = subprocess.run(command, text=True, capture_output=True, timeout=180)
    log = f'{i:02}-{Path(name).stem}.log'
    (OUTPUT/log).write_text(p.stdout+'\n'+p.stderr)
    payload = json.loads(p.stdout) if p.returncode == 0 else None
    passed = p.returncode == 0 and isinstance(payload, dict)
    if payload and 'status' in payload:
        passed = passed and payload['status'] == 'passed'
    result['checks'].append(dict(source=name, command=command, exit_code=p.returncode,
        log=log, result=payload, passed=passed))
    (OUTPUT/'check.json').write_text(json.dumps(result, indent=2)+'\n')
    print(name, 'passed' if passed else 'FAILED', flush=True)
    if not passed:
        print(p.stderr[-4000:]); raise SystemExit(1)
assert sources == {name:digest(PROPOSAL/name) for name in SCRIPTS}
result['passed'] = True
(OUTPUT/'check.json').write_text(json.dumps(result, indent=2)+'\n')
