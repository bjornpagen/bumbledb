"""Rebuild isolated representation probes with installed tools; no dependencies installed."""
from pathlib import Path
import json, platform, re, subprocess
HERE=Path(__file__).resolve().parent
OUT=HERE.parent/'research'/'representation'
def run(*args):
    return subprocess.check_output(args,text=True,stderr=subprocess.STDOUT)
def main():
    result={'scope':'Isolated candidate layout/instruction probes; no engine or throughput benchmark.',
            'machine':platform.machine()}
    layout=HERE/'layout-check'
    binary=HERE/'arm64-probe'
    try:
        result['rustc']=run('rustc','--version').strip()
        run('rustc','--edition=2021',str(HERE/'layout.rs'),'-o',str(layout))
        result['layout']=run(str(layout)).strip()
        if platform.machine() not in ('arm64','aarch64'):
            result['arm64']='not run: host is not ARM64'
        else:
            result['clang']=run('clang','--version').splitlines()[0]
            flags=['clang','-O3','-std=c11']
            if platform.system()=='Darwin':flags+=['-arch','arm64']
            run(*flags,'-S',str(HERE/'arm64-probe.c'),'-o',str(HERE/'arm64-probe.s'))
            run(*flags,'-DPROBE_MAIN',str(HERE/'arm64-probe.c'),'-o',str(binary))
            result['semantics']=run(str(binary)).strip()
            assembly=(HERE/'arm64-probe.s').read_text()
            bodies={}
            for name in ('event_complement','steal_block','boolean4_block','predicate_lookup16'):
                match=re.search(r'^_?'+name+r':.*?\n(.*?)\.cfi_endproc',assembly,re.M|re.S)
                assert match,name
                bodies[name]=match.group(1)
                assert not re.search(r'^\s*(bl|blr|cmp|cmn|tst|adds|subs|ands|b\.[a-z]+)\s',bodies[name],re.M),name
            assert re.search(r'\beor\s+x0,\s*x0,\s*#(?:0x)?1\b',bodies['event_complement'])
            assert re.search(r'\borr[.\s]',bodies['steal_block']) and re.search(r'\bbic[.\s]',bodies['steal_block'])
            assert re.search(r'\btbl[.\s]',bodies['predicate_lookup16'])
            result['assembly_checks']='four leaf bodies checked; complement EOR, steal ORR/BIC, predicate TBL; no calls or selected scalar flag/conditional branch instructions'
        (OUT/'probes.json').write_text(json.dumps(result,indent=2)+'\n')
        print(json.dumps(result,indent=2))
    finally:
        for path in (layout,binary):
            if path.exists():path.unlink()
if __name__=='__main__':main()
