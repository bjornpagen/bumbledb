/** One structural translation of the native grammar. Query builders retain
 * variable objects and owned descriptors; the wire uses ordinals and bytes.
 * This visits all written operands and never evaluates or simplifies them. */
import type { EventExprIr, EventTestIr, RelationExprIr } from "#native.ts"

interface Translation<V, D, W, E> {
	readonly variable: (value: V) => W
	readonly descriptor: (value: D) => E
	readonly enter?: (depth: number) => void
}

function eventTree<V, D, W, E>(node: EventExprIr<V, D>, map: Translation<V, D, W, E>, depth = 1): EventExprIr<W, E> {
	map.enter?.(depth)
	const event = (child: EventExprIr<V, D>) => eventTree(child, map, depth + 1)
	switch (node.kind) {
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
			return Object.freeze({ ...node, relation: relationTree(node.relation, map, depth + 1) })
		case "modal":
			return Object.freeze({ ...node, relation: relationTree(node.relation, map, depth + 1), expr: event(node.expr) })
	}
}

function relationTree<V, D, W, E>(
	node: RelationExprIr<V, D>,
	map: Translation<V, D, W, E>,
	depth: number
): RelationExprIr<W, E> {
	map.enter?.(depth)
	const relation = (child: RelationExprIr<V, D>) => relationTree(child, map, depth + 1)
	switch (node.kind) {
		case "bind":
		case "test":
			return Object.freeze({
				...node,
				descriptor: map.descriptor(node.descriptor),
				expr: eventTree(node.expr, map, depth + 1)
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

function testTree<V, D, W, E>(node: EventTestIr<V, D>, map: Translation<V, D, W, E>): EventTestIr<W, E> {
	switch (node.kind) {
		case "isEmpty":
		case "isFull":
			return Object.freeze({ ...node, expr: eventTree(node.expr, map) })
		default:
			return Object.freeze({ ...node, left: eventTree(node.left, map), right: eventTree(node.right, map) })
	}
}

export { eventTree, relationTree, testTree }
