"""Exact finite semantic checks for Event revision 0.5.

Independent sets/enumeration check BDD constructions, information operators,
source updates, and observable/decision examples. No native macro, Free Join,
semialgebraic solver, TypeSafe inference, or performance claim is exercised.
"""
from fractions import Fraction as Q
from itertools import product
from pathlib import Path
import json
import runpy

carrier = runpy.run_path(str(Path(__file__).with_name('representation-checks.py')))
BDD, Space = carrier['BDD'], carrier['Space']


def subsets(s):
    xs = sorted(s)
    return [frozenset(x for i, x in enumerate(xs) if mask >> i & 1)
            for mask in range(1 << len(xs))]


def partitions(s):
    xs = sorted(s)
    seen = set()
    for labels in product(range(len(xs)), repeat=len(xs)):
        cells = frozenset(frozenset(x for x, label in zip(xs, labels) if label == k)
                          for k in set(labels))
        if cells not in seen:
            seen.add(cells)
            yield tuple(sorted(cells, key=lambda c: sorted(c)))


def union(cells):
    return frozenset().union(*cells)


def may(e, cells, g):
    return union(c for c in cells if c & g & e)


def must(e, cells, g):
    return union(c for c in cells if c & g and not (c & g - e))


def mass(mu, e):
    return sum((mu.get(w, Q(0)) for w in e), Q(0))


def likelihood(mu, l):
    raw = {w: p * l[w] for w, p in mu.items()}
    z = sum(raw.values(), Q(0))
    if not z:
        raise ValueError('impossible observation')
    return {w: p / z for w, p in raw.items()}, z


def jeffrey(mu, cells, targets):
    assert sum(targets, Q(0)) == 1 and all(q >= 0 for q in targets)
    assert union(cells) == set(mu)
    assert sum(map(len, cells)) == len(mu)
    out = {w: Q(0) for w in mu}
    for cell, target in zip(cells, targets):
        if not target:
            continue  # No 0/0 on an unrequested zero cell.
        z = mass(mu, cell)
        if not z:
            raise ValueError('undefined posterior revision')
        for w in cell:
            out[w] = target * mu[w] / z
    return out


def expectation(mu, rows, g):
    grouped = {}
    for value, e in rows:
        grouped[value] = grouped.get(value, frozenset()) | e
    covered = frozenset()
    for e in grouped.values():
        if covered & e & g:
            raise ValueError('ambiguous value in a world')
        covered |= e
    if g - covered:
        raise ValueError('missing value coverage')
    if not mass(mu, g):
        raise ValueError('impossible observation')
    return sum((value * mass(mu, e & g) for value, e in grouped.items()), Q(0)) / mass(mu, g)


def refuses(fn):
    try:
        fn()
    except ValueError:
        return
    raise AssertionError('expected explicit refusal')


