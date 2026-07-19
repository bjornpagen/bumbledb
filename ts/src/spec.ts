/**
 * The lowered wire shapes — a 1:1 TypeScript mirror of bumbledb's
 * `SchemaSpec` bindings contract (PRD-01;
 * `bumbledb/crates/bumbledb/src/schema/spec.rs`): a schema as named plain
 * data, relations and dependency statements each in declaration order (the
 * declaration-order law that mints every id). The SDK's `lower()` emits
 * these values; the napi bridge (PRD-04) marshals them into the Rust
 * `SchemaSpec` verbatim, and the engine's own judge (`SchemaSpec::descriptor`
 * name resolution + `SchemaDescriptor::validate` at `Db.create`/`Db.open`)
 * stays the single semantic authority — the SDK lowers, it never re-judges.
 *
 * Every u64 crosses as `bigint`, never `number` (PRD-04's marshaling law: no
 * 53-bit hazards, no branch). Object keys are always written in one fixed
 * literal order per shape, so serialization of a lowered schema is
 * deterministic (byte-stable) by construction.
 */

/**
 * A structural value type — the one type vocabulary of the `schema!` field
 * grammar (`ValueType` in Rust): `bool`, `u64`, `i64`, `str` (`string`
 * here), `bytes<N>` (`fixedBytes`), and the interval family (`width:
 * undefined` is the general 16-byte encoding with rays representable;
 * `width: w` is the fixed-width `interval<E, w>` whose encoding stores only
 * the start).
 */
type ValueTypeSpec =
	| { readonly kind: "bool" }
	| { readonly kind: "u64" }
	| { readonly kind: "i64" }
	| { readonly kind: "string" }
	| { readonly kind: "fixedBytes"; readonly len: number }
	| {
			readonly kind: "interval"
			readonly element: "u64" | "i64"
			readonly width: bigint | undefined
	  }

/**
 * One plain engine value as carried by a lowered literal — the mirror of
 * `bumbledb::Value` restricted to the schema-literal vocabulary (Allen masks
 * are query-side values, never schema literals). Intervals are half-open
 * `[start, end)`.
 */
type ValueSpec =
	| { readonly kind: "bool"; readonly value: boolean }
	| { readonly kind: "u64"; readonly value: bigint }
	| { readonly kind: "i64"; readonly value: bigint }
	| { readonly kind: "string"; readonly value: string }
	| { readonly kind: "fixedBytes"; readonly value: Uint8Array }
	| { readonly kind: "intervalU64"; readonly start: bigint; readonly end: bigint }
	| { readonly kind: "intervalI64"; readonly start: bigint; readonly end: bigint }

/**
 * One literal as spelled: a plain value, or a closed relation's handle by
 * name (the `| status == Frozen` spelling) — resolved by the engine through
 * the selected field's newtype to the handle's declaration-order row id,
 * exactly as the macro resolves it at expansion.
 */
type LiteralSpec =
	| { readonly kind: "value"; readonly value: ValueSpec }
	| { readonly kind: "handle"; readonly handle: string }

/**
 * One σ binding's right side: a single literal or a literal set (read
 * disjunctively). The SDK's selection resolver refuses the degenerate sets
 * (a membership array needs two members — the empty set selects nothing,
 * the one-element set is the bare literal respelled), so a lowered `many`
 * always carries ≥ 2 literals.
 */
type LiteralSetSpec =
	| { readonly kind: "one"; readonly literal: LiteralSpec }
	| { readonly kind: "many"; readonly literals: readonly LiteralSpec[] }

/**
 * One side of a containment or window: `R(fields… | field == literal…)`,
 * all names. `projection` is π in the statement's written order (positional
 * pairing with the other side); `selection` is σ as (field, literal-or-set)
 * pairs, read conjunctively.
 */
interface SideSpec {
	readonly relation: string
	readonly projection: readonly string[]
	readonly selection: ReadonlyArray<readonly [string, LiteralSetSpec]>
}

