// D23 regressions for the release-evidence checker. Run by battery.sh
// before product work. Verification of the product is NOT performed here.
import assert from 'node:assert/strict'
import crypto from 'node:crypto'
import path from 'node:path'
import {test} from 'node:test'
import {
  computeCandidateSourceDigest,
  describeCandidateEntry,
  frameCandidateEntry,
  inventory,
  inventoryDigest,
  loadInventoryDocument,
  nativeProvenancePath,
  validateResults,
} from './release-results.mjs'

const candidateDigest='c'.repeat(64)
const spec=inventoryDigest()
const expected={
  audit:['ENG-001'],
  gates:['G00','PKG-07B'],
  priorReview:['CORE-001'],
  discriminators:['D23'],
  specificationRevision:spec,
  qualificationCells:[]
}
const evidence=()=>({
  candidateSourceDigest:candidateDigest,
  specificationRevision:spec,
  executed:1,
  skipped:0,
  tests:['permanent-regression'],
  platform:'fixture',
  backend:'none',
  toolchain:'fixture',
  features:'default',
  command:'fixture-test',
  review:'review-reference',
  artifact:{path:'artifact',sha256:'d'.repeat(64)},
  report:{path:'report',sha256:'e'.repeat(64)}
})
const passed=id=>({id,outcome:'Passed',evidence:[evidence()]})
const manifest=()=>({
  format:2,
  candidateSourceDigest:candidateDigest,
  specificationRevision:spec,
  audits:[passed('ENG-001')],
  gates:[passed('G00'),{id:'PKG-07B',outcome:'NotRun',evidence:[]}],
  priorReviews:[passed('CORE-001')],
  discriminators:[passed('D23')],
  qualification:[]
})

