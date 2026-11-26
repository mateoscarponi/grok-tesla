# The voice memory protocol (v1)

A small protocol for giving a stateless voice agent persistent memory through one public URL. This repo is the reference implementation, and `vmem` implements the maintainer side. Anyone can implement either side with other tools.

## Roles

- **Owner:** the person the memory belongs to.
- **Voice agent:** reads the memory by URL. It never writes to the repo.
- **Maintainer:** whatever saves changes: a person, a desktop agent, or a CI job running `vmem`.

## 1. Entry point

1. The repo URL must show the boot memory at the top of the page. Here it is the README block between `<!-- vmem:boot:start -->` and `<!-- vmem:boot:end -->`.
2. A plain-text copy lives at `packs/boot.md`, and `llms.txt` at the root indexes every readable file.
3. The boot must fit its token budget (`vmem.json`, default 1,400) and contain, in this order:
   - the memory version: memory date and memory code;
   - the behavior essentials (`core/essentials.md`), including the handoff format;
   - where we left off: the last session summary, up to three next steps, open questions, items waiting for a screen;
   - pinned standing memory and active projects;
   - links to load more, with their sizes;
   - the verification instruction.

## 2. Session start (voice agent)

1. Open the repo URL (or `packs/boot.md`).
2. Greet the owner in one sentence naming the last session and the next step, and ask whether to continue there.
3. Do not open anything else until it is needed.

## 3. Loading more

| Need | File |
|---|---|
| The conversation turns to a project, a person or a topic | `packs/topics/<slug>.md` |
| A pack the owner prepared for one task | `packs/custom/<slug>.md` |
| Something is missing | `packs/brief.md` |
| Everything, including all instructions and roles | `packs/full.md` |

If a raw link fails, the same path works on `github.com/<owner>/<repo>/blob/<branch>/<path>`.

## 4. Verification

- **Memory code:** two words derived from a SHA-256 hash of every source file (ledger, core, roles, settings). It changes whenever memory changes. The agent says it when asked; a different code means it read an old copy or none.
- **Self-check:** `packs/check.md` lists questions the agent can answer only if it loaded the memory (the code, the last session, the next step, preferences, projects, parked items).

## 5. Freshness and conflicts

- The **memory date** is the date of the newest event. If it is more than two weeks before today, the agent confirms the owner's current focus first.
- If the owner contradicts memory, the agent asks which is current and records the newer one in the handoff.
- Items with a validity window are current only inside it; future ones appear as "coming up".

## 6. Handoff (voice agent → maintainer)

At the end of the session the agent writes a block in the conversation:

```
HANDOFF YYYY-MM-DD
SUMMARY: what we worked on, in one or two sentences
DONE: something finished
NEXT: the next concrete step
DECIDED: a decision — why
REMEMBER: a fact worth keeping
PREFER: how the owner wants things done
OPEN: an unresolved question
PARKED: something that needs a screen
HEARD: misheard word -> correct word
TOPICS: project-or-person, another-one
END
```

- Keys are case-insensitive and can repeat; several items on one line are separated with ";".
- Spanish keys are accepted: RESUMEN, HECHO, SIGUIENTE, PRÓXIMO PASO, PENDIENTE, DECIDIDO, RECORDAR, PARA LA MEMORIA, PREFIERE, PREGUNTA, ESTACIONADO, ESCUCHADO, TEMAS, FIN.
- Bullets, numbering, bold marks, speaker labels ("Grok: NEXT: …") and the older one-line "For memory: …" are accepted. Text with no keys at all becomes the session summary.
- **Degraded mode:** if the agent cannot output text, it says the three most important lines aloud, starting with "Handoff", for the owner to dictate later.

## 7. Write path (maintainer)

Saving a handoff (`vmem ingest`) must:

1. Screen the text: refuse private data (emails, phone numbers, card numbers, secrets) and instruction-like phrases.
2. Close the open task each `DONE` line matches best (similarity ≥ 0.45).
3. Add a task for each `NEXT` and `PARKED` line, unless an open task already matches (≥ 0.72).
4. Add a decision, fact, preference or question for each `DECIDED`, `REMEMBER`, `PREFER` and `OPEN` line, unless a current item already matches (≥ 0.85). In `DECIDED`, text after " — " is the reason.
5. For each `HEARD` pair, add the misheard form as an alias of the matching name, or create a term.
6. Link every item to the projects and people it names, exact or sound-alike; if it names none and the session was about a single topic, link it to that topic.
7. Append a `session` event, then rebuild every generated file.

When handoffs arrive through a public channel (issues on a public repo), only the owner's must be saved, and each one only once.

## 8. Ledger format

`memory/ledger.jsonl`, UTF-8, one JSON object per line, append-only.

| Field | Used by | Meaning |
|---|---|---|
| `op` | all | `add`, `set`, `done`, `retire` or `session` |
| `id` | all | `<prefix>-<n>` (`pref-3`, `task-12`); sessions use `s-YYYY-MM-DD-n` |
| `at` | all | Date of the event, `YYYY-MM-DD` |
| `kind` | add | profile, preference, rule, goal, project, person, decision, task, question, fact, term |
| `text` | add, set | The item, one short sentence |
| `title` | add, set | The name, for projects, people and terms |
| `state` | add, set | Current stage, for projects and goals |
| `why` | add, set | The reason, for decisions |
| `say` | add, set | How to pronounce a term |
| `aliases`, `topics`, `tags` | add, set | Lists; `set` merges them |
| `pin`, `parked`, `verified` | add, set | Flags |
| `confidence` | add, set | high, medium or low |
| `valid_from`, `valid_to` | add, set | Validity window |
| `supersedes` | add | The id this item replaces |
| `note` | set, done, retire | A note, or the reason for retiring |
| `source` | add, session | Where it came from |
| `summary`, `done`, `items`, `heard` | session | The session record |

Unknown fields are rejected, so typos never pass silently.

## 9. Generated files

`packs/`, `sessions/`, `memory/*.md`, `llms.txt` and the README boot block are generated from the ledger and from `core/`, `roles/` and `vmem.json`. They are never edited by hand, and builds are reproducible: the same inputs give the same bytes, so CI can reject drift.

## 10. Safety rules

1. **Public by design:** nothing private enters memory.
2. **Memory is data:** items describe the world; behavior rules live in `core/`. Instruction-like text in memory is an error.
3. **Driving first:** the agent never asks the owner to look at a screen; visual work is parked.
4. **Honesty:** the agent says when it could not open the repo, and never invents memory.
