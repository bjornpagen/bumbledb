"""Inspect actual compiled candidate operations; do not run beside timing sweeps."""
from pathlib import Path
import argparse,hashlib,json,re,subprocess
from run import RESULTS

ap=argparse.ArgumentParser();ap.add_argument('--tag',required=True);ap.add_argument('--scope',choices=['regions','observation','transport','essential','classify','storage','views','scoped','admission'],default='regions');args=ap.parse_args()
assert re.fullmatch(r'[a-z0-9-]+',args.tag), 'Use a simple lowercase artifact tag'
assert not (RESULTS/('assembly-'+args.tag+'.json')).exists(), 'Preserve existing assembly metadata'
metadata=json.loads((RESULTS/'build.json').read_text())
binary=Path(metadata['binary'])
assert hashlib.sha256(binary.read_bytes()).hexdigest()==metadata['binary_sha256']
symbols=subprocess.check_output(['nm','-n',str(binary)],text=True).splitlines()
records=[]
selected = [
    ('packed64-apply', ['6packed','6PackedKj1_','6raw_op']),
    ('packed256-apply', ['6packed','6PackedKj4_','6raw_op']),
    ('packed256-permute-table', ['6packed','6PackedKj4_','13permute_table']),
    ('packed4096-apply', ['6packed','6PackedKj40_','6raw_op']),
    ('packed4096-permute-rec', ['6packed','6PackedKj40_','11permute_rec']),
    ('block64-apply', ['6blocks','7Block64','6raw_op']),
    ('block64-product', ['6blocks','7Block64','11product_rec']),
    ('dense-permutation', ['11Permutation5dense']),
]
if args.scope == 'observation':
    selected = [('observation-table-contract', ['11observation', '9TablePlan8contract'])]
if args.scope == 'transport':
    selected = [('packed512-build-physical', ['6packed','6PackedKj8_','20build_physical_words']),
                ('dense-permutation', ['11Permutation5dense'])]
if args.scope == 'essential':
    selected = [
        ('essential64-apply', ['13essential_raw','5ArenaKm6_','5applyB']),
        ('essential512-apply', ['13essential_raw','5ArenaKm9_','5applyB']),
        ('essential512-table', ['13essential_raw','5ArenaKm9_','5tableB']),
        ('essential512-cofactor', ['13essential_raw','5ArenaKm9_','8cofactorB']),
        *[('essential-words-'+name, ['13essential_raw','12word_kernels',name])
          for name in ['irrelevant','combine','permute','cofactor']],
    ]
if args.scope == 'classify':
    # The allocated-word revision used a Boolean mode. The scratch revision
    # has an explicit u8 mode and compile-time workspace size. Select from
    # actual emitted symbols so old retained binaries remain inspectable.
    scratch_revision = any('classify_with5visitKm6_Kh0_Kj0_' in s for s in symbols)
    def visit(k, mode):
        suffix = f'Kh{mode}_Kj0_' if scratch_revision else f'Kb{mode}_'
        return f'9occupancy13classify_with5visitKm{k}_{suffix}'
    selected = [
        ('signature-word-occupancy', ['9signature14word_occupancy']),
        ('essential64-occupancy-scalar', [visit(6,0)]),
        ('essential64-occupancy-words', [visit(6,1)]),
        ('essential512-occupancy-scalar', [visit(9,0)]),
        ('essential512-occupancy-words', [visit(9,1)]),
        ('packed64-occupancy', ['6packed','6PackedKj1_E9occupancy']),
        ('packed512-occupancy', ['6packed','6PackedKj8_E9occupancy']),
        ('essential-words-broadcast', ['13essential_raw12word_kernels9broadcast']),
        ('essential-words-cofactor', ['13essential_raw12word_kernels8cofactor']),
    ]
    if scratch_revision:
        selected += [
            ('essential64-occupancy-scratch', ['9occupancy13classify_with5visitKm6_Kh2_Kj1_']),
            ('essential512-occupancy-scratch', ['9occupancy13classify_with5visitKm9_Kh2_Kj8_']),
            ('essential64-occupancy-dispatch', ['9occupancy13classify_withKm6_E']),
            ('essential512-occupancy-dispatch', ['9occupancy13classify_withKm9_E']),
            ('essential-scratch-cofactor', ['9occupancy7scratch13cofactor_into']),
            ('essential-scratch-broadcast', ['9occupancy7scratch14broadcast_into']),
        ]
