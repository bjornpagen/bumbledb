"""Check Event proofs with the pinned installed Lean; no downloads or native_decide."""
from pathlib import Path
import argparse
import hashlib
import json
import re
import subprocess

LAB = Path(__file__).resolve().parent
PROOFS = LAB/'lean'
ap = argparse.ArgumentParser()
ap.add_argument('--output', default='lean-check.json')
args = ap.parse_args()
toolchain = (PROOFS/'lean-toolchain').read_text().strip()
installed = subprocess.check_output(['elan', 'toolchain', 'list'], text=True)
assert toolchain in [line.split()[0] for line in installed.splitlines() if line.strip()], (
    'The pinned Lean toolchain must already be installed: '+toolchain)
binary = Path(subprocess.check_output(['elan', 'which', 'lean'], cwd=PROOFS, text=True).strip())
version = subprocess.check_output([str(binary), '--version'], text=True).strip()
assert 'version '+toolchain.split(':v')[1]+',' in version, version
expected = {
    'EventSignatures.lean': [
        'possible_apply_iff', 'full_apply_iff', 'signature_nonempty', 'swap',
        'complement_left', 'complement_right', 'signature_iff_all_binary_tests',
        'composition_distinguishes_same_signature', 'no_signature_factorization'],
    'EssentialCoordinates.lean': [
        'essential_sufficient', 'essential_minimum', 'support_iff_contains_essential',
        'complement_essential', 'canonical_projection', 'cofactor_characterization',
        'quantified_coordinate_inessential', 'cofactor_essential_subset',
        'cofactor_can_remove_other_dependence'],
    'SupportedDependence.lean': [
        'product_support_rectangular', 'intersection_when_rectangular',
        'copied_has_incomparable_dependencies', 'no_least_supported_dependency'],
    'Anchored.lean': [
        'root_excludes_anchor', 'root_excludes_outside', 'decode_root',
        'complement_same_root', 'complement_flips_polarity', 'exact_identity',
        'adjusted_zero', 'binary_apply_root'],
    'ReadoutMaps.lean': [
        'possibility_iff_exact_support_image', 'occupancy_preserved',
        'equality_reflected', 'relative_complement_preserved',
        'fixing_coordinate_can_remove_possibility'],
    'BaseChange.lean': [
        'relational_certificate_iff',
        'automatic_inclusion', 'exact_iff_complete_fibres', 'universal_base_change',
        'nonvacuous_base_change',
        'copied_bit_counterexample', 'complete_fibres_need_not_have_unique_lifts'],
    'ViewProduct.lean': [
        'contraction_through_image', 'contraction_exact_iff_image',
        'product_is_composition', 'separate_surjectivity_is_insufficient',
        'same_roots_different_maps', 'output_bijection', 'joint_reindex_with_section', 'shannon_merge'],
}
files = [PROOFS/name for name in expected]+[PROOFS/'lean-toolchain']
hashes = {str(p.relative_to(LAB)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
records = []
logs = []
for name, theorems in expected.items():
    command = [str(binary), name]
    process = subprocess.run(command, cwd=PROOFS, text=True, capture_output=True)
    log = process.stdout+process.stderr
    logs.append(name+'\n'+log)
    axioms = {}
    for theorem in theorems:
        full = Path(name).stem+'.'+theorem
        empty = "'"+full+"' does not depend on any axioms"
        match = re.search(re.escape("'"+full+"' depends on axioms: [")+r'([^\]]*)\]', log)
        if empty in log:
            axioms[full] = []
        elif match:
            axioms[full] = [s.strip() for s in match.group(1).split(',')]
    permitted = {'propext', 'Quot.sound', 'Classical.choice'}
    passed = (process.returncode == 0 and len(axioms) == len(theorems)
              and all(set(a) <= permitted for a in axioms.values()))
    records.append(dict(file=name, command=command, exit_code=process.returncode,
                        checked_axioms=axioms, passed=passed))
assert hashes == {str(p.relative_to(LAB)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files}, 'Proofs changed during checking'
log_name = Path(args.output).stem+'.log'
(LAB/'results'/log_name).write_text('\n'.join(logs))
result = dict(passed=all(r['passed'] for r in records), toolchain=toolchain,
              lean_version=version, lean_binary=str(binary),
              lean_binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
              source_hashes=hashes, log=log_name, files=records,
              verifier_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              boundary='Kernel-checked denotational theorems. Rust correspondence, arena canonicality and machine code are not formally verified.')
(LAB/'results'/args.output).write_text(json.dumps(result, indent=2)+'\n')
print('\n'.join(logs))
print('Lean proof check:', 'passed' if result['passed'] else 'FAILED')
raise SystemExit(0 if result['passed'] else 1)
