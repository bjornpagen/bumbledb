"""Check retained artifacts and original-source isolation, without running benchmarks."""
from pathlib import Path
import hashlib, json, re
from prepare import LAB, ROOT
from run import check_engine_sources

check_engine_sources()
snapshots = []
for metadata_name, source_dir in [
    ('build-baseline.json','results/baseline-src'),
    ('build-fastpaths.json','results/fastpaths-src'),
    ('build-anchored.json','results/anchored-src'),
    ('build-product.json','results/product-src'),
    ('build-dispatched.json','results/dispatched-src'),
    ('build-packed.json','results/packed-src'),
    ('build-relations-v1.json','results/relations-v1-src'),
    ('build-relations.json','results/relations-src'),
    ('build-block-v1.json','results/block-v1-src'),
    ('build-tail.json','results/tail-src'),
    ('build-observation.json','results/observation-src'),
    ('build-dependency.json','results/dependency-src'),
    ('build-transfer.json','results/transfer-src'),
    ('build-transfer-tables.json','results/transfer-tables-src'),
    ('build-transfer-words.json','results/transfer-words-src'),
    ('build-owned.json','results/owned-src'),
    ('build-diagonal.json','results/diagonal-src'),
    ('build-essential.json','results/essential-src'),
    ('build-essential-derived.json','results/essential-derived-src'),
    ('build-classify-v1.json','results/classify-v1-src'),
    ('build-classify.json','results/classify-src'),
    ('build-slab.json','results/slab-src'),
    ('build-words.json','results/words-src'),
    ('build.json','src'),
]:
    metadata = json.loads((LAB/'results'/metadata_name).read_text())
    for name, expected in metadata['lab_sources'].items():
        source = LAB/source_dir/Path(name).name
        assert hashlib.sha256(source.read_bytes()).hexdigest() == expected, source
    snapshots.append(metadata_name)

missing = []
bad_fences = []
docs = sorted(set((ROOT/'proposal').glob('*.md')) | set((ROOT/'proposal/coup').glob('*.md')) | set((ROOT/'proposal/research').glob('*.md')) | set(LAB.glob('*.md')))
for doc in docs:
    source = doc.read_text()
    if sum(line.startswith('```') for line in source.splitlines()) % 2: bad_fences.append(str(doc))
    rendered = re.sub(r'(?ms)^```.*?^```[^\n]*','',source)
    rendered = re.sub(r'`[^`\n]+`','',rendered)
    for link in re.findall(r'\]\(([^)]+)\)',rendered):
        link = link.strip('<>')
        if link.startswith(('http:','https:','#','mailto:','app:')): continue
        link = link.split('#')[0]
        if not link: continue
        path = Path(re.sub(r':\d+$','',link))
        target = path if path.is_absolute() else doc.parent/path
        if not target.exists(): missing.append([str(doc.relative_to(ROOT)),link])
assert not missing, missing
assert not bad_fences, bad_fences

research_sources = []
for source in json.loads((LAB/'results/observation-reading.json').read_text()):
    assert hashlib.sha256((ROOT/source['file']).read_bytes()).hexdigest() == source['sha256']
    research_sources.append(source['arxiv'])
for name, digest in json.loads((LAB/'results/dependency-reading.json').read_text())['files'].items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest, name
for folder in ['representation-search', 'block-decomposition', 'space-maps']:
    research_dir = ROOT/'proposal/research'/folder
    for source in json.loads((research_dir/'sources.json').read_text()):
        for name, digest in [(source['file'],source['sha256']), (source['text_file'],source['text_sha256'])]:
            assert hashlib.sha256((research_dir/name).read_bytes()).hexdigest() == digest, name
        research_sources.append(source['arxiv'])

