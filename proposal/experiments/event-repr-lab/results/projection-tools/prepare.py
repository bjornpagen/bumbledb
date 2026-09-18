"""Rebuildable disposable engine copy. Never modifies the real workspace."""
from pathlib import Path
import hashlib, json, shutil, subprocess

LAB = Path(__file__).resolve().parent
ROOT = LAB.parents[2]
SCRATCH = LAB / '.scratch'

def prepare():
    (LAB / 'results').mkdir(parents=True, exist_ok=True)
    engine = SCRATCH / 'engine'
    engine.mkdir(parents=True, exist_ok=True)
    names = ['bumbledb', 'bumbledb-theory', 'bumbledb-macros', 'bumbledb-query-macros']
    hashes = {}
    for name in names:
        src = ROOT / 'crates' / name
        dst = engine / 'crates' / name
        if dst.exists(): shutil.rmtree(dst)
        shutil.copytree(src, dst)
        for path in sorted(src.rglob('*')):
            if path.is_file(): hashes[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest = (ROOT / 'Cargo.toml').read_text()
    start = manifest.index('members = [')
    end = manifest.index('\n]', start) + 2
    manifest = manifest[:start] + 'members = [' + ','.join('"crates/'+n+'"' for n in names) + ']' + manifest[end:]
    (engine / 'Cargo.toml').write_text(manifest)
    shutil.copy2(ROOT / 'Cargo.lock', engine / 'Cargo.lock')
    shutil.copy2(ROOT / 'rust-toolchain.toml', engine / 'rust-toolchain.toml')
    lib = engine / 'crates/bumbledb/src/lib.rs'
    lib.write_text(lib.read_text() + '\n#[cfg(test)]\n#[path = ' + json.dumps(str(LAB / 'src/lib.rs')) + ']\nmod event_repr_lab;\n')
    cargo = engine / 'crates/bumbledb/Cargo.toml'
    cargo.write_text(cargo.read_text().replace('[dev-dependencies]', '[dev-dependencies]\nroaring = "=0.10.12"\nrustc-hash = "=2.1.0"\nnum-bigint = "=0.4.8"\nnum-rational = "=0.4.2"\nnum-traits = "=0.2.19"'))
    (LAB / 'results/source-manifest.json').write_text(json.dumps({
        'git_head': subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'source_sha256': hashes,
        'changes_in_copy': ['test-only event_repr_lab module', 'five lab-only dev dependencies', 'workspace member subset'],
    }, indent=2)+'\n')
    return engine

if __name__ == '__main__': print(prepare())
