// :unionfind — the class laws (lowering.md §3): union-find over the
// projected paired faces, the one-generator wall, generator-first naming.
// The analysis verdict is total: every check is computed defensively so
// any single failure produces exactly its own diagnostic.
export module bumbledb:unionfind;

import std;
import :name;
import :spec;

namespace bdb::detail {

inline constexpr std::size_t no_index = ~std::size_t{0};

/// The analysis verdict schema()'s static_asserts read. Total: every
/// check is computed defensively so any single failure produces exactly
/// its own diagnostic.
template<std::size_t CoordCount>
struct law_verdict {
    bool members_known = true;
    std::size_t unknown_statement = 0;
    coord_ref unknown_coordinate{};
    bool relation_missing = false;

    bool lawful = true;
    coord_ref generator_a{};
    coord_ref generator_b{};
    std::size_t wall_statement = 0;

    bool no_restated_implied_key = true;
    std::size_t restated_statement = 0;
    coord_ref restated_fresh{};

    bool no_duplicate_key = true;
    std::size_t duplicate_statement = 0;

    std::array<class_entry, CoordCount> classes{};
};

/// The flat coordinate roster of a relation table.
template<std::size_t CoordCount, std::size_t RelationCount>
consteval auto coord_roster(
    std::array<relation_data, RelationCount> const& relations)
    -> std::array<coord_ref, CoordCount> {
    auto out = std::array<coord_ref, CoordCount>{};
    auto index = std::size_t{0};
    for (auto const& relation : relations) {
        for (auto field = std::size_t{0}; field != relation.field_count;
            ++field) {
            out[index] = coord_ref{
                .relation = relation.name,
                .field = relation.fields[field].name,
            };
            ++index;
        }
    }
    return out;
}

/// The whole §3 computation over the flattened tables.
template<std::size_t CoordCount, std::size_t RelationCount,
    std::size_t StatementCount>
consteval auto analyze(
    std::array<relation_data, RelationCount> const& relations,
    std::array<statement_data, StatementCount> const& statements)
    -> law_verdict<CoordCount> {
    auto verdict = law_verdict<CoordCount>{};
    auto const coords = coord_roster<CoordCount>(relations);

    // The generator judgment (lowering.md §3.2): a fresh-marked field of
    // an ordinary member, or a CLOSED member's synthetic id (sealed
    // index 0 — closedness itself mints the class).
    auto fresh = std::array<bool, CoordCount>{};
    {
        auto index = std::size_t{0};
        for (auto const& relation : relations) {
            for (auto field = std::size_t{0};
                field != relation.field_count; ++field) {
                fresh[index] = relation.fields[field].fresh
                    || (relation.closed && field == 0);
                ++index;
            }
        }
    }

    auto const index_of = [&](name_text relation,
                              name_text field) -> std::size_t {
        for (auto index = std::size_t{0}; index != CoordCount; ++index) {
            if (coords[index].relation == relation
                && coords[index].field == field) {
                return index;
            }
        }
        return no_index;
    };
    auto const relation_known = [&](name_text relation) -> bool {
        for (auto const& entry : relations) {
            if (entry.name == relation) {
                return true;
            }
        }
        return false;
    };

    // Union-find with one generator slot per root (the wall's witness).
    auto parent = std::array<std::size_t, CoordCount>{};
    auto generator = std::array<std::size_t, CoordCount>{};
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        parent[index] = index;
        generator[index] = fresh[index] ? index : no_index;
    }
    auto const find = [&](std::size_t at) -> std::size_t {
        while (parent[at] != at) {
            at = parent[at];
        }
        return at;
    };

    auto paired = std::array<bool, CoordCount>{};

    auto const visit_coordinate = [&](std::size_t statement,
                                      name_text relation,
                                      name_text field) -> std::size_t {
        auto const at = index_of(relation, field);
        if (at == no_index && verdict.members_known) {
            verdict.members_known = false;
            verdict.unknown_statement = statement;
            verdict.unknown_coordinate =
                coord_ref{.relation = relation, .field = field};
            verdict.relation_missing = !relation_known(relation);
        }
        return at;
    };

    for (auto statement = std::size_t{0}; statement != StatementCount;
        ++statement) {
        auto const& data = statements[statement];
        if (data.form == statement_form::key) {
            for (auto position = std::size_t{0};
                position != data.source.width; ++position) {
                visit_coordinate(statement, data.source.relation,
                    data.source.fields[position]);
            }
            continue; // an FD pairs nothing (lowering.md §3.3)
        }
        for (auto position = std::size_t{0};
            position != data.source.width; ++position) {
            auto const a = visit_coordinate(statement,
                data.source.relation, data.source.fields[position]);
            auto const b = visit_coordinate(statement,
                data.target.relation, data.target.fields[position]);
            if (a == no_index || b == no_index) {
                continue;
            }
            paired[a] = true;
            paired[b] = true;
            auto const root_a = find(a);
            auto const root_b = find(b);
            if (root_a == root_b) {
                continue;
            }
            parent[root_b] = root_a;
            if (generator[root_a] != no_index
                && generator[root_b] != no_index) {
                if (verdict.lawful) {
                    verdict.lawful = false;
                    verdict.generator_a = coords[generator[root_a]];
                    verdict.generator_b = coords[generator[root_b]];
                    verdict.wall_statement = statement;
                }
            } else if (generator[root_b] != no_index) {
                generator[root_a] = generator[root_b];
            }
        }
    }

    // Naming: generator-first, else the least member coordinate in
    // relation-declaration × field-declaration order (§3.5).
    auto class_name = std::array<std::size_t, CoordCount>{};
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        class_name[index] = no_index;
    }
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        auto const root = find(index);
        if (class_name[root] == no_index) {
            class_name[root] = generator[root] != no_index
                ? generator[root]
                : index;
        }
    }
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        auto const classed = fresh[index] || paired[index];
        verdict.classes[index] = class_entry{
            .coordinate = coords[index],
            .classed = classed,
            .class_name = classed ? coords[class_name[find(index)]]
                                  : coord_ref{},
        };
    }

    // Re-stating an implied key doubles it and moves the fingerprint —
    // reject at construction, like TS (lowering.md §7.1). Also reject an
    // exact duplicate declared key.
    for (auto statement = std::size_t{0}; statement != StatementCount;
        ++statement) {
        auto const& data = statements[statement];
        if (data.form != statement_form::key) {
            continue;
        }
        if (data.source.width == 1) {
            auto const at = index_of(
                data.source.relation, data.source.fields[0]);
            if (at != no_index && fresh[at]
                && verdict.no_restated_implied_key) {
                verdict.no_restated_implied_key = false;
                verdict.restated_statement = statement;
                verdict.restated_fresh = coords[at];
            }
        }
        for (auto other = std::size_t{0}; other != statement; ++other) {
            auto const& prior = statements[other];
            if (prior.form != statement_form::key
                || !(prior.source.relation == data.source.relation)
                || prior.source.width != data.source.width) {
                continue;
            }
            auto equal = true;
            for (auto position = std::size_t{0};
                position != data.source.width; ++position) {
                if (!(prior.source.fields[position]
                        == data.source.fields[position])) {
                    equal = false;
                }
            }
            if (equal && verdict.no_duplicate_key) {
                verdict.no_duplicate_key = false;
                verdict.duplicate_statement = statement;
            }
        }
    }

    return verdict;
}

} // namespace bdb::detail
