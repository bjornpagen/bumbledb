"""Exercise the close-race regression, with the missing-wakeup negative control.
Run only with an idle build tree: temporarily edits one helper and always restores it.
"""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
output = Path(__file__).resolve().parent
source = root / 'ts/crate/src/runtime.rs'
tests = root / 'ts/crate/src/runtime/tests.rs'
original = source.read_bytes()
sha = lambda data: hashlib.sha256(data).hexdigest()
start = original.index(b'    fn reclaim_cancelled_operations(')
end = original.index(b'    fn supervise(', start)
helper = original[start:end]
assert helper.count(b'self.changed.notify_all();') == 1
mutant = original[:start] + helper.replace(b'self.changed.notify_all();', b'// Negative control: omit the registry-change wakeup.') + original[end:]
command = ['cargo', 'test', '--manifest-path', 'ts/crate/Cargo.toml', '--lib',
           'runtime::tests::cancellation_reclamation_wakes_waiters_for_success_and_error_slots', '--', '--exact']
result = {'runtime_sha256': sha(original), 'test_sha256': sha(tests.read_bytes()),
          'checker_sha256': sha(Path(__file__).read_bytes()), 'command': command, 'checks': []}
try:
    source.write_bytes(mutant)
    run = subprocess.run(command, cwd=root, text=True, capture_output=True, timeout=120)
    log = run.stdout + run.stderr
    (output / 'without-wakeup.log').write_text(log)
    result['checks'].append({'name': 'without-wakeup', 'exit_code': run.returncode,
        'expected_regression': run.returncode != 0 and 'registry removal must wake cleanup without another job' in log,
        'log_sha256': sha(log.encode()), 'mutant_sha256': sha(mutant)})
finally:
    source.write_bytes(original)
run = subprocess.run(command, cwd=root, text=True, capture_output=True, timeout=120)
log = run.stdout + run.stderr
(output / 'with-wakeup.log').write_text(log)
result['checks'].append({'name': 'with-wakeup', 'exit_code': run.returncode, 'log_sha256': sha(log.encode())})
result['source_restored'] = source.read_bytes() == original
result['passed'] = result['checks'][0]['expected_regression'] and run.returncode == 0 and result['source_restored']
(output / 'check.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result, indent=2))
raise SystemExit(0 if result['passed'] else 1)