map_check = json.loads((LAB/'results/space-map-check.json').read_text())
for name, digest in map_check['files'].items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest, name
assert json.loads((ROOT/'proposal/research/space-maps/checks.json').read_text())['passed']
dependency_queries = json.loads((LAB/'results/dependency-query-checks.json').read_text())
assert dependency_queries['passed']
assert dependency_queries['checker_sha256'] == hashlib.sha256((LAB/'dependency_queries.py').read_bytes()).hexdigest()
map_graphs = json.loads((ROOT/'proposal/research/space-maps/graphs.json').read_text())
assert map_graphs['passed']
assert map_graphs['checker_sha256'] == hashlib.sha256((ROOT/'proposal/research/space-maps/graphs.py').read_bytes()).hexdigest()
reach_checks = json.loads((ROOT/'proposal/research/block-decomposition/reach-checks.json').read_text())
assert reach_checks['passed']
assert reach_checks['checker_sha256'] == hashlib.sha256((ROOT/'proposal/research/block-decomposition/reach_checks.py').read_bytes()).hexdigest()
assert reach_checks['paper_sha256'] == hashlib.sha256((ROOT/'proposal/research/block-decomposition/decision-diagram-reachability.pdf').read_bytes()).hexdigest()
reach_schedules = json.loads((ROOT/'proposal/research/block-decomposition/reach-schedules.json').read_text())
assert reach_schedules['passed']
assert reach_schedules['checker_sha256'] == hashlib.sha256((ROOT/'proposal/research/block-decomposition/reach_schedules.py').read_bytes()).hexdigest()
assert reach_schedules['paper_sha256'] == reach_checks['paper_sha256']
essential_reference = json.loads((LAB/'results/essential-reference.json').read_text())
assert essential_reference['passed']
assert essential_reference['checker_sha256'] == hashlib.sha256((LAB/'essential_reference.py').read_bytes()).hexdigest()
essential_rust = json.loads((LAB/'results/essential-rust-check.json').read_text())
assert essential_rust['passed']
assert (LAB/'results'/essential_rust['log']).is_file()
for name, digest in essential_rust['source_hashes'].items():
    source = LAB/'results/classify-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
for check_name, source_dir in [
    ('essential-scalar-check.json', 'results/essential-src'),
    ('essential-derived-check.json', 'results/essential-derived-src'),
    ('classify-complete-check.json', 'results/classify-v1-src'),
    ('classify-owned-check.json', 'results/classify-src'),
    ('slab-sanity-check.json', 'results/slab-sanity-src'),
    ('slab-enum-check.json', 'results/slab-src'),
    ('slab-complete-check.json', 'results/slab-src'),
    ('words-slab-check.json', 'results/words-src'),
    ('words-enum-check.json', 'results/words-src'),
]:
    record = json.loads((LAB/'results'/check_name).read_text())
    assert record['passed'], check_name
    for name, digest in record['lab_sources'].items():
        assert hashlib.sha256((LAB/source_dir/Path(name).name).read_bytes()).hexdigest() == digest, (check_name, name)
essential_baseline = json.loads((LAB/'results/essential-initial-sweep.json').read_text())
assert essential_baseline['build_metadata'] == json.loads((LAB/'results/build-essential.json').read_text())
assert all(run['status'] == 'passed' for run in essential_baseline['runs'])
for profile_name in ['essential-baseline-profile.json', 'essential-derived-profile.json']:
    profile = json.loads((LAB/'results'/profile_name).read_text())
    assert profile['process_exit'] == profile['sample_exit'] == 0
    assert profile['binary_sha256'] == json.loads((LAB/'results'/profile['build']).read_text())['binary_sha256']
    assert hashlib.sha256((LAB/'results'/profile['profile']).read_bytes()).hexdigest() == profile['sha256']
for experiment in ['essential-derived-sweep.json', 'essential-derived-repeat.json']:
    evidence = json.loads((LAB/'results'/experiment).read_text())
    assert evidence['build_metadata'] == json.loads((LAB/'results/build-essential-derived.json').read_text())
    assert all(run['status'] == 'passed' for run in evidence['runs'])
classification = json.loads((LAB/'results/classify-check.json').read_text())
assert classification['passed']
assert (LAB/'results'/classification['log']).is_file()
for name, digest in classification['source_hashes'].items():
    source = LAB/'results/classify-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
