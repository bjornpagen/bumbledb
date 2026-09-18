// Actual N-API relation query executions; explicit two-state matrix answers.
const assert = require('node:assert/strict');

module.exports = async ({ collect, query, fixture, full, a, number, blob }) => {
  const states = Buffer.from(full); states[5] = 1; states.fill(60, 8, 40);
  const environment = Buffer.from(states); environment[5] = 0; environment.fill(61, 8, 40);
  const base = Buffer.concat([blob(states), blob(environment), number(0)]);
  const fibre = Buffer.concat([full.subarray(8, 40), Buffer.from([0]), base, base]);
  const faces = Buffer.concat([Buffer.from('BEDC'), Buffer.from([1, 3]), fibre]);
  const plan = Buffer.concat([Buffer.from('BEDC'), Buffer.from([1, 5]), Buffer.alloc(32, 62), fibre, fibre, fibre]);
  const projection = Buffer.concat([Buffer.from('BEDC'), Buffer.from([1, 0]), blob(full), blob(states), number(1), blob(fixture)]);
  const relation = { kind: 'bind', descriptor: faces, expr: a };
  const predicate = { kind: 'map', op: 'image', descriptor: projection, expr: a };
  const view = (op, r = relation) => ({ kind: 'relation', op, relation: r });

  // Canonical tiny full-support BEVT from a truth table, independent of native kernels.
  const golden = (header, bits) => {
    const nodes = [], unique = new Map();
    const visit = (coordinate, world) => {
      if (coordinate === header[5]) return (bits >> world) & 1;
      let low = visit(coordinate + 1, world), high = visit(coordinate + 1, world | (1 << coordinate));
      if (low === high) return low;
      const sign = low & 1; low ^= sign; high ^= sign;
      const key = `${coordinate}/${low}/${high}`;
      let ref = unique.get(key);
      if (ref === undefined) {
        const node = Buffer.alloc(9); node[0] = coordinate;
        node.writeUInt32LE(low, 1); node.writeUInt32LE(high, 5);
        nodes.push(node); ref = 2 * nodes.length; unique.set(key, ref);
      }
      return ref ^ sign;
    };
    const root = visit(0, 0), out = Buffer.alloc(52);
    header.copy(out, 0, 0, 40); out.writeUInt32LE(nodes.length, 40);
    out.writeUInt32LE(1, 44); out.writeUInt32LE(root, 48);
    return Buffer.concat([out, ...nodes]);
  };
  assert.deepEqual(golden(full, 10), fixture);
  const operations = [
    [view('region'), bits => golden(full, bits)],
    [view('domain'), bits => golden(states, bits === 10 ? 2 : 1)],
    [view('range'), () => golden(states, 3)],
    [view('region', { kind: 'converse', relation }), bits => golden(full, bits)],
    [view('region', { kind: 'not', relation }), bits => golden(full, bits ^ 15)],
    [view('region', { kind: 'identity', descriptor: faces }), () => golden(full, 9)],
    [view('region', { kind: 'test', descriptor: faces, expr: predicate }), bits => golden(full, bits & 9)],
    [view('region', { kind: 'star', descriptor: plan, relation }), bits => golden(full, bits | 9)],
    ...['compose', 'leftResidual', 'rightResidual'].map(op => [view('region', {
      kind: 'product', op, descriptor: plan, left: relation, right: relation
    }), bits => {
      let out = 0;
      const cell = (s, t) => Boolean(bits & (1 << (s + 2*t)));
      for (let s = 0; s < 2; s++) for (let t = 0; t < 2; t++) {
        const pass = op === 'compose' ? [0, 1].some(m => cell(s, m) && cell(m, t))
          : op === 'leftResidual' ? [0, 1].every(m => !cell(m, s) || cell(m, t))
          : [0, 1].every(m => !cell(t, m) || cell(s, m));
        if (pass) out |= 1 << (s + 2*t);
      }
      return golden(full, out);
    }]),
    ...['may', 'all', 'must', 'post'].map(op => [{ kind: 'modal', op, relation, expr: predicate }, bits => {
      const domain = bits === 10 ? 2 : 1;
      return golden(states, op === 'may' ? domain : op === 'all' ? domain ^ 3 : op === 'must' ? 0 : 3);
    }]),
    ...Array.from({ length: 16 }, (_, bits) => [view('region', {
      kind: 'apply', bits, left: relation, right: relation
    }), before => golden(full, ((bits & 1) ? before ^ 15 : 0) | ((bits & 8) ? before : 0))]),
  ];
  for (const [expr, expected] of operations) {
    const ir = query({ kind: 'event', expr });
    ir.head.unshift({ kind: 'var' }); ir.rules[0].finds.unshift({ kind: 'var', var: 0 });
    const rows = await collect(ir);
    assert.equal(rows.length, 2);
    for (const [before, after] of rows) {
      const bits = Buffer.from(before).equals(fixture) ? 10 : 5;
      assert.deepEqual(Buffer.from(after), expected(bits));
    }
  }
  const bad = [
    view('unknown'),
    view('region', { kind: 'bind', descriptor: plan, expr: a }),
    view('region', { kind: 'product', op: 'compose', descriptor: faces, left: relation, right: relation }),
    view('region', { kind: 'product', op: 'unknown', descriptor: plan, left: relation, right: relation }),
    view('region', { kind: 'star', descriptor: plan, relation, ignored: true }),
    { kind: 'modal', op: 'unknown', relation, expr: predicate },
    { kind: 'modal', op: 'may', relation, expr: a }, // participating pair is not an endpoint Event
  ];
  const cycle = { kind: 'converse' }; cycle.relation = cycle; bad.push(view('region', cycle));
  for (const expr of bad) await assert.rejects(() => collect(query({ kind: 'event', expr })));
  return { event_relation_evaluations: operations.length, malformed_relations_refused: bad.length };
};
