// Cookbook recipe 10 — Trees and ASTs (ts/COOKBOOK.md §10): node header +
// per-kind arms (recipe 2's pattern); every edge resolves; the shape
// theorems come from keys on the edge relations. The edge containments
// put lhs/rhs in the "Node.id" class — which is exactly what lets the
// query join lhs against Lit.node. Functional parent (one parent per
// child) ⇒ paths-or-cycles; acyclicity itself is host discipline.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r10 <64-hex>"); lhsLiteral (the two-atom join with a param) prepares
// AND answers the recipe's own semantics; a Node whose arm is missing is
// commit-rejected (the ψ-selected mirrors, recipe 2's theorems).
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto Kind = bdb::closed<"Kind", "Lit", "Add">();

struct NodeRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    bdb::ref_to<Kind.id> kind;
};

struct LitRow {
    std::uint64_t node;
    std::int64_t value;
};

struct AddRow {
    std::uint64_t node;
    std::uint64_t lhs;
    std::uint64_t rhs;
};

struct ParentRow {
    std::uint64_t child;
    std::uint64_t parent;
};

inline constexpr auto Node = bdb::relation<"Node", NodeRow>;
inline constexpr auto Lit = bdb::relation<"Lit", LitRow>;
inline constexpr auto Add = bdb::relation<"Add", AddRow>;
inline constexpr auto Parent = bdb::relation<"Parent", ParentRow>;

inline constexpr auto Ast = bdb::schema<"Ast">(
    Kind,
    Node,
    Lit,
    Add,
    Parent,

    bdb::contained(
        bdb::on(Node.kind),
        bdb::on(Kind.id)
    ),
    bdb::key(Lit.node),
    bdb::key(Add.node),

    // Every node's arm is total, valid, and exclusive (recipe 2's
    // theorems):
    bdb::mirrors(
        bdb::on(bdb::where(Node, {.kind = Kind.Lit}), Node.id),
        bdb::on(Lit.node)
    ),
    bdb::mirrors(
        bdb::on(bdb::where(Node, {.kind = Kind.Add}), Node.id),
        bdb::on(Add.node)
    ),

    // Every child edge resolves — no dangling subtrees, judged at commit
    // (these containments also put lhs/rhs in the "Node.id" class, which
    // is exactly what lets the query below join lhs against Lit.node):
    bdb::contained(
        bdb::on(Add.lhs),
        bdb::on(Node.id)
    ),
    bdb::contained(
        bdb::on(Add.rhs),
        bdb::on(Node.id)
    ),

    // Functional parent (one parent per child) ⇒ the reachable shape is
    // paths-or-cycles; acyclicity itself is outside the ∀∃ vocabulary —
    // host discipline, recorded. Transitive reach is recipe 24's closure.
    bdb::key(Parent.child),
    bdb::contained(
        bdb::on(Parent.child),
        bdb::on(Node.id)
    ),
    bdb::contained(
        bdb::on(Parent.parent),
        bdb::on(Node.id)
    )
);

// The two-atom join: the lhs edge (in the "Node.id" class) meets the Lit
// arm's node column.
inline constexpr auto LhsLiteral =
    bdb::query(Ast).rule([](auto r) consteval {
        auto add = r.vars(Add);
        auto lit = r.vars(Lit);
        return r
            .match(Add,
                {
                    .node = bdb::param<"n">(),
                    .lhs = add.lhs,
                })
            .match(Lit,
                {
                    .node = add.lhs,
                    .value = lit.value,
                })
            .find({
                .value = lit.value,
            });
    });

