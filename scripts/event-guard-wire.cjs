// Guard grammar and native plan admission, independently of SDK authoring.
module.exports = async ({ native, runtime, snapshot, operation, close, collect, query, full, empty, fixture }) => {
  const assert = require('node:assert/strict');
  const exact = async n => operation(cb => native.runtimeExactRational(runtime, 'fraction',
    [Buffer.from(String(n)), Buffer.from('1')], 0, cb), native.runtimeBytesTake);
  const one = { kind: 'literal', bytes: await exact(1) };
  const zero = { kind: 'literal', bytes: await exact(0) };
  const truth = { kind: 'sign', number: one, signs: 4 };
  const hole = { kind: 'sign', number: { kind: 'divide', left: one, right: zero }, signs: 7 };
  const plan = { kind: 'existing', source: full };
  let evaluations = 0;
  for (const [kind, predicate, expected] of [
    ['holds', truth, full], ['fails', truth, empty], ['undefined', truth, empty],
    ['holds', hole, empty], ['fails', hole, empty], ['undefined', hole, full],
  ]) {
    const rows = await collect(query({ kind: 'guard', expr: { kind, predicate, plan } }));
    assert.equal(rows.length, 1);
    assert.deepEqual(Buffer.from(rows[0][0]), expected);
    evaluations++;
  }
  for (const kind of ['lift','descend']) {
    const ir = query({ kind: 'guard', expr: { kind, predicate: truth, plan, input: 0 } });
    ir.head.unshift({kind:'var'}); ir.rules[0].finds.unshift({kind:'var',var:0});
    const rows = await collect(ir);
    assert.equal(rows.length,2);
    for (const [original,mapped] of rows) assert.deepEqual(mapped,original);
    evaluations++;
  }
  const base = {kind:'holds',predicate:truth,plan};
  const malformed = [
    {...base,kind:'maybe'}, {...base,ignored:true}, {...base,input:0},
    {...base,predicate:{kind:'var',var:0}},
    {...base,plan:{...plan,source:fixture}},
    {...base,plan:{...plan,source:Buffer.from('BEVT\x01')}},
    {...base,plan:{...plan,ignored:true}},
    {...base,plan:{...plan,identity:new Uint8Array(32)}},
    {...base,plan:{kind:'refine',source:full,identity:new Uint8Array(32)}},
    {...base,plan:{kind:'refine',source:full,identity:new Uint8Array(31)}},
    {...base,plan:{...plan,source:new Uint8Array(new SharedArrayBuffer(full.length))}},
    {...base,kind:'lift'}, {...base,kind:'descend',input:-1},
    {...base,kind:'lift',input:1},
  ];
  let deep=one;
  for(let i=1;i<127;i++)deep={kind:'abs',value:deep};
  malformed.push({...base,predicate:{kind:'sign',number:deep,signs:7}});
  let wide=one;
  for(let i=0;i<11;i++)wide={kind:'add',left:wide,right:wide};
  malformed.push({...base,predicate:{kind:'sign',number:wide,signs:7}});
  for(const expr of malformed) await assert.rejects(() => operation(cb =>
    native.runtimeSnapshotPrepare(snapshot,query({kind:'guard',expr}),cb),native.runtimePreparedTake));
  // Preparing valid work after refusal must still succeed.
  const prepared=await operation(cb=>native.runtimeSnapshotPrepare(snapshot,query({kind:'guard',expr:base}),cb),native.runtimePreparedTake);
  await close(native.runtimePreparedClose,prepared);
  return {guard_wire_evaluations:evaluations,malformed_guard_programs_refused:malformed.length};
};
