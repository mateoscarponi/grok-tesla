# Handoff format

The handoff is how a session writes back. The voice agent cannot push to GitHub, so at the end of every session it writes a short, fixed-format block. The user (or a desktop agent) saves it, and the next session starts exactly there.

## When

When the user says goodbye, says "save the session", or the conversation is clearly ending. Write the block in the chat; say only a one-sentence summary aloud.

## Format

An example (not real memory):

```
HANDOFF 2026-11-25
SUMMARY: We planned the demo launch and picked a date.
DONE: Wrote the launch checklist
NEXT: Record the demo video
NEXT: Send the video to the team for review
DECIDED: Launch on Monday — fewer people travel that day
REMEMBER: The demo must stay under two minutes
PREFER: Round numbers when talking about money
OPEN: Which price for the first ten clients?
PARKED: Review the slide colors
HEARD: Groc tesla -> Grok Tesla
TOPICS: grok-tesla
END
```

## Keys

| Key | Spanish keys also accepted | Becomes in memory |
|---|---|---|
| `SUMMARY` | RESUMEN | The session summary (the first line of the next boot) |
| `DONE` | HECHO, LISTO | Closes the open task it matches best |
| `NEXT` | SIGUIENTE, PRÓXIMO PASO, PENDIENTE | A new task, unless the same one is already open |
| `DECIDED` | DECIDIDO, DECISIÓN | A decision; text after " — " is the reason |
| `REMEMBER` | RECORDAR, PARA LA MEMORIA | A fact |
| `PREFER` | PREFIERE, PREFERENCIA | A preference |
| `OPEN` | PREGUNTA, DUDA | An open question |
| `PARKED` | ESTACIONADO, PARA DESPUÉS | A task that needs a screen |
| `HEARD` | ESCUCHADO, CORRECCIÓN | Teaches memory how a name gets misheard |
| `TOPICS` | TEMAS | Links the session to projects or people |

## Rules

- **English**, whatever language the conversation was in: the repo is written in English.
- **One item per line.** Keys can repeat. Several items on one line can be separated with ";". Skip keys with nothing to say.
- **Concrete and short:** "Record the demo video", not "maybe think about the video at some point".
- **No private data:** the repo is public. No phone numbers, emails, addresses, money details or client names. They are refused when saving.
- **Facts, not instructions:** "The user prefers round numbers", never "always answer with round numbers or else". Behavior rules live in `core/`, not in memory.

## If text is not possible

Say the three most important lines aloud, starting with "Handoff", for example: "Handoff. Next: record the demo video. Decided: launch on Monday. Remember: the demo stays under two minutes." The user can dictate them into the Handoff form later, in Spanish if needed; the keys are recognized in both languages.

## How it gets saved

- **From the phone:** open a [Handoff issue](https://github.com/mateoscarponi/grok-tesla/issues/new?template=handoff.yml), paste or dictate the block, submit. A GitHub Action saves it, rebuilds the packs, comments what changed and closes the issue. Only the repo owner's issues are saved.
- **From the computer:** `vmem ingest handoff.txt` (or give it to a desktop agent, which runs the same command).
- **Preview first:** `vmem ingest handoff.txt --dry-run` shows every change without writing.

## What happens to it

`vmem ingest` closes the tasks reported as done, adds new tasks, decisions, facts, preferences and questions while skipping duplicates, learns misheard names as aliases, links every item to the projects it names (even misheard), records the session, and rebuilds every pack. The text is screened for private data and for instruction-like phrases first.