namespace {

struct CaseResult {
    std::string name;
    bool passed;
};

auto golden_of(std::string_view fixtures, std::string_view recipe)
    -> std::optional<std::string> {
    for (auto const line_range : std::views::split(fixtures, '\n')) {
        auto const line = std::string_view{line_range};
        if (!line.starts_with(recipe)) {
            continue;
        }
        auto const rest = line.substr(recipe.size());
        if (!rest.starts_with(' ')) {
            continue;
        }
        auto hex = rest.substr(1);
        while (!hex.empty() && (hex.back() == '\r' || hex.back() == ' ')) {
            hex.remove_suffix(1);
        }
        if (hex.size() != 64) {
            return std::nullopt;
        }
        return std::string{hex};
    }
    return std::nullopt;
}

auto slurp(std::string_view path) -> std::optional<std::string> {
    auto stream = std::ifstream{std::string{path},
        std::ios::binary | std::ios::ate};
    if (!stream) {
        return std::nullopt;
    }
    auto const size = stream.tellg();
    if (size < 0) {
        return std::nullopt;
    }
    auto text = std::string(static_cast<std::size_t>(size), '\0');
    stream.seekg(0);
    stream.read(text.data(), size);
    if (!stream) {
        return std::nullopt;
    }
    return text;
}

auto make_store_dir() -> std::optional<std::filesystem::path> {
    auto code = std::error_code{};
    auto const root = std::filesystem::temp_directory_path(code);
    if (code) {
        return std::nullopt;
    }
    auto device = std::random_device{};
    auto const dir = root
        / std::format("bumbledb-cookbook-r10-{:08x}{:08x}", device(),
            device());
    std::filesystem::remove_all(dir, code);
    code.clear();
    std::filesystem::create_directories(dir, code);
    if (code) {
        return std::nullopt;
    }
    return dir;
}

struct SeedIds {
    std::uint64_t two;
    std::uint64_t three;
    std::uint64_t sum;
};

/// The AST for (2 + 3): two Lit leaves, one Add root, parent edges up —
/// every node's arm rides the same commit (the mirrors demands the pair).
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
    using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
    using Result = std::expected<Decision, bdb::Error>;
    auto written = db.write([&](bdb::WriteTx& tx) -> Result {
        auto ids = SeedIds{};
        auto const mint = [&](std::uint64_t& out)
            -> std::expected<bool, bdb::Error> {
            return tx.alloc(Node.id).transform(
                [&](std::uint64_t minted) {
                    out = minted;
                    return true;
                });
        };
        auto rows_land = mint(ids.two)
            .and_then([&](bool) { return mint(ids.three); })
            .and_then([&](bool) { return mint(ids.sum); })
            .and_then([&](bool) {
                return tx.insert(Node,
                    NodeRow{.id = ids.two, .kind = Kind.Lit});
            })
            .and_then([&](bool) {
                return tx.insert(Lit,
                    LitRow{.node = ids.two, .value = 2});
            })
            .and_then([&](bool) {
                return tx.insert(Node,
                    NodeRow{.id = ids.three, .kind = Kind.Lit});
            })
            .and_then([&](bool) {
                return tx.insert(Lit,
                    LitRow{.node = ids.three, .value = 3});
            })
            .and_then([&](bool) {
                return tx.insert(Node,
                    NodeRow{.id = ids.sum, .kind = Kind.Add});
            })
            .and_then([&](bool) {
                return tx.insert(Add,
                    AddRow{.node = ids.sum,
                        .lhs = ids.two,
                        .rhs = ids.three});
            })
            .and_then([&](bool) {
                return tx.insert(Parent,
                    ParentRow{.child = ids.two, .parent = ids.sum});
            })
            .and_then([&](bool) {
                return tx.insert(Parent,
                    ParentRow{.child = ids.three, .parent = ids.sum});
            });
        if (!rows_land.has_value()) {
            return std::unexpected{std::move(rows_land).error()};
        }
        return bdb::commit(ids);
    });
    if (!written.has_value()
        || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
        return std::nullopt;
    }
    return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path,
    std::vector<CaseResult>& results) -> void {
    auto const fixtures = slurp(fixtures_path);
    auto const golden =
        fixtures.has_value() ? golden_of(*fixtures, "r10") : std::nullopt;
    if (!golden.has_value()) {
        results.push_back(CaseResult{
            .name = "fixtures file carries an r10 line", .passed = false});
        return;
    }

    auto const dir = make_store_dir();
    if (!dir.has_value()) {
        results.push_back(CaseResult{
            .name = "r10 store directory", .passed = false});
        return;
    }
    auto db = bdb::Db::ephemeral(dir->native(), Ast);
    if (!db.has_value()) {
        std::println("  Db::ephemeral: {}", db.error().message());
        results.push_back(CaseResult{
            .name = "Db::ephemeral admits Ast", .passed = false});
        return;
    }

    auto const fingerprint = db->fingerprint();
    results.push_back(CaseResult{
        .name = "r10 fingerprint matches the pinned golden",
        .passed = fingerprint.has_value() && *fingerprint == *golden,
    });

    auto const ids = seed(*db);
    results.push_back(CaseResult{
        .name = "the (2 + 3) tree commits: headers, arms, and the "
                "parent edges together",
        .passed = ids.has_value(),
    });
    if (!ids.has_value()) {
        return;
    }

    // A node whose arm is MISSING violates the arm-totality mirrors.
    auto armless = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.alloc(Node.id).and_then(
            [&](std::uint64_t node) {
                return tx.insert(Node,
                    NodeRow{.id = node, .kind = Kind.Add});
            });
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "an Add header without its Add arm is commit-rejected "
                "(arm totality)",
        .passed = !armless.has_value()
            && armless.error().kind() == bdb::ErrorKind::CommitRejected
            && !armless.error().violations().empty(),
    });

    auto lhs_literal = db->prepare<LhsLiteral>();
    results.push_back(CaseResult{
        .name = "lhsLiteral prepares through the engine validator",
        .passed = lhs_literal.has_value(),
    });
    if (!lhs_literal.has_value()) {
        return;
    }

    // The root's lhs edge resolves to the Lit arm carrying 2.
    auto at_root = db->execute(*lhs_literal, {.n = ids->sum});
    results.push_back(CaseResult{
        .name = "lhsLiteral(n: root) answers {2}",
        .passed = at_root.has_value() && at_root->size() == 1
            && at_root->rows().front().value == 2,
    });

    // A leaf is not an Add — the join finds nothing.
    auto at_leaf = db->execute(*lhs_literal, {.n = ids->two});
    results.push_back(CaseResult{
        .name = "lhsLiteral(n: leaf) answers the empty set",
        .passed = at_leaf.has_value() && at_leaf->size() == 0,
    });

    auto code = std::error_code{};
    std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
    auto const arguments =
        std::span{argv, static_cast<std::size_t>(argc)};
    if (arguments.size() < 2) {
        std::println("FAIL: usage: r10_ast <fixtures-file>");
        return 1;
    }

    auto results = std::vector<CaseResult>{};
    run_cases(std::string_view{arguments[1]}, results);

    auto failures = std::size_t{0};
    for (auto const& result : results) {
        if (result.passed) {
            std::println("pass: {}", result.name);
        } else {
            std::println("FAIL: {}", result.name);
            ++failures;
        }
    }
    return failures == 0 ? 0 : 1;
}