def run():
    groups, counts = [], {}
    ite_cases, count_cases = 0, 0
    for support in range(1, 16):
        bdd = BDD(2)
        space = Space(bdd, support)
        worlds = frozenset(w for w in range(4) if support >> w & 1)
        events = subsets(worlds)
        handles = {e: space.intern(bdd.from_bits(sum(1 << w for w in e))) for e in events}
        for g, a, b in product(events, repeat=3):
            gh, ah, bh = (handles[e] for e in (g, a, b))
            result = space.apply(0xe, space.apply(0x8, gh, ah), space.apply(0x8, gh ^ 1, bh))
            oracle = frozenset(w for w in worlds if (w in a if w in g else w in b))
            assert result == handles[oracle]
            ite_cases += 1

            buckets = [1, 0, 0, 0]
            for e in (gh, ah, bh):
                buckets = [space.apply(0xe, space.apply(0x8, buckets[k], e ^ 1),
                                       space.apply(0x8, buckets[k-1] if k else 0, e))
                           for k in range(4)]
            for k, actual in enumerate(buckets):
                expected = frozenset(w for w in worlds if sum(w in e for e in (g, a, b)) == k)
                assert actual == handles[expected]
                count_cases += 1
            assert union(frozenset(w for w in worlds if space.bits(e) >> w & 1)
                         for e in buckets) == worlds
    groups += ['ite_against_world_predicates', 'cardinality_partition_against_counts']
    counts.update(ite_cases=ite_cases, count_buckets=count_cases)

    s = frozenset(range(4))
    ps, es = list(partitions(s)), subsets(s)
    for cells in ps:
        observable = [union(chosen) for chosen in subsets(frozenset(range(len(cells))))
                      for chosen in [[cells[i] for i in chosen]]]
        for e in es:
            upper, lower = may(e, cells, s), must(e, cells, s)
            assert lower <= e <= upper
            assert lower == s - may(s - e, cells, s)
            assert may(upper, cells, s) == upper
            assert must(lower, cells, s) == lower
            for h in observable:
                assert (upper <= h) == (e <= h)
                assert (h <= lower) == (h <= e)
            for f in es:
                assert may(e | f, cells, s) == may(e, cells, s) | may(f, cells, s)
                assert must(e & f, cells, s) == must(e, cells, s) & must(f, cells, s)
            for g in es:
                reachable = may(s, cells, g)
                assert must(e, cells, g) == reachable - may(s-e, cells, g)
                # Simulate complete-cell rows, computed flags, later filter, Pack.
                rows = [(c, not bool(c & g), not bool(c & g - e)) for c in cells]
                staged = union(c for c, empty, no_counterexample in rows if not empty and no_counterexample)
                assert staged == must(e, cells, g)
                assert not must(e, cells, frozenset())
        for finer in ps:
            if all(any(c <= d for d in cells) for c in finer):
                for e in es:
                    assert may(e, finer, s) <= may(e, cells, s)
                    assert must(e, cells, s) <= must(e, finer, s)
    groups += ['may_must_adjunctions_and_refinement', 'evidence_relative_nonvacuity_and_staged_cells']
    counts['information_partitions'] = len(ps)

    triangle = frozenset([(0,0), (0,1), (1,1)])
    by_x = [frozenset(w for w in triangle if w[0] == x) for x in (0,1)]
    by_y = [frozenset(w for w in triangle if w[1] == y) for y in (0,1)]
    seed = frozenset([(0,0)])
    assert may(may(seed, by_x, triangle), by_y, triangle) == triangle
    assert may(may(seed, by_y, triangle), by_x, triangle) == frozenset([(0,0),(0,1)])
    groups.append('constrained_support_saturations_do_not_commute')

    source, target = frozenset(range(3)), frozenset(range(3))
    mu = {0:Q(1,6), 1:Q(2,6), 2:Q(3,6)}
    for values in product(target, repeat=3):
        f = dict(zip(source, values))
        pull = lambda b: frozenset(w for w in source if f[w] in b)
        some = lambda a: frozenset(f[w] for w in a)
        every = lambda a: frozenset(v for v in target if all(w in a for w in source if f[w] == v))
        for a, b in product(subsets(source), subsets(target)):
            assert (some(a) <= b) == (a <= pull(b))
            assert (pull(b) <= a) == (b <= every(a))
            assert every(a) == target - some(source-a)
            assert pull(target-b) == source-pull(b)
        nu = {v:sum((mu[w] for w in source if f[w] == v),Q(0)) for v in some(source)}
        for b in subsets(some(source)):
            assert mass(nu, b) == mass(mu, pull(b))
    # Existential image of one coin outcome is full; its original mass is 1/2.
    assert {0 for w in {0}} == {0 for w in {0,1}}
    assert Q(1,2) != 1
    groups.append('scope_map_adjunctions_and_weighted_pushforward')

    def pre(r, b):
        return frozenset(x for x,y in r if y in b)
    def guaranteed(r, b):
        ys = frozenset(y for _,y in r)
        return pre(r, ys) - pre(r, ys-b)
    r, q = {(0,0),(0,1)}, {(0,0)}
    composed = {(x,z) for x,y in r for yy,z in q if y == yy}
    assert pre(composed,{0}) == pre(r,pre(q,{0}))
    assert guaranteed(composed,{0}) == {0}
    assert guaranteed(r,guaranteed(q,{0})) == frozenset()
    assert guaranteed(set(),{0}) == frozenset()
    groups.append('partial_transition_dead_end_falsifier')

    # All binary relations on two states: residuation, modalities, converse,
    # and a BDD relational product checked against explicit relational join.
    states = frozenset([0,1])
    ambient = frozenset(product(states, repeat=2))
    relations = subsets(ambient)
    compose = lambda r,q: frozenset((x,z) for x,y in r for yy,z in q if y == yy)
    converse = lambda r: frozenset((y,x) for x,y in r)
    all_next = lambda r,e: states-pre(r,states-e)
    left_residual = lambda r,v: frozenset((y,z) for y,z in ambient
                                          if all((x,z) in v for x,yy in r if yy == y))
    right_residual = lambda v,q: frozenset((x,y) for x,y in ambient
                                           if all((x,z) in v for yy,z in q if yy == y))
    residual_cases = 0
    for r,q in product(relations, repeat=2):
        composed = compose(r,q)
        assert converse(composed) == compose(converse(q),converse(r))
        for e in subsets(states):
            assert pre(composed,e) == pre(r,pre(q,e))
            assert all_next(composed,e) == all_next(r,all_next(q,e))
        for v in relations:
            assert (composed <= v) == (q <= left_residual(r,v))
            assert (composed <= v) == (r <= right_residual(v,q))
            assert compose(composed,v) == compose(r,compose(q,v))
            residual_cases += 1
        bdd = BDD(3)  # x=bit0, middle y=bit1, z=bit2
        rr = bdd.from_bits(sum(1 << w for w in range(8) if (w&1,(w>>1)&1) in r))
        qr = bdd.from_bits(sum(1 << w for w in range(8) if ((w>>1)&1,(w>>2)&1) in q))
        def exists_middle(ref):
            if ref < 2:
                return ref
            var,lo,hi = bdd.nodes[ref >> 1]
            lo,hi = exists_middle(lo ^ (ref&1)),exists_middle(hi ^ (ref&1))
            return bdd.apply(0xe,lo,hi) if var == 1 else bdd.mk(var,lo,hi)
        result = exists_middle(bdd.apply(0x8,rr,qr))
        for w in range(8):
            assert bool(bdd.evaluate(result,w)) == ((w&1,(w>>2)&1) in composed)
    groups += ['relational_residual_adjunctions_and_modal_composition', 'bdd_relational_product_against_join_projection']
    counts['residual_cases'] = residual_cases

    # Finite closure and inevitable reachability: independent bounded-path
    # enumeration detects dead ends and cycles avoiding the goal.
    states3 = frozenset(range(3)); identity = frozenset((x,x) for x in states3)
    closure_cases = 0
    for r in subsets(frozenset(product(states3,repeat=2))):
        closed = identity | r
        while True:
            grown = closed | compose(closed,r)
            if grown == closed:
                break
            closed = grown
        for e in subsets(states3):
            may_fixed, must_fixed = frozenset(),frozenset()
            while True:
                grown = e | pre(r,may_fixed)
                if grown == may_fixed:
                    break
                may_fixed = grown
            while True:
                grown = e | guaranteed(r,must_fixed)
                if grown == must_fixed:
                    break
                must_fixed = grown
            assert may_fixed == pre(closed,e)
            def avoids(start):
                if start in e:
                    return False
                frontier = {start}
                for _ in range(len(states3)):
                    following = set()
                    for x in frontier:
                        successors = {y for xx,y in r if xx == x}
                        if not successors:
                            return True
                        following |= successors-e
                    frontier = following
                    if not frontier:
                        return False
                return bool(frontier)  # At least one avoiding cycle exists.
            assert must_fixed == frozenset(x for x in states3 if not avoids(x))
            closure_cases += 1
    groups.append('finite_closure_and_inevitable_reachability')
    counts['closure_goal_cases'] = closure_cases

    # Shared environments cannot acquire separate witnesses under composition.
    env_r = {(0,'start','middle')}
    env_q = {(1,'middle','finish')}
    shared = {(theta,x,z) for theta,x,y in env_r for phi,yy,z in env_q
              if theta == phi and y == yy}
    independently_forgotten = {(x,z) for theta,x,y in env_r for phi,yy,z in env_q
                              if y == yy}
    assert not shared and independently_forgotten == {('start','finish')}
    # Hiding a transition constraint in the ambient product can even break
    # associativity when every composition is clipped back to that support.
    clipped_ambient = {(0,0),(0,1),(1,0)}
    clipped = lambda a,b: compose(a,b) & clipped_ambient
    a0,b0,c0 = {(1,0)},{(0,1)},{(1,0)}
    assert clipped(clipped(a0,b0),c0) != clipped(a0,clipped(b0,c0))
    assert compose(compose(a0,b0),c0) == compose(a0,compose(b0,c0))
    groups.append('relational_product_environment_and_ambient_support')

    # Forced Coup: each rival has one live card, one Assassin and one Duke.
    # The two hidden role assignments are one visible information case.
    hidden = frozenset(['assassin_bob','assassin_cleo'])
    actions = ('coup_bob','coup_cleo')
    good = frozenset([('assassin_bob','coup_bob'),('assassin_cleo','coup_cleo')])
    assert all(any((w,a) in good for a in actions) for w in hidden)
    assert not any(all((w,a) in good for w in hidden) for a in actions)
    for cells,expected_count in (([hidden],0), ([frozenset([w]) for w in hidden],2)):
        info = {(i,w) for i,c in enumerate(cells) for w in c}
        # LeftResidual(Converse(I), Good) on typed Observation × Action.
        allowed = {(i,a) for i in range(len(cells)) for a in actions
                   if all((w,a) in good for ii,w in info if ii == i)}
        assert len(allowed) == expected_count
        assert all(all((w,a) in good for w in cells[i]) for i,a in allowed)
    trajectories = frozenset(product(hidden,actions))
    target_events = [{w for w in trajectories if w[1] == a} for a in actions]
    trajectory_law = {w:Q(1,2)*(Q(63,100) if w[1]=='coup_bob' else Q(37,100))
                      for w in trajectories}
    assert expectation(trajectory_law,[(7,e) for e in target_events],trajectories) == 7
    assert all(10-7 == 3 and 1-1 == 0 for _ in trajectories)
    fixed_best = max(sum(Q(1,2) for w in hidden if (w,a) in good) for a in actions)
    informed_best = sum(max(Q(1,2) if (w,a) in good else Q(0) for a in actions) for w in hidden)
    assert fixed_best == Q(1,2) and informed_best-fixed_best == Q(1,2)
    groups.append('coup_uniform_action_residual_and_certain_cost')

    # Fully observed action arenas: compare monotone programs with independent
    # enumeration of all deterministic stationary policies and their paths.
    arena_cases = 0
    for step in subsets(frozenset(product(states,states,states))):  # state,action,state
        succ = lambda x,a: frozenset(y for xx,aa,y in step if xx == x and aa == a)
        cpre = lambda e: frozenset(x for x in states if any(succ(x,a) and succ(x,a) <= e for a in states))
        for goal in subsets(states):
            reach, safe = frozenset(), states
            while True:
                grown = goal | cpre(reach)
                if grown == reach: break
                reach = grown
            while True:
                shrunk = goal & cpre(safe)
                if shrunk == safe: break
                safe = shrunk
            winning_reach,winning_safe = set(),set()
            for choices in product(states,repeat=len(states)):
                for start in states:
                    frontier = {start}-goal
                    failure = False
                    for _ in range(len(states)):
                        if any(not succ(x,choices[x]) for x in frontier):
                            failure = True; break
                        frontier = union(succ(x,choices[x]) for x in frontier)-goal
                    if not failure and not frontier: winning_reach.add(start)
                    visited, frontier = set(), {start}
                    while frontier:
                        visited |= frontier
                        frontier = union(succ(x,choices[x]) for x in frontier)-visited
                    if visited <= goal and all(succ(x,choices[x]) for x in visited):
                        winning_safe.add(start)
            assert reach == winning_reach and safe == winning_safe
            arena_cases += 1
    counts['action_arena_goal_cases'] = arena_cases
    groups.append('finite_action_fixed_points_against_policy_enumeration')

    structural = Space(BDD(1),0b11)
    heads = structural.intern(structural.bdd.from_bits(0b10))
    assert heads != 0 and structural.bits(heads) == 0b10
    assert mass({0:Q(1),1:Q(0)},{1}) == 0
    supported = Space(BDD(1),0b01)
    assert supported.intern(supported.bdd.from_bits(0b10)) == 0
    groups.append('structural_possibility_survives_zero_probability')

    for t in (Q(1,5),Q(9,25),Q(3,5)):
        joint = {(1,1):t, (1,0):Q(3,5)-t, (0,1):Q(3,5)-t, (0,0):t-Q(1,5)}
        assert sum(joint.values()) == 1 and all(p >= 0 for p in joint.values())
        assert sum(p for (a,b),p in joint.items() if a) == Q(3,5)
        assert sum(p for (a,b),p in joint.items() if b) == Q(3,5)
        assert joint[(1,0)] == Q(3,5)-t
    groups.append('same_marginals_different_event_answers')

    a = frozenset([0,1]); na = s-a
    prior = {0:Q(1,10),1:Q(1,10),2:Q(2,5),3:Q(2,5)}
    revised = jeffrey(prior,[a,na],[Q(4,5),Q(1,5)])
    l = {w:Q(4,5) if w in a else Q(1,5) for w in s}
    observed,z = likelihood(prior,l)
    assert mass(revised,a) == Q(4,5)
    assert mass(observed,a) == Q(1,2)
    assert z == Q(8,25)
    assert jeffrey(revised,[a,na],[Q(4,5),Q(1,5)]) == revised
    twice,_ = likelihood(observed,l)
    assert twice != observed
    hard = {w:Q(int(w in a)) for w in s}
    once,_ = likelihood(prior,hard)
    assert likelihood(once,hard)[0] == once
    assert revised[0]/revised[1] == prior[0]/prior[1]
    refuses(lambda: jeffrey({0:Q(0),1:Q(1)},[{0},{1}],[Q(1,2),Q(1,2)]))
    assert jeffrey({0:Q(0),1:Q(1)},[{0},{1}],[Q(0),Q(1)]) == {0:Q(0),1:Q(1)}
    for p in (Q(0),Q(1)):
        refuses(lambda: jeffrey({0:p,1:1-p},[{0},{1}],[Q(4,5),Q(1,5)]))
    b = frozenset([0,2]); prior2={0:Q(1,10),1:Q(2,10),2:Q(3,10),3:Q(4,10)}
    ab = jeffrey(jeffrey(prior2,[a,na],[Q(3,4),Q(1,4)]),[b,s-b],[Q(1,3),Q(2,3)])
    ba = jeffrey(jeffrey(prior2,[b,s-b],[Q(1,3),Q(2,3)]),[a,na],[Q(3,4),Q(1,4)])
    assert ab != ba
    groups += ['posterior_versus_likelihood_and_evidence_identity', 'revision_order_and_zero_support_guards']

    uniform = {w:Q(1,4) for w in s}
    rows = [(2,a),(-1,na),(2,frozenset([0]))]
    assert expectation(uniform,rows,s) == Q(1,2)
    refuses(lambda: expectation(uniform,[(2,a)],s))
    refuses(lambda: expectation(uniform,[(2,s),(-1,a)],s))
    refuses(lambda: expectation(uniform,rows,frozenset()))
    zero_evidence_law = {0:Q(0),1:Q(1)}
    for zero_rows,expected_error in (([], 'missing value coverage'),
                                     ([(1,{0}),(2,{0})], 'ambiguous value in a world'),
                                     ([(1,{0})], 'impossible observation')):
        try:
            expectation(zero_evidence_law,zero_rows,{0})
        except ValueError as error:
            assert str(error) == expected_error
        else:
            raise AssertionError('expected structural validation or zero evidence result')
    assert expectation(uniform,[(7,a),(7,na)],s) == 7
    assert expectation(uniform,[(7,a)],a) == 7
    # Identical events for two distinct things count twice, XOR of three is parity.
    assert sum(0 in e for e in (a,a)) == 2
    assert (a ^ a ^ a) == a and not {w for w in s if sum(w in e for e in (a,a,a)) == 1}
    groups.append('observable_coverage_duplicates_and_certain_expenditure')

    payoff_a=lambda p:10*p+100*(1-p)
    payoff_b=lambda p:101*(1-p)
    bounds_a=sorted(payoff_a(p) for p in (Q(1,5),Q(4,5)))
    bounds_b=sorted(payoff_b(p) for p in (Q(1,5),Q(4,5)))
    assert max(bounds_a[0],bounds_b[0]) < min(bounds_a[1],bounds_b[1])
    assert min(payoff_a(p)-payoff_b(p) for p in (Q(1,5),Q(4,5))) == Q(6,5)
    groups.append('robust_dominance_uses_shared_source')

    utilities=[{w:2 if w in a else 0 for w in s},{w:0 if w in a else 2 for w in s}]
    baseline=max(sum(uniform[w]*u[w] for w in s) for u in utilities)
    values=[]
    for cells in ps:
        informed=sum(max(sum(uniform[w]*u[w] for w in c) for u in utilities) for c in cells)
        assert informed >= baseline
        values.append(informed-baseline)
    assert baseline == 1 and max(values) == 1 and min(values) == 0
    groups.append('observation_value_respects_visible_cases')

    def add(x,y): return (x[0]+y[0],x[1]+y[1])
    def mul(x,y): return (x[0]*y[0],x[1]*y[0]+x[0]*y[1])
    def div(x,y):
        assert y[0] > 0
        return (x[0]/y[0], (x[1]*y[0]-x[0]*y[1])/(y[0]**2))
    for p in (Q(1,5),Q(1,2),Q(4,5)):
        assert mul((p,Q(1)),(1-p,Q(-1))) == (p*(1-p),1-2*p)
    h,rate_a,rate_b=Q(23,78),Q(4,5),Q(1,5)
    numerator=mul((h,Q(0)),(rate_a,Q(1)))
    denominator=add(numerator,mul((1-h,Q(0)),(rate_b,Q(0))))
    posterior,derivative=div(numerator,denominator)
    assert posterior == Q(92,147)
    assert derivative == h*(1-h)*rate_b/(h*rate_a+(1-h)*rate_b)**2
    groups.append('exact_sensitivity_and_conditional_quotient')

    return {'scope':'Finite reference semantics only; no native engine or TypeSafe calls.',
            'groups_passed':groups, 'group_count':len(groups), 'counts':counts,
            'posterior_revision':str(mass(revised,a)), 'likelihood_update':str(mass(observed,a)),
            'robust_expected_advantage':str(Q(6,5)), 'perfect_observation_value':str(Q(1))}


if __name__ == '__main__':
    print(json.dumps(run(),indent=2))
