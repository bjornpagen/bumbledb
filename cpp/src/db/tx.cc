// :tx — the lexical borrowed write capability (TODO_CPP §17, §19, §24–§26):
// alive exactly for the Db::write / Db::write_from callback. Nothing is
// judged until commit; the callback's decision (§19) is the commit/abandon
// switch.
export module bumbledb:tx;

import std;
import :error;
import :answers;
import :row;
import :key;
import :schema;
import :manifest;
import bumbledb_foreign;

export namespace bdb {

/// A lexical borrowed write capability (§17): alive exactly for the
/// Db::write / Db::write_from callback. Non-copyable, non-movable,
/// constructible only by Db's trampoline. Nothing is judged until commit;
/// the callback's decision (§19) is the commit/abandon switch.
class WriteTx {
	foreign::bdb_tx_ref& raw_;
	detail::Manifest const& manifest_;

	WriteTx(foreign::bdb_tx_ref& raw, detail::Manifest const& manifest) : raw_{raw}, manifest_{manifest} {}

	friend class Db;

	[[nodiscard]] auto relation_id(std::string_view relation) const -> std::uint32_t {
		return detail::resolved_relation(manifest_, relation);
	}

public:
	WriteTx(WriteTx const&) = delete;
	auto operator=(WriteTx const&) -> WriteTx& = delete;
	~WriteTx() = default;

	/// Records an insert into the delta (reflection-marshalled, §24);
	/// true = the final state changed. Shape violations are the engine's
	/// typed FactShape error. This phase does not type-check Row against
	/// Facade — that theorem belongs to the schema phase (§28).
	template<class Facade, class Row>
	[[nodiscard]] auto insert(Facade const& relation, Row const& row) -> std::expected<bool, Error> {
		auto const cells = marshal_row(row);
		return foreign::tx_insert(raw_, relation_id(detail::facade_relation_name(relation)), cells)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/// Records a delete into the delta; true = the final state changed.
	template<class Facade, class Row>
	[[nodiscard]] auto remove(Facade const& relation, Row const& row) -> std::expected<bool, Error> {
		auto const cells = marshal_row(row);
		return foreign::tx_remove(raw_, relation_id(detail::facade_relation_name(relation)), cells)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/// Final-state membership (base + pending delta — what the commit
	/// judgment judges; check-then-act is race-free under the single
	/// writer).
	template<class Facade, class Row>
	[[nodiscard]] auto contains(Facade const& relation, Row const& row) const -> std::expected<bool, Error> {
		auto const cells = marshal_row(row);
		return foreign::tx_contains(raw_, relation_id(detail::facade_relation_name(relation)), cells)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/// Final-state keyed point read (§26, the WriteTx twin): the stored
	/// key law value is the selector; reads base + pending delta.
	template<class Facade, class First, class... Rest>
	[[nodiscard]] auto get(Facade const& relation, key_law<First, Rest...> const& law,
	                       typename key_law<First, Rest...>::pattern const& key) const -> std::expected<std::optional<RowSet>, Error> {
		static_assert(detail::facade_relation_name(Facade{}) == key_law<First, Rest...>::relation_name.view(),
		              detail::keyed_get_mismatch<Facade, key_law<First, Rest...>>());
		auto const cells = marshal_row(key);
		return foreign::tx_get(raw_, relation_id(detail::facade_relation_name(relation)), detail::resolved_key(manifest_, law), cells)
		    .transform(detail::lift_row)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/// The fresh-field primary read against the final state (§26).
	template<class Facade>
	    requires(fresh_field_count<Facade>() >= 1)
	[[nodiscard]] auto get(Facade const& relation, fresh_pattern_of<Facade> const& key) const
	    -> std::expected<std::optional<RowSet>, Error> {
		auto const cells = marshal_row(key);
		return foreign::tx_get(raw_, relation_id(detail::facade_relation_name(relation)),
		                       detail::resolved_primary(manifest_, detail::facade_relation_name(relation)), cells)
		    .transform(detail::lift_row)
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}

	/// Mints the next fresh id for the coordinate's field (§25):
	/// `tx.alloc(Service.id)`. The coordinate carries relation name and
	/// ordinal in its type; resolution is the pre-schema name lane (module
	/// comment). Fresh fields are u64 by construction, so only u64
	/// coordinates allocate.
	template<class Field>
	    requires std::same_as<typename Field::value_type, std::uint64_t>
	[[nodiscard]] auto alloc(Field const& field) -> std::expected<std::uint64_t, Error> {
		return foreign::tx_alloc(raw_, relation_id(field.relation()), static_cast<std::uint16_t>(Field::ordinal))
		    .transform_error([](foreign::error_handle handle) {
			    return detail::lift(std::move(handle));
		    });
	}
};

} // namespace bdb
