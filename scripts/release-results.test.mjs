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
test('pre-promotion cannot demand not-yet-authorized public distribution',()=>{
  assert.deepEqual(validateResults(manifest(),expected,revision,'pre-promotion',()=>true),[])
  assert.match(validateResults(manifest(),expected,revision,'post-promotion',()=>true).join('\n'),/PKG-07B/)
})
test('missing stale empty skipped and mismatched evidence refuse',()=>{
  for(const mutate of [m=>m.audits=[],m=>m.sourceRevision=spec,m=>m.gates[0].evidence[0].executed=0,m=>m.gates[0].evidence[0].skipped=1,m=>m.audits.push(passed('ENG-001')),m=>m.audits[0].outcome='NotApplicable']){
    const m=manifest();mutate(m);assert.ok(validateResults(m,expected,revision,'pre-promotion',()=>true).length)
  }
  assert.ok(validateResults(manifest(),expected,revision,'pre-promotion',()=>false).length)
})
