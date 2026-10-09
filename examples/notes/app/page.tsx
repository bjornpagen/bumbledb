/** The example is an API; this page documents it and imports nothing from the database. */
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
