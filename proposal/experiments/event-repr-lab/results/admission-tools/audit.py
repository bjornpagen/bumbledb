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
    ('build-scratch-v1.json','results/scratch-v1-src'),
    ('build-scratch.json','results/scratch-src'),
    ('build-views.json','results/views-src'),
    ('build-views2.json','results/views2-src'),
    ('build-reuse.json','results/reuse-src'),
    ('build-scoped.json','results/scoped-src'),
    ('build-admission.json','results/admission-src'),
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
for name, digest in json.loads((LAB/'results/scratch-reading.json').read_text())['files'].items():
    assert hashlib.sha256((ROOT/name).read_bytes()).hexdigest() == digest, name
for reading in ['view-reading.json', 'view2-reading.json', 'reuse-reading.json', 'scoped-reading.json', 'legal-relations-reading.json']:
    for name, digest in json.loads((LAB/'results'/reading).read_text())['files'].items():
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
    ('scratch-slab-check.json', 'results/scratch-v1-src'),
    ('scratch-enum-check.json', 'results/scratch-v1-src'),
    ('view-shared-enum-check.json', 'results/views-src'),
    ('view-shared-slab-check.json', 'results/views-src'),
    ('view2-shared-enum-check.json', 'results/views2-src'),
    ('view2-shared-slab-check.json', 'results/views2-src'),
    ('reuse-shared-enum-check.json', 'results/reuse-src'),
    ('reuse-shared-slab-check.json', 'results/reuse-src'),
    ('scoped-shared-enum-check.json', 'results/scoped-prebench-src'),
    ('scoped-complete-enum-check.json', 'results/scoped-src'),
    ('scoped-complete-slab-check.json', 'results/scoped-src'),
    ('admission-initial-check.json', 'results/admission-prebench-src'),
    ('admission-complete-enum-check.json', 'results/admission-src'),
    ('admission-complete-slab-check.json', 'results/admission-src'),
]:
    record = json.loads((LAB/'results'/check_name).read_text())
    assert record['passed'], check_name
    for name, digest in record['lab_sources'].items():
        assert hashlib.sha256((LAB/source_dir/Path(name).name).read_bytes()).hexdigest() == digest, (check_name, name)
for prefix, source_dir, checker in [
    ('view', 'results/views-src', 'results/views-tools/view_check.py'),
    ('view2', 'results/views2-src', 'results/views2-tools/view_check.py'),
    ('reuse-source', 'results/reuse-src', 'view_check.py'),
    ('reuse-deferred', 'results/reuse-src', 'view_check.py'),
]:
    for layout in ['enum', 'slab']:
        record = json.loads((LAB/f'results/{prefix}-raw-{layout}-check.json').read_text())
        assert record['passed'] and record['essential_layout'] == layout
        assert (LAB/'results'/record['log']).is_file()
        assert record['checker_sha256'] == hashlib.sha256((LAB/checker).read_bytes()).hexdigest()
        for name, digest in record['source_hashes'].items():
            source = LAB/source_dir/Path(name).name if name.startswith('src/') else LAB/name
            assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
        assert record['cargo_lock_sha256'] == hashlib.sha256((LAB/'essential-prototype/Cargo.lock').read_bytes()).hexdigest()
view_sanity = json.loads((LAB/'results/view2-sanity-check.json').read_text())
assert not view_sanity['passed']
view_failure=(LAB/'results'/view_sanity['log']).read_text()
assert 'test result: FAILED. 8 passed; 1 failed' in view_failure
assert 'scattered_multiword_maps_and_complements_match_materialization ... FAILED' in view_failure
for name,digest in view_sanity['source_hashes'].items():
    source=LAB/'results/views2-sanity-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest()==digest, name
reuse_sanity=json.loads((LAB/'results/reuse-sanity-check.json').read_text())
assert not reuse_sanity['passed']
reuse_failure=(LAB/'results'/reuse_sanity['log']).read_text()
assert 'test result: FAILED. 10 passed; 1 failed' in reuse_failure
assert 'plane_cache_collisions_maps_pins_complements_and_arena_growth ... FAILED' in reuse_failure
for name,digest in reuse_sanity['source_hashes'].items():
    source=LAB/'results/reuse-sanity-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest()==digest,name
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
scratch_classification = json.loads((LAB/'results/scratch-raw-check.json').read_text())
assert scratch_classification['passed']
assert (LAB/'results'/scratch_classification['log']).is_file()
for name, digest in scratch_classification['source_hashes'].items():
    source = LAB/'results/scratch-v1-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest() == digest, name
