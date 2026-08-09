// compile-fail (TODO_CPP §21): the consteval interval lane — an invalid
// constant interval (lo >= hi under half-open [lo, hi)) is a compile
// error whose diagnostic names the violated precondition.
import std;
import bumbledb;

constexpr auto broken = bdb::interval<std::int64_t>::literal(9, 9);