test('selected inventory remains complete, not a vacuous match',()=>{
  const actual=inventory(); assert.equal(actual.audit.length,68); assert.equal(actual.gates.length,237)
  assert.equal(actual.priorReview.length,78); assert.equal(actual.discriminators.length,29)
  assert.equal(actual.qualificationCells.length,8)
})
test('inventory lives in docs/reference and no longer scrapes proposal markdown',()=>{
  const doc=loadInventoryDocument()
  assert.equal(doc.format,1)
  assert.ok(doc.generatedFrom.length)
  assert.ok(doc.generatedFrom.every(entry=>entry.startsWith('docs/reference/')))
  assert.ok(doc.generatedFrom.every(entry=>!entry.startsWith('final-solution/')))
  assert.ok(doc.parentGates.includes('G16'))
  assert.ok(doc.childFamilies.includes('APP-MAGIC'))
  assert.ok(doc.priorReview.includes('REVIEW-004'))
  assert.ok(doc.discriminators.includes('D29'))
  assert.equal(doc.qualificationCells.find(cell=>cell.id==='real-s3-iam')?.allowNotApplicable,undefined)
  assert.equal(doc.qualificationCells.find(cell=>cell.id==='graviton-arm64-runtime')?.allowNotApplicable,undefined)
})
test('the expanded child families carry exact padded spellings across every chapter',()=>{
  const {gates}=inventory()
  for(const child of ['CONC-01','CONC-06','E-BRIDGE','E-NO-RESERVE','F-INTERVAL','F-OPT-NEG',
    'Q-LARGE-STORE','Q-INJECT','P-KERNEL','P-PERF','PROTO-01','PROTO-20','STORE-10','LOCAL-03',
    'GC-13','FS-05','S3-06','REC-07','BACKUP-05','RESTORE-03','MIG-14','ERASE-04','OPS-TEST-02',
    'API-12','RUN-15','FFI-08','PKG-06','PKG-07A','PKG-07B','TS-MIG-10','APP-08','APP-MAGIC',
    'SPACE-02','HASH-04','G00','G16'])
    assert.ok(gates.includes(child),`inventory lost ${child}`)
  for(const wrong of ['GC-1','PROTO-1','RUN-1']) assert.ok(!gates.includes(wrong),`unpadded ${wrong} appeared`)
  const {audit,priorReview,discriminators}=inventory()
  for(const id of audit) assert.ok(!gates.includes(id),`audit ${id} leaked into the gate index`)
  for(const id of priorReview) assert.ok(!gates.includes(id),`prior-review ${id} leaked into the gate index`)
  for(const id of discriminators) assert.ok(!gates.includes(id),`discriminator ${id} leaked into the gate index`)
})
test('pre-promotion cannot demand not-yet-authorized public distribution',()=>{
  assert.deepEqual(validateResults(manifest(),expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}),[])
  assert.match(validateResults(manifest(),expected,'post-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/PKG-07B/)
})
test('PKG-07B is the ONLY excused cell: any other NotRun still blocks pre-promotion',()=>{
  const m=manifest(); m.gates[0]={id:'G00',outcome:'NotRun',evidence:[]}
  assert.match(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/G00: NotRun \(required\)/)
})
test('missing stale empty skipped and mismatched evidence refuse',()=>{
  for(const mutate of [m=>m.audits=[],m=>m.candidateSourceDigest='f'.repeat(64),m=>m.gates[0].evidence[0].executed=0,m=>m.gates[0].evidence[0].skipped=1,m=>m.audits.push(passed('ENG-001')),m=>m.audits[0].outcome='NotApplicable']){
    const m=manifest();mutate(m);assert.ok(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).length)
  }
  assert.ok(validateResults(manifest(),expected,'pre-promotion',()=>false,{candidateSourceDigest:candidateDigest}).length)
})
test('D23 garbage evidence strings and NotApplicable qualification cells refuse',()=>{
  const m=manifest(); m.gates[0].evidence=['garbage']
  assert.match(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/structured record/)
  const full=inventory()
  const qual=manifest()
  qual.qualification=full.qualificationCells.map(cell=>({id:cell.id,outcome:'Passed',evidence:[evidence()]}))
  qual.qualification.find(row=>row.id==='real-s3-iam').outcome='NotApplicable'
  assert.match(validateResults(qual,full,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/real-s3-iam: NotApplicable/)
})
test('D23 nonexistent report and hash-mismatched stale dist refuse',()=>{
  assert.match(validateResults(manifest(),expected,'pre-promotion',file=>file.path!=='report',{candidateSourceDigest:candidateDigest}).join('\n'),/report missing or hash mismatch/)
  assert.match(validateResults(manifest(),expected,'pre-promotion',file=>file.path!=='artifact',{candidateSourceDigest:candidateDigest}).join('\n'),/artifact missing or hash mismatch/)
})
test('D23 digestOverride and caller path overrides refuse',()=>{
  const overridden=manifest(); overridden.digestOverride='a'.repeat(64)
  assert.match(validateResults(overridden,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/digestOverride/)
  const paths=manifest(); paths.candidatePaths=['README.md']
  assert.match(validateResults(paths,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/candidatePaths/)
  assert.throws(()=>validateResults(manifest(),expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest,digestOverride:'a'.repeat(64)}),/overrides/)
  assert.throws(()=>computeCandidateSourceDigest(['README.md']),/cannot override recomputed membership/)
})
test('D23 unknown and duplicate qualification cell IDs refuse',()=>{
  const unknown=manifest(); unknown.qualification=[{id:'not-a-cell',outcome:'Passed',evidence:[evidence()]}]
  const full=inventory()
  assert.match(validateResults(unknown,full,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/unknown not-a-cell/)
  const dup=manifest()
  dup.qualification=full.qualificationCells.flatMap(cell=>[
    {id:cell.id,outcome:'Passed',evidence:[evidence()]},
    {id:cell.id,outcome:'Passed',evidence:[evidence()]}
  ])
  assert.match(validateResults(dup,full,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/duplicate check-macos-arm64/)
})
test('every evidence field the campaign relies on is individually load-bearing',()=>{
  for(const strip of ['platform','backend','toolchain','features','command','review']){
    const m=manifest(); delete m.gates[0].evidence[0][strip]
    assert.match(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),new RegExp(`missing ${strip}`))
  }
  for(const strip of ['artifact','report']){
    const m=manifest(); delete m.gates[0].evidence[0][strip]
    assert.match(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/hash\/path/)
  }
  const m=manifest(); m.gates[0].evidence[0].specificationRevision='e'.repeat(64)
  assert.match(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/stale evidence specification revision/)
  const bare=manifest(); delete bare.specificationRevision
  assert.match(validateResults(bare,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/missing or stale/)
})
test('foreign duplicate and missing rows are each named exactly',()=>{
  const foreign=manifest(); foreign.gates.push(passed('G99'))
  assert.match(validateResults(foreign,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/unknown G99/)
  const short={...manifest(),gates:[passed('G00')]}
  assert.match(validateResults(short,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/missing PKG-07B/)
  const doubled=manifest(); doubled.gates.push(passed('G00'))
  assert.match(validateResults(doubled,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/duplicate G00/)
  const noPrior={...manifest()}; delete noPrior.priorReviews
  assert.match(validateResults(noPrior,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/missing priorReviews/)
  const noDisc={...manifest()}; delete noDisc.discriminators
  assert.match(validateResults(noDisc,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/missing discriminators/)
})
test('a skipped case without a recorded reason is unexplained, with one it is not',()=>{
  const m=manifest(); m.gates[0].evidence[0].skipped=2; m.gates[0].evidence[0].skipReasons=['arm-only lane on x86 host','credential-gated real-S3 case counted NotRun']
  assert.deepEqual(validateResults(m,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}),[])
  const bad=manifest(); bad.gates[0].evidence[0].skipped=2
  assert.match(validateResults(bad,expected,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/unexplained skipped/)
})
test('required qualification cells must be present and passed with substantive evidence',()=>{
  const full=inventory()
  const m=manifest()
  m.qualification=full.qualificationCells.map(cell=>({id:cell.id,outcome:'Passed',evidence:[evidence()]}))
  assert.deepEqual(validateResults(m,full,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}),[])
  m.qualification[0].outcome='NotRun'
  assert.match(validateResults(m,full,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/check-macos-arm64/)
  m.qualification[0].outcome='Passed'
  m.qualification[0].evidence=['garbage']
  assert.match(validateResults(m,full,'pre-promotion',()=>true,{candidateSourceDigest:candidateDigest}).join('\n'),/check-macos-arm64: .*structured record/)
})
test('candidate digest frames record symlink targets modes and deleted paths',()=>{
  const fileEntry={rel:'scripts/run.sh',kind:'file',mode:0o755,payload:'a'.repeat(64)}
  const linkEntry={rel:'scripts/alias.sh',kind:'symlink',mode:0o777,payload:'run.sh'}
  const deletedEntry={rel:'removed/tracked.rs',kind:'deleted',mode:0,payload:''}
  const digest=crypto.createHash('sha256')
  digest.update(frameCandidateEntry(fileEntry))
  digest.update(frameCandidateEntry(linkEntry))
  digest.update(frameCandidateEntry(deletedEntry))
  assert.match(digest.digest('hex'),/^[a-f0-9]{64}$/)
  assert.notEqual(frameCandidateEntry(fileEntry),frameCandidateEntry({...fileEntry,mode:0o644}))
  assert.notEqual(frameCandidateEntry(linkEntry),frameCandidateEntry({...linkEntry,payload:'other.sh'}))
})
test('deleted tracked paths frame as deleted without crashing enumeration',()=>{
  const entry=describeCandidateEntry(path.join('definitely', 'missing', 'tracked', 'file.rs'))
  assert.equal(entry.kind,'deleted')
  assert.equal(entry.payload,'')
})
test('native provenance sidecar sits beside the platform binary',()=>{
  assert.equal(nativeProvenancePath('ts/npm/darwin-arm64/bumbledb.node'),'ts/npm/darwin-arm64/.native-provenance.json')
})
test('the live candidate digest is stable for the current checkout inventory',()=>{
  assert.match(computeCandidateSourceDigest(),/^[a-f0-9]{64}$/)
})
