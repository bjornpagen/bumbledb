"""Finite exact checks for the proposed event denotation; no engine code."""

from fractions import Fraction as Q
from itertools import combinations, product
import json


WORLDS = frozenset(range(4))
EVENTS = [frozenset(w for w in WORLDS if mask & (1 << w)) for mask in range(16)]
UNIFORM = {w: Q(1, 4) for w in WORLDS}
NONUNIFORM = {w: Q(w + 1, 10) for w in WORLDS}


def probability(event, law):
    return sum((law[w] for w in event), Q(0))


def disjoint(events):
    return all(not (a & b) for a, b in combinations(events, 2))


def covered(events):
    return frozenset().union(*events)


def check():
    checked = 0
    partitions = 0
    for events in product(EVENTS, repeat=3):
        checked += 1
        key = disjoint(events)
        mirror = covered(events) == WORLDS
        pointwise_exactly_one = all(sum(w in e for e in events) == 1 for w in WORLDS)
        assert (key and mirror) == pointwise_exactly_one
        if key and mirror:
            partitions += 1
            for law in [UNIFORM, NONUNIFORM]:
                assert sum((probability(e, law) for e in events), Q(0)) == 1
    assert partitions == 3**4

    a = frozenset([0, 1])
    complement = WORLDS - a
    independent = frozenset([0, 2])
    for b, overlap in [(complement, Q(0)), (a, Q(1, 2)), (independent, Q(1, 4))]:
        assert probability(a, UNIFORM) + probability(b, UNIFORM) == 1
        assert probability(a & b, UNIFORM) == overlap
    assert disjoint([a, complement]) and covered([a, complement]) == WORLDS
    assert not disjoint([a, a]) and covered([a, a]) != WORLDS
    assert not disjoint([a, independent]) and covered([a, independent]) != WORLDS

    cover_only = [frozenset([0, 1]), frozenset([1, 2, 3])]
    assert covered(cover_only) == WORLDS and not disjoint(cover_only)
    assert sum((probability(e, UNIFORM) for e in cover_only), Q(0)) == Q(5, 4)
    disjoint_only = [frozenset([0]), frozenset([1])]
    assert disjoint(disjoint_only) and covered(disjoint_only) != WORLDS
    assert sum((probability(e, UNIFORM) for e in disjoint_only), Q(0)) == Q(1, 2)

    for event in EVENTS:
        assert event | event == event
        assert event & event == event
        assert not (event & (WORLDS - event))
        assert event | (WORLDS - event) == WORLDS
    assert probability(a | a | a, UNIFORM) == Q(1, 2)

    # Covering an evidence event preserves its mass; it is not total certainty.
    evidence = a
    pieces = [frozenset([0]), frozenset([1])]
    assert disjoint(pieces) and covered(pieces) == evidence
    assert sum((probability(e, UNIFORM) for e in pieces), Q(0)) == Q(1, 2)
    assert all(probability(e, UNIFORM) / probability(evidence, UNIFORM) == Q(1, 2)
               for e in pieces)

    # One pair of fair marginals admits multiple couplings; no default product.
    for r in [Q(0), Q(1, 4), Q(1, 2)]:
        joint = {0: r, 1: Q(1, 2) - r, 2: Q(1, 2) - r, 3: r}
        assert sum(joint.values()) == 1
        assert probability(a, joint) == Q(1, 2)
        assert probability(independent, joint) == Q(1, 2)
        assert probability(a & independent, joint) == r

    # Coefficients in ascending powers of one shared unknown p.
    paths = [(0, 0, 1), (0, 1, -1), (0, 1, -1), (1, -2, 1)]
    assert tuple(sum(path[i] for path in paths) for i in range(3)) == (1, 0, 0)
    evidence_polynomial = tuple(paths[1][i] + paths[2][i] for i in range(3))
    assert evidence_polynomial == (0, 2, -2)
    assert tuple(Q(c, 2) for c in evidence_polynomial) == paths[1]

    return {
        "event_triples_checked": checked,
        "partitions_including_empty_branches": partitions,
        "law_measures_checked": ["uniform", "nonuniform"],
        "groups_passed": [
            "pointwise_key_and_mirror_iff_exact_partition",
            "partition_implies_unit_mass",
            "equal_masses_do_not_determine_events_or_a_partition",
            "coverage_and_disjointness_are_both_required",
            "Boolean_event_laws_and_duplicate_paths",
            "proper_parent_retains_evidence_mass",
            "fair_marginals_do_not_imply_independence",
            "shared_parameter_partition_and_fair_extractor_evidence",
        ],
    }


if __name__ == "__main__":
    print(json.dumps(check(), indent=2))