slab_classification = json.loads((LAB/'results/slab-classify-check.json').read_text())
assert slab_classification['passed']
for name, digest in slab_classification['source_hashes'].items():
    source = LAB/'results/slab-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
assert hashlib.sha256((LAB/'classify-prototype/Cargo.lock').read_bytes()).hexdigest() == classification['cargo_lock_sha256']
word_classification = json.loads((LAB/'results/words-raw-check.json').read_text())
assert word_classification['passed']
assert (LAB/'results'/word_classification['log']).is_file()
for name, digest in word_classification['source_hashes'].items():
    source = LAB/'results/words-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
assert hashlib.sha256((LAB/'classify-prototype/Cargo.lock').read_bytes()).hexdigest() == word_classification['cargo_lock_sha256']
signature_calculus = json.loads((LAB/'results/signature-calculus-check.json').read_text())
assert signature_calculus['passed']
assert hashlib.sha256((LAB/'signature_calculus.py').read_bytes()).hexdigest() == signature_calculus['checker_sha256']
for name, digest in signature_calculus['reading']['files'].items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest, name
lean_check = json.loads((LAB/'results/lean-check.json').read_text())
assert lean_check['passed']
assert (LAB/'results'/lean_check['log']).is_file()
assert hashlib.sha256((LAB/'verify_lean.py').read_bytes()).hexdigest() == lean_check['verifier_sha256']
for name, digest in lean_check['source_hashes'].items():
    assert hashlib.sha256((LAB/name).read_bytes()).hexdigest() == digest, name
for revision, expected_count in [('lean-v1',17), ('lean-v2',24)]:
    lean_previous = LAB/'results'/revision
    previous = json.loads((lean_previous/'lean-check.json').read_text())
    assert previous['passed']
    assert (lean_previous/previous['log']).is_file()
    assert sum(len(record['checked_axioms']) for record in previous['files']) == expected_count
    assert previous['verifier_sha256'] == hashlib.sha256((lean_previous/'verify_lean.py').read_bytes()).hexdigest()
    for name, digest in previous['source_hashes'].items():
        assert hashlib.sha256((lean_previous/name).read_bytes()).hexdigest() == digest, name
assert sum(len(record['checked_axioms']) for record in lean_check['files']) == 29
for checked in lean_check['files']:
    assert checked['passed'] and checked['exit_code'] == 0
    assert all(set(axioms) <= {'propext','Quot.sound','Classical.choice'} for axioms in checked['checked_axioms'].values())
for experiment in ['classify-smoke-fixed.json', 'classify-initial-sweep.json', 'classify-focused-repeat.json', 'classify-essential-repeat.json']:
    evidence = json.loads((LAB/'results'/experiment).read_text())
    assert evidence['build_metadata'] == json.loads((LAB/'results/build-classify.json').read_text())
    assert all(run['status'] == 'passed' for run in evidence['runs'])
    for run in evidence['runs']:
        for row in run['rows']:
            if row.get('kind') != 'signature_free_join': continue
            assert row['histogram'][0] == 0 and sum(row['histogram']) == row['rows']
            assert 0 < row['accepted'] < row['rows'] and row['verified']
            if row['classifier'] == 'direct':
                assert row['input_nodes'] == row['final_nodes']
                assert row['input_bytes_est'] == row['final_bytes_est']

slab_comparison = json.loads((LAB/'results/slab-comparison.json').read_text())
assert slab_comparison['passed']
assert slab_comparison['binary_sha256'] == json.loads((LAB/'results/build-slab.json').read_text())['binary_sha256']
assert slab_comparison['checker_sha256'] == hashlib.sha256((LAB/'results/slab-tools/slab_comparison.py').read_bytes()).hexdigest()
from slab_comparison import collect
slab_rows, slab_processes, slab_binary, slab_pairs = collect(slab_comparison['inputs'])
assert len(slab_rows) == slab_comparison['cases']
assert slab_pairs == slab_comparison['pairs']
assert slab_processes == slab_comparison['processes']
assert all(p['status'] == 'passed' for p in slab_processes)
for experiment in slab_comparison['inputs']:
    evidence = json.loads((LAB/'results'/experiment).read_text())
    assert evidence['build_metadata'] == json.loads((LAB/'results/build-slab.json').read_text())

