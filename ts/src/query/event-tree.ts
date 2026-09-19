/** One structural translation of the native grammar. Query builders retain
 * variable objects and owned descriptors; the wire uses ordinals and bytes.
 * This visits all written operands and never evaluates or simplifies them. */
import type { EventExprIr, EventTestIr, RelationExprIr } from "#native.ts"

interface Translation<V, D, S, W, E, T> {
	readonly variable: (value: V) => W
	readonly descriptor: (value: D) => E
	readonly scope: (value: S) => T
	readonly bound?: (value: number, nesting: number) => number
	readonly enter?: (depth: number) => void
}

function eventTree<V, D, S, W, E, T>(
	node: EventExprIr<V, D, S>,
	map: Translation<V, D, S, W, E, T>,
	depth = 1,
	nesting = 0
): EventExprIr<W, E, T> {
	map.enter?.(depth)
	const event = (child: EventExprIr<V, D, S>) => eventTree(child, map, depth + 1, nesting)
	switch (node.kind) {
		case "bound":
			return Object.freeze({ ...node, depth: map.bound?.(node.depth, nesting) ?? node.depth })
		case "fixed":
			return Object.freeze({
				...node,
				scope: map.scope(node.scope),
				expr: eventTree(node.expr, map, depth + 1, nesting + 1)
			})
		case "var":
		case "empty":
		case "full":
			return Object.freeze({ ...node, var: map.variable(node.var) })
		case "not":
			return Object.freeze({ ...node, expr: event(node.expr) })
		case "apply":
			return Object.freeze({ ...node, left: event(node.left), right: event(node.right) })
		case "ite":
			return Object.freeze({ ...node, condition: event(node.condition), high: event(node.high), low: event(node.low) })
		case "cardinality":
			return Object.freeze({ ...node, events: Object.freeze(node.events.map(event)) })
		case "map":
			return Object.freeze({ ...node, descriptor: map.descriptor(node.descriptor), expr: event(node.expr) })
		case "relation":
			return Object.freeze({ ...node, relation: relationTree(node.relation, map, depth + 1, nesting) })
		case "modal":
			return Object.freeze({
				...node,
				relation: relationTree(node.relation, map, depth + 1, nesting),
				expr: event(node.expr)
			})
	}
}

function relationTree<V, D, S, W, E, T>(
	node: RelationExprIr<V, D, S>,
	map: Translation<V, D, S, W, E, T>,
	depth: number,
	nesting: number
): RelationExprIr<W, E, T> {
	map.enter?.(depth)
	const relation = (child: RelationExprIr<V, D, S>) => relationTree(child, map, depth + 1, nesting)
	switch (node.kind) {
		case "bind":
		case "test":
			return Object.freeze({
				...node,
				descriptor: map.descriptor(node.descriptor),
				expr: eventTree(node.expr, map, depth + 1, nesting)
			})
		case "identity":
			return Object.freeze({ ...node, descriptor: map.descriptor(node.descriptor) })
		case "not":
		case "converse":
			return Object.freeze({ ...node, relation: relation(node.relation) })
		case "apply":
			return Object.freeze({ ...node, left: relation(node.left), right: relation(node.right) })
		case "product":
			return Object.freeze({
				...node,
				descriptor: map.descriptor(node.descriptor),
				left: relation(node.left),
				right: relation(node.right)
			})
		case "star":
			return Object.freeze({ ...node, descriptor: map.descriptor(node.descriptor), relation: relation(node.relation) })
	}
}

function testTree<V, D, S, W, E, T>(
	node: EventTestIr<V, D, S>,
	map: Translation<V, D, S, W, E, T>
): EventTestIr<W, E, T> {
	switch (node.kind) {
		case "isEmpty":
		case "isFull":
			return Object.freeze({ ...node, expr: eventTree(node.expr, map) })
		default:
			return Object.freeze({ ...node, left: eventTree(node.left, map), right: eventTree(node.right, map) })
	}
}

export { eventTree, relationTree, testTree }