if args.scope == 'storage':
    selected = [
        ('essential-store-node', ['13essential_raw7storage', '5Store4node']),
        ('essential-store-find', ['13essential_raw7storage', '5Store4find']),
        ('essential-store-insert', ['13essential_raw7storage', '5Store6insert']),
        ('essential-raw-evaluate', ['13essential_raw', '5ArenaKm1_E8evaluate']),
    ]
if args.scope in ['views', 'scoped', 'admission']:
    selected = [(f'essential{k}-mapped-visit', ['12view_product',f'7Product5visitKm{v}_'])
                for k,v in [(64,6),(512,9)]]
    entry = '23mapped_product_observed' if any('5ArenaKm6_E23mapped_product_observed' in line for line in symbols) else '14mapped_product'
    selected += [(f'essential{k}-mapped-entry', ['12view_product',f'5ArenaKm{v}_',entry])
                 for k,v in [(64,6),(512,9)]]
    selected += [('essential-mapped-plane', ['12view_product5planeKm1_'])]
    # LLVM may merge cutoff-independent helper instantiations. Inspect the
    # emitted shared helper, without claiming its mangled cutoff is a policy.
    if any('10PlaneCache7prepare' in line for line in symbols):
        selected += [
            ('essential-mapped-cache-prepare', ['12view_product','10PlaneCache7prepare']),
            ('essential-mapped-cache-read', ['12view_product','10PlaneCache4read']),
            ('essential-mapped-source-normalize', ['12view_product','10Restricted12canonicalize']),
        ]
    if args.scope in ['scoped', 'admission']:
        selected += [(f'essential{k}-scoped-product', ['9essential', f'9EssentialKm{v}_', '12view_product'])
                     for k,v in [(64,6),(512,9)]]
if args.scope == 'admission':
    selected += [(f'essential{k}-preserving-product', ['9essential', f'9EssentialKm{v}_', '18preserving_product']) for k,v in [(64,6),(512,9)]]
for label, parts in selected:
    matches=[line.split()[-1] for line in symbols if 'event_repr_lab' in line and all(p in line for p in parts)]
    assert len(matches)==1,(label,matches)
    assert not (RESULTS/(label+'-'+args.tag+'-arm64.s')).exists(),label
for label, parts in selected:
    matches=[line.split()[-1] for line in symbols if 'event_repr_lab' in line and all(p in line for p in parts)]
    assert len(matches)==1,(label,matches)
    cmd=['xcrun','llvm-objdump','--disassemble-symbols='+matches[0],'--no-show-raw-insn',str(binary)]
    assembly=subprocess.check_output(cmd,text=True)
    assert len(assembly.splitlines())>20, label
    name=label+'-'+args.tag+'-arm64.s'
    assert not (RESULTS/name).exists(), name
    (RESULTS/name).write_text(assembly)
    instructions=[line.strip() for line in assembly.splitlines() if re.search(r'\b(?:and|bic|orr|eor|bsl)\.16b\b',line)]
    counts=[line.strip() for line in assembly.splitlines() if re.search(r'\b(?:cnt|uaddlv|addv)(?:\.[a-z0-9]+)?\b',line)]
    vector_memory=[line.strip() for line in assembly.splitlines() if re.search(r'\b(?:ldp|stp|ldr|str)\s+q\d',line)]
    record={'label':label,'symbol':matches[0],'command':cmd,'assembly':name,'sha256':hashlib.sha256(assembly.encode()).hexdigest(),'vector_boolean_instructions':instructions,'count_instructions':counts,'vector_memory_instructions':vector_memory}
    record['scalar_boolean_instructions']=[line.strip() for line in assembly.splitlines() if re.search(r'\b(?:and|bic|orr|eor)\s+[xw]\d',line)]
    record['stack_adjustment_instructions']=[line.strip() for line in assembly.splitlines()
        if re.search(r'\b(?:sub|add)\s+sp,\s*sp,',line)
        or re.search(r'\b(?:stp|str)\s+.*\[sp,\s*#-.*\]!',line)]
    records.append(record)
    print(label, len(assembly.splitlines()), 'lines;', len(instructions), 'NEON Boolean instructions')
    if counts: print('\n'.join(counts))
out={'binary_sha256':metadata['binary_sha256'],'lab_sources':metadata['lab_sources'],'profile':metadata['profile'],'symbols':records}
(RESULTS/('assembly-'+args.tag+'.json')).write_text(json.dumps(out,indent=2)+'\n')
