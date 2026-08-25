/**
 * The replica is a value on this execution environment. An in-flight
 * open is not the handle: a refused open leaves the value absent so
 * the next invoke retries.
 */

type Held<T> =
	| { readonly tag: "live"; readonly value: T }
	| { readonly tag: "unavailable"; readonly status: 503; readonly reason: string }

function holdReplica<T>(open: () => Promise<T>): () => Promise<Held<T>> {
	let value: T | undefined
	let opening: Promise<Held<T>> | undefined

	return function acquire(): Promise<Held<T>> {
		if (value !== undefined) {
			return Promise.resolve({ tag: "live", value })
		}
		if (opening === undefined) {
			opening = (async function attempt() {
				try {
					const opened = await open()
					value = opened
					return { tag: "live", value: opened }
				} catch {
					return { tag: "unavailable", status: 503, reason: "replica is unavailable" }
				} finally {
					opening = undefined
				}
			})()
		}
		return opening
	}
}

export type { Held }
export { holdReplica }