word_comparison = json.loads((LAB/'results/word-classifier-comparison.json').read_text())
assert word_comparison['passed']
assert word_comparison['binary_sha256'] == json.loads((LAB/'results/build-words.json').read_text())['binary_sha256']
assert word_comparison['checker_sha256'] == hashlib.sha256((LAB/'words_comparison.py').read_bytes()).hexdigest()
from words_comparison import collect as collect_words
word_rows, word_processes, word_binary, word_pairs = collect_words(word_comparison['inputs'])
assert len(word_rows) == word_comparison['cases']
assert len(word_pairs) == word_comparison['matched_kernel_cases']
assert word_pairs == word_comparison['pairs']
assert word_processes == word_comparison['processes']
assert all(p['status'] == 'passed' for p in word_processes)
for experiment in word_comparison['inputs']:
    evidence = json.loads((LAB/'results'/experiment).read_text())
    assert evidence['build_metadata'] == json.loads((LAB/'results/build-words.json').read_text())

assemblies = {}
for tag in ['relations', 'tail', 'dependency', 'transfer-words', 'essential', 'essential-derived', 'classify', 'slab', 'slab-storage', 'words']:
    assembly = json.loads((LAB/f'results/assembly-{tag}.json').read_text())
    build_tag = 'slab' if tag == 'slab-storage' else tag
    assert assembly['binary_sha256'] == json.loads((LAB/f'results/build-{build_tag}.json').read_text())['binary_sha256']
    for symbol in assembly['symbols']:
        assert hashlib.sha256((LAB/'results'/symbol['assembly']).read_bytes()).hexdigest() == symbol['sha256']
    assemblies[tag] = len(assembly['symbols'])
regressions = json.loads((LAB/'results/regressions.json').read_text())
for regression in regressions:
    assert (LAB/'results'/regression['failure_log']).exists()
    assert (LAB/'results'/regression['source_snapshot']).is_dir()
    assert all(run['status']=='passed' for run in json.loads((LAB/'results'/regression['verified_by']).read_text())['runs'])

runs = {}
shared_checks = {}
for path in sorted((LAB/'results').glob('*.json')):
    data = json.loads(path.read_text())
    if isinstance(data, dict) and 'shared_modules' in data:
        assert (LAB/'results'/data['log']).is_file(), data['log']
        shared_checks[path.name] = {'passed':data['passed'], 'profile':data['profile']}
    if 'runs' not in data: continue
    statuses = {}
    for record in data['runs']:
        assert (LAB/'results'/record['log']).exists(),record['log']
        statuses[record['status']] = statuses.get(record['status'],0) + 1
        for row in record['rows']:
            assert row.get('verified',row.get('passed',False)),row
    runs[path.name] = statuses
out = {'production_source_hashes_match':True,'source_snapshots_verified':snapshots,'documents_checked':len(docs),'missing_local_links':missing,'unbalanced_code_fences':bad_fences,'run_statuses':runs,'shared_check_logs_present':shared_checks,'representation_research_sources_verified':research_sources,'space_map_check_verified':map_check,'assembly_symbols_verified':assemblies,'regression_fixes_verified':len(regressions)}
out['backward_reach_check_verified'] = reach_checks
out['backward_reach_schedules_verified'] = reach_schedules
out['essential_reference_verified'] = essential_reference
out['essential_rust_verified'] = essential_rust
out['classification_verified'] = classification
out['signature_calculus_verified'] = signature_calculus
out['lean_proofs_verified'] = lean_check
out['slab_layouts_verified'] = {'cases':len(slab_rows),'matched_cases':len(slab_pairs),'binary_sha256':slab_binary}
out['word_classifier_verified'] = {'cases':len(word_rows),'matched_cases':len(word_pairs),'binary_sha256':word_binary}
(LAB/'results/audit.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
