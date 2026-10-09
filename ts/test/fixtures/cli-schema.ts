import { str, u64 } from "../../src/fields.ts"
import { relation } from "../../src/relation.ts"
import { schema } from "../../src/schema.ts"
import { key } from "../../src/statements.ts"

const Note = relation("Note", { id: u64, text: str })

export const App = schema("Notes", { Note }, [key(Note, ["id"])])