assert hashlib.sha256((LAB/'classify-prototype/Cargo.lock').read_bytes()).hexdigest() == scratch_classification['cargo_lock_sha256']
axes_reference = json.loads((LAB/'results/axes-reference.json').read_text())
assert axes_reference['passed']
assert axes_reference['checker_sha256'] == hashlib.sha256((LAB/'axes_reference.py').read_bytes()).hexdigest()
for name, digest in axes_reference['source_hashes'].items():
    assert hashlib.sha256((LAB/'results/scratch-src'/Path(name).name).read_bytes()).hexdigest() == digest, name
scratch_smoke = json.loads((LAB/'results/scratch-native-smoke.json').read_text())
assert scratch_smoke['build_metadata'] == json.loads((LAB/'results/build-scratch-v1.json').read_text())
assert all(r['status']=='passed' for r in scratch_smoke['runs'])
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
for revision, expected_count in [('lean-v1',17), ('lean-v2',24), ('lean-v3',29), ('lean-v4',36), ('lean-v5',41), ('lean-v6',44), ('lean-v7',46), ('lean-v8',50), ('lean-v9',54), ('lean-v10',65)]:
    lean_previous = LAB/'results'/revision
    previous = json.loads((lean_previous/'lean-check.json').read_text())
    assert previous['passed']
    assert (lean_previous/previous['log']).is_file()
    assert sum(len(record['checked_axioms']) for record in previous['files']) == expected_count
    assert previous['verifier_sha256'] == hashlib.sha256((lean_previous/'verify_lean.py').read_bytes()).hexdigest()
    for name, digest in previous['source_hashes'].items():
        assert hashlib.sha256((lean_previous/name).read_bytes()).hexdigest() == digest, name
assert sum(len(record['checked_axioms']) for record in lean_check['files']) == 67
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
assert word_comparison['checker_sha256'] == hashlib.sha256((LAB/'results/words-tools/words_comparison.py').read_bytes()).hexdigest()
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

scratch_comparison = json.loads((LAB/'results/scratch-comparison.json').read_text())
assert scratch_comparison['passed']
assert scratch_comparison['binary_sha256'] == json.loads((LAB/'results/build-scratch.json').read_text())['binary_sha256']
assert scratch_comparison['checker_sha256'] == hashlib.sha256((LAB/'scratch_comparison.py').read_bytes()).hexdigest()
assert scratch_comparison['row_checker_sha256'] == hashlib.sha256((LAB/'words_comparison.py').read_bytes()).hexdigest()
from scratch_comparison import collect as collect_scratch
scratch_rows, scratch_processes, scratch_binary, scratch_pairs = collect_scratch(scratch_comparison['inputs'])
assert len(scratch_rows) == scratch_comparison['cases']
assert len(scratch_pairs) == scratch_comparison['matched_kernel_cases']
assert scratch_pairs == scratch_comparison['pairs']
assert scratch_processes == scratch_comparison['processes']
assert all(p['status'] == 'passed' for p in scratch_processes)
for experiment in scratch_comparison['inputs']:
    evidence = json.loads((LAB/'results'/experiment).read_text())
    assert evidence['build_metadata'] == json.loads((LAB/'results/build-scratch.json').read_text())

from view_comparison import collect as collect_views
from view_factorial import collect as collect_factorial
from reuse_comparison import collect as collect_reuse
from scoped_comparison import collect as collect_scoped
from legal_comparison import collect as collect_legal
view_summaries={}
for record_name, build_name, checker_name, collector in [
    ('view-comparison.json', 'build-views.json', 'view_comparison.py', collect_views),
    ('view-factorial.json', 'build-views2.json', 'view_factorial.py', collect_factorial),
    ('reuse-comparison.json', 'build-reuse.json', 'reuse_comparison.py', collect_reuse),
    ('scoped-comparison.json', 'build-scoped.json', 'scoped_comparison.py', collect_scoped),
    ('legal-comparison.json', 'build-admission.json', 'legal_comparison.py', collect_legal),
]:
    report=json.loads((LAB/'results'/record_name).read_text())
    assert report.get('passed', report.get('checked', False))
    assert report['checker_sha256']==hashlib.sha256((LAB/checker_name).read_bytes()).hexdigest()
    rows, processes, binary, pairs, comparison_missing=collector(report['inputs'])
    assert report['cases']==len(rows) and report['matched_cases']==len(pairs)
    assert report['pairs']==pairs and report['processes']==processes and report['missing_baselines']==comparison_missing
    build=json.loads((LAB/'results'/build_name).read_text())
    assert report['binary_sha256']==binary==build['binary_sha256']
    for name in report['inputs']:
        assert json.loads((LAB/'results'/name).read_text())['build_metadata']==build
    view_summaries[record_name]={'cases':len(rows),'matched_cases':len(pairs),'binary_sha256':binary}