/**
 * A cardinality window's bounds — the canonical-utterance law's surviving
 * spellings only, since the SDK's `Count` constructors make every banned
 * spelling unwritable or a construction error: `exact` is `{n}` (`{0}` the
 * exclusion), `range` is `{lo..hi}` with lo < hi, `floor` is `{lo..*}` with
 * lo ≥ 2.
 */
type WindowSpec =
	| { readonly kind: "exact"; readonly n: bigint }
	| { readonly kind: "range"; readonly lo: bigint; readonly hi: bigint }
	| { readonly kind: "floor"; readonly lo: bigint }

/**
 * One field: name, structural type, host newtype name — the field's
 * DOMAIN (the macro's declared `as NewType`; the SDK's law-computed class
 * name), carried for handle resolution only, dropped by the engine at
 * descriptor lowering and never fingerprinted — and the `fresh` mint mark.
 */
interface FieldSpec {
	readonly name: string
	readonly valueType: ValueTypeSpec
	readonly newtype: string | undefined
	readonly fresh: boolean
}

/**
 * One ground axiom of a closed relation: the handle plus one literal per
 * declared intrinsic column, in field-declaration order (row id = index).
 */
interface RowSpec {
	readonly handle: string
	readonly values: readonly LiteralSpec[]
}

/**
 * One relation. `extension: rows` declares it closed (the option is the
 * kind); a closed relation's `fields` are its declared intrinsic columns
 * only — the synthetic (`id`, u64) handle field is materialized by the
 * engine's schema validation. `newtype` is the handle newtype of a closed
 * relation (the SDK emits the id's law-computed generator class,
 * `` `${name}.id` `` — the same label every referencing field carries by
 * law), undefined on an ordinary one.
 */
interface RelationSpec {
	readonly name: string
	readonly newtype: string | undefined
	readonly fields: readonly FieldSpec[]
	readonly extension: readonly RowSpec[] | undefined
}

/**
 * One dependency statement, tagged by form. `==` is not a variant: a
 * bidirectional containment is `containment` with `bidirectional: true`,
 * lowered by the engine to the two adjacent containments (`source <=
 * target` first). `cardinality` is B-family, target-left: the target is the
 * per-group parent, the source is counted.
 */
type StatementSpec =
	| { readonly kind: "fd"; readonly relation: string; readonly projection: readonly string[] }
	| {
			readonly kind: "containment"
			readonly source: SideSpec
			readonly target: SideSpec
			readonly bidirectional: boolean
	  }
	| {
			readonly kind: "cardinality"
			readonly target: SideSpec
			readonly window: WindowSpec
			readonly source: SideSpec
	  }

/**
 * The whole theory as named plain data — what `lower()` produces and what
 * the bridge's `dbCreate`/`dbOpen` take. Both lists are in declaration
 * order. Only DECLARED statements appear: the engine materializes the
 * fresh-implied and closed auto-keys itself
 * (`SchemaDescriptor::materialized_statements` — fresh keys first, closed
 * auto-keys second, declared statements last), so re-stating them here
 * would double them and change the fingerprint.
 */
interface SchemaSpec {
	readonly relations: readonly RelationSpec[]
	readonly statements: readonly StatementSpec[]
}

/**
 * The characters Rust's `char::escape_debug` (the engine renderer's string
 * formatter, `schema/render.rs` `literal`) escapes as `\u{…}`: everything
 * non-printable per rustc's generated tables — the C categories (Cc, Cf,
 * Cs, Co, Cn) and the Z separators (Zs, Zl, Zp) except U+0020 itself
 * (`library/core/src/unicode/printable.py`).
 */
const NON_PRINTABLE = /[\p{C}\p{Z}]/u

/** Grapheme-extending characters, which `char::escape_debug` always escapes even when printable. */
const GRAPHEME_EXTEND = /\p{Grapheme_Extend}/u

/**
 * One char exactly as Rust's `char::escape_debug` spells it (the engine
 * renders strings char by char through it): `\0`, `\t`, `\r`, `\n`,
 * backslash-escaped `\\`/`\'`/`\"`, `\u{hex}` (lowercase, unpadded) for
 * grapheme-extending and non-printable chars, the char itself otherwise.
 */
