"""Verify the implementation proposal, fresh semantics and retained lab evidence.

--record seals the current revision once; ordinary invocation detects drift.
No Rust/Lean build, benchmark, provider request or download is performed here.
"""
from pathlib import Path
import argparse, datetime, hashlib, json, re, subprocess, sys

HERE = Path(__file__).resolve().parent
PROPOSAL = HERE.parent
ROOT = PROPOSAL.parent
LAB = PROPOSAL / 'experiments/event-repr-lab'
RESULTS = HERE / 'results'
RECORD = RESULTS / 'implementation-ready.json'

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def read(path):
    return json.loads(path.read_text())

def documents():
    return sorted(set(PROPOSAL.glob('*.md')) | set((PROPOSAL/'coup').glob('*.md')) |
        set((PROPOSAL/'research').glob('*.md')) | set(LAB.glob('*.md')) | set(HERE.glob('*.md')))

def check_proofs():
    folder = RESULTS / 'readiness'
    data = read(folder/'check.json')
    assert data['passed'] and data['scope'] == 'central-plus-pack-plus-public-contract'
    assert data['reports'] == 246 and data['axiom_free'] == 160 and len(data['files']) == 30
    assert digest(Path(data['lean_binary'])) == data['lean_binary_sha256']
    assert digest(HERE/'check.py') == digest(folder/'check.py') == data['checker_sha256']
    for name, sha in data['source_sha256'].items():
        assert digest(PROPOSAL/name) == digest(folder/'sources'/name) == sha, name
    count = free = 0
    all_names = set()
    for row in data['files']:
        assert row['passed'] and row['exit_code'] == 0
        assert set(row['expected']) == set(row['axioms'])
        log = (folder/row['log']).read_text()
        assert 'error:' not in log and 'sorryAx' not in log
        for name, axioms in row['axioms'].items():
            assert name not in all_names
            all_names.add(name)
            assert set(axioms) <= {'propext','Quot.sound','Classical.choice'}
            if not axioms:
                assert f"'{name}' does not depend on any axioms" in log
            else:
                m = re.search(re.escape(f"'{name}' depends on axioms: [")+r'([^\]]*)\]',log)
                assert m and [x.strip() for x in m.group(1).split(',')] == axioms
            count += 1; free += not axioms
    assert (count, free) == (246, 160)
    return dict(reports=count, axiom_free=free, files=len(data['files']))

def check_references():
    folder = RESULTS/'reference-checks'
    data = read(folder/'check.json')
    assert data['passed'] and len(data['checks']) == 7
    assert digest(HERE/'reference_checks.py') == digest(folder/'reference_checks.py') == data['checker_sha256']
    for name, sha in data['source_sha256'].items():
        assert digest(PROPOSAL/name) == digest(folder/'sources'/name) == sha
    for row in data['checks']:
        assert row['passed'] and row['exit_code'] == 0
        assert json.loads((folder/row['log']).read_text()) == row['result']
    return dict(suites=7, scope='exact finite reference examples and historical source subtheories')

def check_documents():
    paths = documents()
    for path in paths:
        source = path.read_text()
        assert sum(line.startswith('```') for line in source.splitlines()) % 2 == 0, path
        prose = re.sub(r'(?ms)^```.*?^```[^\n]*', '', source)
        prose = re.sub(r'`[^`\n]+`', '', prose)
        for link in re.findall(r'\]\(([^)]+)\)', prose):
            link = link.strip('<>')
            if link.startswith(('http:','https:','#','mailto:','app:')):
                continue
            link = re.sub(r':\d+$', '', link.split('#')[0])
            if link:
                target = Path(link) if Path(link).is_absolute() else path.parent/link
                assert target.exists(), (str(path.relative_to(PROPOSAL)), link)
    for name, phrase in [('event-algebra.md','Existing two-word EventKey'),
            ('query-algebra.md','Revision 0.5 retains the'),
            ('first-principles.md','Its shared space owns a reduced decision diagram')]:
        assert phrase not in (PROPOSAL/name).read_text(), name
    plan = (PROPOSAL/'implementation-plan.md').read_text()
    for i in range(9):
        assert f'## M{i} ' in plan
    archive = PROPOSAL/'archive/implementation-prep-0.7'
    for name, sha in read(archive/'manifest.json')['files'].items():
        assert digest(archive/name) == sha
    old = read(archive/'research-closeout.json')
    for name, sha in old['documents_sha256'].items():
        assert digest(archive/Path(name).relative_to('proposal')) == sha
    return {str(path.relative_to(PROPOSAL)):digest(path) for path in paths}

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--record', action='store_true')
    args = ap.parse_args()
    if args.record:
        assert not RECORD.exists(), 'Preserve the existing readiness record.'
        RECORD.write_text(json.dumps(dict(status='pending_verification'))+'\n')
    else:
        old = read(RECORD)
        assert old['passed']
        for name, sha in old['documents_sha256'].items():
            assert digest(PROPOSAL/name) == sha, name
        for name, sha in old['evidence_sha256'].items():
            assert digest(PROPOSAL/name) == sha, name
    proof = check_proofs()
    references = check_references()
    docs = check_documents()
    p = subprocess.run([sys.executable, str(LAB/'audit.py')],cwd=LAB,text=True,capture_output=True,timeout=180)
    assert p.returncode == 0, p.stderr[-6000:]
    lab = read(LAB/'results/audit.json')
    assert lab['production_source_hashes_match'] and not lab['missing_local_links'] and not lab['unbalanced_code_fences']
    assert lab['relation_pack_round_verified']['passed']
    assert not (LAB/'.scratch/participation-engine').exists()
    shelved = read(LAB/'results/participation-shelved.json')
    for name, sha in {**shelved['source_sha256'], **shelved['helper_sha256']}.items():
        assert digest(LAB/name) == sha
    if args.record:
        (RESULTS/'lab-audit.json').write_text(json.dumps(lab,indent=2)+'\n')
        (RESULTS/'lab-audit.log').write_text(p.stdout+'\n'+p.stderr)
        evidence = [HERE/'check.py', HERE/'reference_checks.py', Path(__file__),
            RESULTS/'lab-audit.json', RESULTS/'lab-audit.log',
            LAB/'results/participation-shelved.json', LAB/'audit.py']
        for folder in [RESULTS/'readiness',RESULTS/'reference-checks']:
            evidence += sorted(path for path in folder.rglob('*') if path.is_file())
        out = dict(passed=True, revision='0.8', baseline='v1.3.1',
            verified_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
            documents_checked=len(docs), documents_sha256=docs,
            proofs=proof, references=references,
            production_source_hashes_match=True, shelved_draft_excluded=True,
            evidence_sha256={str(path.relative_to(PROPOSAL)):digest(path) for path in evidence},
            boundary='Ready for implementation against M0–M8 gates. Native Event integration, canonical persistent codec, general source solver and fixed-point implementation refinement remain unimplemented obligations.')
        RECORD.write_text(json.dumps(out,indent=2)+'\n')
    else:
        assert docs == old['documents_sha256']
        assert lab == read(RESULTS/'lab-audit.json')
    print(json.dumps(dict(passed=True, documents=len(docs), proofs=proof, references=references,
        production_source_hashes_match=True, native_event_implemented=False),indent=2))

if __name__ == '__main__':
    main()