view_work=json.loads((LAB/'results/view2-work.json').read_text())
assert view_work['passed']
assert view_work['checker_sha256']==hashlib.sha256((LAB/'view_work.py').read_bytes()).hexdigest()
for name,digest in view_work['source_hashes'].items():
    source=LAB/'results/views2-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest()==digest, name
assert len(view_work['runs'])==4
assert sum(len(r['rows']) for r in view_work['runs'])==96
for run in view_work['runs']:
    assert run['exit_code']==0 and all(row['verified'] for row in run['rows'])
acceptance=json.loads((LAB/'results/view2-acceptance.json').read_text())
assert acceptance['build_metadata']==json.loads((LAB/'results/build-views2.json').read_text())
assert all(r['status']=='passed' for r in acceptance['runs'])
for run in acceptance['runs']:
    for row in run['rows']:
        if row.get('kind')=='owned_free_join':
            assert row['output_roots']==80 and all(n>0 for n in row['novel_publication_classes'])
        if row.get('kind')=='symbolic_relation_free_join':
            assert row['new_classes']>0 and len(row['output_counts'])==5

reuse_acceptance=json.loads((LAB/'results/reuse-acceptance.json').read_text())
assert reuse_acceptance['build_metadata']==json.loads((LAB/'results/build-reuse.json').read_text())
assert all(r['status']=='passed' for r in reuse_acceptance['runs'])
reuse_acceptance_rows={}
for run in reuse_acceptance['runs']:
    assert run['job']['EVENT_LAB_VIEW_NORMALIZE']=='source'
    for row in run['rows']:
        kind=row.get('kind')
        if kind=='owned_free_join':
            assert row['output_roots']==80 and all(n>0 for n in row['novel_publication_classes'])
        elif kind=='symbolic_relation_free_join':
            assert row['new_classes']>0 and len(row['output_counts'])==5
        else:
            continue
        assert row['verified']
        reuse_acceptance_rows[kind]=reuse_acceptance_rows.get(kind,0)+1

