use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{Fact, Value};

pub(super) fn sailor(id: u64, rating: u64) -> Fact {
    Fact::new(
        "Sailor",
        [("id", Value::Serial(id)), ("rating", Value::U64(rating))],
    )
}
pub(super) fn boat(id: u64, color: u8) -> Fact {
    Fact::new(
        "Boat",
        [("id", Value::Serial(id)), ("color", Value::Enum(color))],
    )
}
pub(super) fn reserve(sailor: u64, boat: u64, day: i64) -> Fact {
    Fact::new(
        "Reserve",
        [
            ("sailor", Value::Serial(sailor)),
            ("boat", Value::Serial(boat)),
            ("day", Value::Timestamp(TimestampMicros(day))),
        ],
    )
}
pub(super) fn edge_ab(a: u64, b: u64) -> Fact {
    Fact::new("EdgeAB", [("a", Value::U64(a)), ("b", Value::U64(b))])
}
pub(super) fn edge_ac(a: u64, c: u64) -> Fact {
    Fact::new("EdgeAC", [("a", Value::U64(a)), ("c", Value::U64(c))])
}
pub(super) fn edge_bc(b: u64, c: u64) -> Fact {
    Fact::new("EdgeBC", [("b", Value::U64(b)), ("c", Value::U64(c))])
}
pub(super) fn customer(id: u64, nation: u64) -> Fact {
    Fact::new(
        "Customer",
        [("id", Value::Serial(id)), ("nation", Value::U64(nation))],
    )
}
pub(super) fn supplier(id: u64, nation: u64) -> Fact {
    Fact::new(
        "Supplier",
        [("id", Value::Serial(id)), ("nation", Value::U64(nation))],
    )
}
pub(super) fn orders(id: u64, customer: u64) -> Fact {
    Fact::new(
        "Orders",
        [
            ("id", Value::Serial(id)),
            ("customer", Value::Serial(customer)),
        ],
    )
}
pub(super) fn lineitem(id: u64, order: u64, price: i128) -> Fact {
    Fact::new(
        "LineItem",
        [
            ("id", Value::Serial(id)),
            ("order", Value::Serial(order)),
            ("extended_price", Value::Decimal(DecimalRaw(price))),
        ],
    )
}
pub(super) fn title(id: u64, year: i64) -> Fact {
    Fact::new(
        "Title",
        [("id", Value::Serial(id)), ("year", Value::I64(year))],
    )
}
pub(super) fn name(id: u64) -> Fact {
    Fact::new("Name", [("id", Value::Serial(id))])
}
pub(super) fn principal(title: u64, name: u64, category: u8, ordering: u64) -> Fact {
    Fact::new(
        "Principal",
        [
            ("title", Value::Serial(title)),
            ("name", Value::Serial(name)),
            ("category", Value::Enum(category)),
            ("ordering", Value::U64(ordering)),
        ],
    )
}
pub(super) fn player(id: u64) -> Fact {
    Fact::new("Player", [("id", Value::Serial(id))])
}
pub(super) fn team(id: u64, year: i64) -> Fact {
    Fact::new(
        "Team",
        [("id", Value::Serial(id)), ("year", Value::I64(year))],
    )
}
pub(super) fn batting(player: u64, team: u64, year: i64, hits: i64) -> Fact {
    Fact::new(
        "Batting",
        [
            ("player", Value::Serial(player)),
            ("team", Value::Serial(team)),
            ("year", Value::I64(year)),
            ("hits", Value::I64(hits)),
        ],
    )
}
pub(super) fn salary(player: u64, team: u64, year: i64, salary: i64) -> Fact {
    Fact::new(
        "Salary",
        [
            ("player", Value::Serial(player)),
            ("team", Value::Serial(team)),
            ("year", Value::I64(year)),
            ("salary", Value::I64(salary)),
        ],
    )
}
pub(super) fn person(id: u64) -> Fact {
    Fact::new("Person", [("id", Value::Serial(id))])
}
pub(super) fn knows(left: u64, right: u64) -> Fact {
    Fact::new(
        "Knows",
        [
            ("person1", Value::Serial(left)),
            ("person2", Value::Serial(right)),
        ],
    )
}