function escapeDebugChar(ch: string): string {
	if (ch === "\0") {
		return "\\0"
	}
	if (ch === "\t") {
		return "\\t"
	}
	if (ch === "\r") {
		return "\\r"
	}
	if (ch === "\n") {
		return "\\n"
	}
	if (ch === "\\" || ch === "'" || ch === '"') {
		return `\\${ch}`
	}
	if (GRAPHEME_EXTEND.test(ch) || (ch !== " " && NON_PRINTABLE.test(ch))) {
		const codePoint = ch.codePointAt(0)
		if (codePoint === undefined) {
			return ch
		}
		return `\\u{${codePoint.toString(16)}}`
	}
	return ch
}

/**
 * One byte exactly as Rust's `u8::escape_ascii` spells it (the engine
 * renders `bytes<N>` literals byte by byte through it): `\t`, `\r`, `\n`,
 * `\\`, `\'`, `\"` as two-char escapes, printable ASCII (0x20–0x7e)
 * verbatim, everything else `\xNN` lowercase.
 */
function escapeAsciiByte(byte: number): string {
	if (byte === 0x09) {
		return "\\t"
	}
	if (byte === 0x0d) {
		return "\\r"
	}
	if (byte === 0x0a) {
		return "\\n"
	}
	if (byte === 0x5c) {
		return "\\\\"
	}
	if (byte === 0x27) {
		return "\\'"
	}
	if (byte === 0x22) {
		return '\\"'
	}
	if (byte >= 0x20 && byte <= 0x7e) {
		return String.fromCharCode(byte)
	}
	return `\\x${byte.toString(16).padStart(2, "0")}`
}

/**
 * Renders one lowered literal in the exact macro spelling the engine's own
 * renderer uses (`schema/render.rs` `literal`): handles bare by name,
 * integers as digits, `true`/`false`, intervals as `start..end`, strings
 * char-escaped through the `char::escape_debug` mirror, bytes as `b"…"`
 * byte-escaped through the `u8::escape_ascii` mirror — byte-for-byte the
 * engine's violation canonicals, so TS-side construction errors and
 * engine-side violations read identically and `renderStatement` equals the
 * violation's `canonical`.
 */
function renderLiteral(literal: LiteralSpec): string {
	if (literal.kind === "handle") {
		return literal.handle
	}
	const value = literal.value
	switch (value.kind) {
		case "bool":
			return value.value ? "true" : "false"
		case "u64":
		case "i64":
			return value.value.toString()
		case "string": {
			let out = '"'
			for (const ch of value.value) {
				out += escapeDebugChar(ch)
			}
			return `${out}"`
		}
		case "fixedBytes": {
			let out = 'b"'
			for (const byte of value.value) {
				out += escapeAsciiByte(byte)
			}
			return `${out}"`
		}
		case "intervalU64":
		case "intervalI64":
			return `${value.start}..${value.end}`
	}
}

/**
 * Renders one σ binding's right side: a bare literal, or a disjunctive
 * literal set in braces (`{A, B}`).
 */
function renderLiteralSet(set: LiteralSetSpec): string {
	if (set.kind === "one") {
		return renderLiteral(set.literal)
	}
	return `{${set.literals.map(renderLiteral).join(", ")}}`
}

/**
 * Renders window bounds in their one canonical spelling: `{n}` exact
 * (`{0}` the exclusion), `{lo..hi}`, `{lo..*}` — the spelling set the
 * engine's renderer emits for sealed statements.
 */
function renderWindow(window: WindowSpec): string {
	switch (window.kind) {
		case "exact":
			return `{${window.n}}`
		case "range":
			return `{${window.lo}..${window.hi}}`
		case "floor":
			return `{${window.lo}..*}`
	}
}

export type {
	FieldSpec,
	LiteralSetSpec,
	LiteralSpec,
	RelationSpec,
	RowSpec,
	SchemaSpec,
	SideSpec,
	StatementSpec,
	ValueSpec,
	ValueTypeSpec,
	WindowSpec
}
export { renderLiteral, renderLiteralSet, renderWindow }
