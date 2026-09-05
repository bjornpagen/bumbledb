/**
 * The example is an API surface; this page only documents it. No database
 * import exists in any client/page module — native work is server-route
 * only (APP-01).
 */
export default function Home() {
	return (
		<main>
			<h1>bumbledb notes example</h1>
			<p>
				Authenticated JSON API: <code>GET/POST /api/notes</code>,{" "}
				<code>GET/PATCH /api/notes/[id]</code>, <code>POST /api/notes/[id]/attachment</code>. See the README for
				the local development and deployment runbooks.
			</p>
		</main>
	)
}
