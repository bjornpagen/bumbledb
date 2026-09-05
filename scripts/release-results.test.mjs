// P12-owned regressions for the release-evidence checker (scripts/
// release-results.mjs is P00's; these tests pin its behavior so evidence
// bookkeeping bugs cannot quietly qualify a release). Run by battery.sh
// before anything else. Verification of the product is NOT performed here.
import assert from 'node:assert/strict'
import {test} from 'node:test'
import {inventory,validateResults} from './release-results.mjs'

const revision='a'.repeat(40), spec='b'.repeat(40)
const expected={audit:['ENG-001'],gates:['G00','PKG-07B']}
const evidence=()=>({sourceRevision:revision,specificationRevision:spec,executed:1,skipped:0,tests:['permanent-regression'],platform:'fixture',toolchain:'fixture',features:'default',command:'fixture-test',review:'review-reference',artifact:{path:'artifact',sha256:'c'.repeat(64)},report:{path:'report',sha256:'d'.repeat(64)}})
const passed=id=>({id,outcome:'Passed',evidence:[evidence()]})
const manifest=()=>({format:1,sourceRevision:revision,specificationRevision:spec,audits:[passed('ENG-001')],gates:[passed('G00'),{id:'PKG-07B',outcome:'NotRun',evidence:[]}]})

test('selected inventory remains complete, not a vacuous match',()=>{
  const actual=inventory(); assert.equal(actual.audit.length,68); assert.equal(actual.gates.length,237)
})
test('the expanded child families carry exact padded spellings across every chapter',()=>{
  const {gates}=inventory()
  // One representative per chapter-70 family group: a rename, a range-expansion
  // bug or a padding regression in the proposal/checker fails here, not at F3.
  for(const child of ['CONC-01','CONC-06','E-BRIDGE','E-NO-RESERVE','F-INTERVAL','F-OPT-NEG',
    'Q-LARGE-STORE','Q-INJECT','P-KERNEL','P-PERF','PROTO-01','PROTO-20','STORE-10','LOCAL-03',
    'GC-13','FS-05','S3-06','REC-07','BACKUP-05','RESTORE-03','MIG-14','ERASE-04','OPS-TEST-02',
    'API-12','RUN-15','FFI-08','PKG-06','PKG-07A','PKG-07B','TS-MIG-10','APP-08','APP-MAGIC',
    'SPACE-02','HASH-04','G00','G16'])
    assert.ok(gates.includes(child),`inventory lost ${child}`)
  // Zero-padding is load-bearing: unpadded spellings must NOT exist.
  for(const wrong of ['GC-1','PROTO-1','RUN-1']) assert.ok(!gates.includes(wrong),`unpadded ${wrong} appeared`)
  // Audits and child gates never collide namespaces.
  const {audit}=inventory()
  for(const id of audit) assert.ok(!gates.includes(id),`audit ${id} leaked into the gate index`)
})
test('pre-promotion cannot demand not-yet-authorized public distribution',()=>{
  assert.deepEqual(validateResults(manifest(),expected,revision,'pre-promotion',()=>true),[])
  assert.match(validateResults(manifest(),expected,revision,'post-promotion',()=>true).join('\n'),/PKG-07B/)
})
test('PKG-07B is the ONLY excused cell: any other NotRun still blocks pre-promotion',()=>{
  const m=manifest(); m.gates[0]={id:'G00',outcome:'NotRun',evidence:[]}
  assert.match(validateResults(m,expected,revision,'pre-promotion',()=>true).join('\n'),/G00: NotRun \(required\)/)
})
test('missing stale empty skipped and mismatched evidence refuse',()=>{
  for(const mutate of [m=>m.audits=[],m=>m.sourceRevision=spec,m=>m.gates[0].evidence[0].executed=0,m=>m.gates[0].evidence[0].skipped=1,m=>m.audits.push(passed('ENG-001')),m=>m.audits[0].outcome='NotApplicable']){
    const m=manifest();mutate(m);assert.ok(validateResults(m,expected,revision,'pre-promotion',()=>true).length)
  }
  assert.ok(validateResults(manifest(),expected,revision,'pre-promotion',()=>false).length)
})
test('every evidence field the campaign relies on is individually load-bearing',()=>{
  for(const strip of ['platform','toolchain','features','command','review']){
    const m=manifest(); delete m.gates[0].evidence[0][strip]
    assert.match(validateResults(m,expected,revision,'pre-promotion',()=>true).join('\n'),new RegExp(`missing ${strip}`))
  }
  for(const strip of ['artifact','report']){
    const m=manifest(); delete m.gates[0].evidence[0][strip]
    assert.match(validateResults(m,expected,revision,'pre-promotion',()=>true).join('\n'),/hash\/path/)
  }
  // Evidence bound to a different specification revision is stale.
  const m=manifest(); m.gates[0].evidence[0].specificationRevision='e'.repeat(40)
  assert.match(validateResults(m,expected,revision,'pre-promotion',()=>true).join('\n'),/stale evidence revision/)
  // A manifest without a specification revision cannot qualify anything.
  const bare=manifest(); delete bare.specificationRevision
  assert.match(validateResults(bare,expected,revision,'pre-promotion',()=>true).join('\n'),/missing specification revision/)
})
test('foreign duplicate and missing rows are each named exactly',()=>{
  const foreign=manifest(); foreign.gates.push(passed('G99'))
  assert.match(validateResults(foreign,expected,revision,'pre-promotion',()=>true).join('\n'),/unknown G99/)
  const short={...manifest(),gates:[passed('G00')]}
  assert.match(validateResults(short,expected,revision,'pre-promotion',()=>true).join('\n'),/missing PKG-07B/)
  const doubled=manifest(); doubled.gates.push(passed('G00'))
  assert.match(validateResults(doubled,expected,revision,'pre-promotion',()=>true).join('\n'),/duplicate G00/)
})
test('a skipped case without a recorded reason is unexplained, with one it is not',()=>{
  const m=manifest(); m.gates[0].evidence[0].skipped=2; m.gates[0].evidence[0].skipReasons=['arm-only lane on x86 host','credential-gated real-S3 case counted NotRun']
  assert.deepEqual(validateResults(m,expected,revision,'pre-promotion',()=>true),[])
  const bad=manifest(); bad.gates[0].evidence[0].skipped=2
  assert.match(validateResults(bad,expected,revision,'pre-promotion',()=>true).join('\n'),/unexplained skipped/)
})