scoped_acceptance=json.loads((LAB/'results/scoped-acceptance.json').read_text())
assert scoped_acceptance['build_metadata']==json.loads((LAB/'results/build-scoped.json').read_text())
assert len(scoped_acceptance['runs'])==33 and all(r['status']=='passed' for r in scoped_acceptance['runs'])
scoped_acceptance_rows={}
for run in scoped_acceptance['runs']:
    if run['job'].get('verify') or run['job'].get('EVENT_LAB_CANDIDATE','').startswith('essential'):
        assert run['job']['EVENT_LAB_VIEW_NORMALIZE']=='source'
        assert run['job']['EVENT_LAB_VIEW_REUSE']=='bounded'
    for row in run['rows']:
        kind=row.get('kind')
        if kind=='owned_free_join':
            assert row['output_roots']==80 and all(n>0 for n in row['novel_publication_classes'])
        elif kind=='symbolic_relation_free_join':
            assert row['new_classes']>0 and len(row['output_counts'])==5
            assert row['bits'] in [18,36,60] and row['states']==1<<(row['bits']//3)
        else:
            continue
        assert row['verified']
        scoped_acceptance_rows[kind]=scoped_acceptance_rows.get(kind,0)+1
assert scoped_acceptance_rows=={'owned_free_join':116,'symbolic_relation_free_join':27}

admission_acceptance=json.loads((LAB/'results/admission-acceptance.json').read_text())
assert admission_acceptance['build_metadata']==json.loads((LAB/'results/build-admission.json').read_text())
assert len(admission_acceptance['runs'])==13 and all(r['status']=='passed' for r in admission_acceptance['runs'])
admission_acceptance_rows={}
for run in admission_acceptance['runs']:
    if run['job'].get('verify') or run['job'].get('EVENT_LAB_CANDIDATE','').startswith('essential'):
        assert run['job']['EVENT_LAB_VIEW_NORMALIZE']=='source' and run['job']['EVENT_LAB_VIEW_REUSE']=='bounded'
    for row in run['rows']:
        kind=row.get('kind')
        if kind=='owned_free_join':
            assert row['output_roots']==80 and all(n>0 for n in row['novel_publication_classes'])
        elif kind=='symbolic_relation_free_join':
            assert row['new_classes']>0 and len(row['output_counts'])==5
            assert row['bits'] in [18,36,60] and row['states']==1<<(row['bits']//3)
        else:
            continue
        assert row['verified']
        admission_acceptance_rows[kind]=admission_acceptance_rows.get(kind,0)+1
assert admission_acceptance_rows=={'owned_free_join':34,'symbolic_relation_free_join':15}


reuse_work=json.loads((LAB/'results/reuse-work.json').read_text())
assert reuse_work['passed']
assert reuse_work['checker_sha256']==hashlib.sha256((LAB/'reuse_work.py').read_bytes()).hexdigest()
for name,digest in reuse_work['source_hashes'].items():
    source=LAB/'results/reuse-src'/Path(name).name if name.startswith('src/') else LAB/name
    assert hashlib.sha256(source.read_bytes()).hexdigest()==digest,name
assert len(reuse_work['runs'])==8 and sum(len(r['rows']) for r in reuse_work['runs'])==192
for run in reuse_work['runs']:
    assert run['exit_code']==0 and all(row['verified'] for row in run['rows'])
scoped_reference=json.loads((LAB/'results/scoped-reference.json').read_text())
assert scoped_reference['passed'] and sum(scoped_reference['cases'].values())==157668
assert scoped_reference['checker_sha256']==hashlib.sha256((LAB/'scoped_reference.py').read_bytes()).hexdigest()
assert set(scoped_reference['counterexamples'])=={'missing_witness_gate','missing_output_gate'}
legal_reference=json.loads((LAB/'results/legal-relations-reference.json').read_text())
assert legal_reference['passed'] and legal_reference['cases']['domain_sets']==16
assert legal_reference['checker_sha256']==hashlib.sha256((LAB/'legal_relations_reference.py').read_bytes()).hexdigest()
assert legal_reference['counterexample']['witness']==[0,1,0]
admission_checks={}
for name in ['admission-complete-enum-check.json', 'admission-complete-slab-check.json']:
    record=json.loads((LAB/'results'/name).read_text())
    rows=[json.loads(line.split('EVENT_LAB ',1)[1]) for line in (LAB/'results'/record['log']).read_text().splitlines() if 'EVENT_LAB ' in line]
    finite=[r for r in rows if r.get('kind')=='legal_relation_verification']
    symbolic=[r for r in rows if r.get('kind')=='legal_symbolic_verification']
    assert len(finite)==16 and len(symbolic)==3
    assert all(r['passed'] and r['admission_cases']==765 and r['relation_pairs']==(3456 if r['candidate'].startswith('essential') else 1728) for r in finite)
    assert all(r['passed'] and r['coordinates']==61 and r['population']==((1<<20)-3)**3+((1<<20)-5)**3 for r in symbolic)
    admission_checks[name]={'finite_carriers':16,'symbolic_carriers':3}
assemblies = {}
for tag in ['relations', 'tail', 'dependency', 'transfer-words', 'essential', 'essential-derived', 'classify', 'slab', 'slab-storage', 'words', 'scratch', 'views-v1', 'views-v2', 'reuse', 'scoped', 'admission']:
    assembly = json.loads((LAB/f'results/assembly-{tag}.json').read_text())
    build_tag = {'slab-storage':'slab','views-v1':'views','views-v2':'views2'}.get(tag,tag)
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
out['view_products_verified'] = view_summaries
out['view_work_verified'] = {'cases':96,'runs':4}
out['reuse_work_verified'] = {'cases':192,'runs':8}
out['scoped_reference_verified'] = scoped_reference
out['legal_relations_reference_verified'] = legal_reference
out['legal_admission_shared_checks_verified'] = admission_checks
out['legal_admission_native_acceptance_verified'] = {'processes':13, 'rows':admission_acceptance_rows}
out['view_native_acceptance_verified'] = {'processes':len(acceptance['runs'])}
out['reuse_native_acceptance_verified'] = {'processes':len(reuse_acceptance['runs']), 'rows':reuse_acceptance_rows}
out['scoped_native_acceptance_verified'] = {'processes':len(scoped_acceptance['runs']), 'rows':scoped_acceptance_rows}
out['scratch_classifier_verified'] = {'cases':len(scratch_rows),'matched_cases':len(scratch_pairs),'binary_sha256':scratch_binary}
(LAB/'results/audit.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
