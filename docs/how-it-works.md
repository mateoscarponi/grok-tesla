# How it works

## The problem in one paragraph

A voice agent like Grok in a car is stateless: every session starts blank, and inside a session it can only open a few pages before it has to answer out loud. People repeat themselves, and the agent cannot pick up where the last conversation ended. What the agent *can* do is open a public link. This repo turns one link into a small, complete and verifiable memory, and gives every session a way to write back.

## Two paths

```
READ PATH (start of every session)

  voice agent ── opens the repo link ──► README boot block   (≤ 1,400 tokens)
                                         rules · where we left off · next steps
                                         memory code · what to load next
                                            │
                                            ├─► packs/topics/<topic>.md   when the talk turns to a topic
                                            ├─► packs/custom/<task>.md    a pack prepared for one task
                                            ├─► packs/brief.md            when something is missing
                                            └─► packs/full.md             rarely: everything in one file

WRITE PATH (end of every session)

  voice agent ── HANDOFF block ──► the user saves it ───► vmem ingest ──► memory/ledger.jsonl
                                   (Handoff issue form     closes tasks, adds new items,
                                    or a desktop agent)    skips duplicates, learns
                                                           misheard names, records the session
                                                                │
                                                                └─► vmem build ──► packs, topic pages,
                                                                                   views, sessions,
                                                                                   README block, llms.txt
```

The voice agent never writes to the repo. It only reads, and it writes a block of text at the end that a person or another agent saves.

## The ledger: one source of truth

`memory/ledger.jsonl` holds every memory event, one JSON object per line, and nothing else is memory. Events are only ever appended:

- `add` creates an item of one of eleven kinds: profile, preference, rule, goal, project, person, decision, task, question, fact, term.
- `set` changes fields (a project's stage, a new alias, a pin, "verified today").
- `done` closes a task or answers a question. `retire` marks something that is no longer true.
- `add` with `supersedes` replaces an older item, which stays in the history.
- `session` records one conversation: its summary and the items it created or touched.

Because nothing is deleted, `vmem history <id>` shows how an item changed, and `vmem search --as-of 2026-11-01` shows what memory looked like on that date. Items can also carry a validity window (`valid_from`, `valid_to`): a trip next January is "coming up" today and "current" in January.

## The packs: what the agent reads

| File | Size | What it holds | When the agent reads it |
|---|---|---|---|
| README block / `packs/boot.md` | ≤ 1,400 tokens | Behavior essentials, where we left off, pinned memory, active projects, links to load more, memory code | Always, first |
| `packs/topics/<slug>.md` | small | Everything about one project, person or topic, including recent sessions | When the conversation turns to it |
| `packs/custom/<slug>.md` | chosen budget | Memory selected for one task, prepared on the computer | When the user asks for that pack |
| `packs/brief.md` | ≤ 3,200 tokens | All current memory, most important first, with a note of what did not fit | When something is missing |
| `packs/full.md` | no budget | System prompt, roles, handoff format and all memory | Rarely |
| `packs/spoken.txt` | ≤ 90 words | A recap written to be said aloud | At the greeting |
| `packs/check.md` | small | Questions that prove the memory was loaded | When the user tests the agent |
| `packs/manifest.json` | small | Version, code, size and hash of every file | Tools and audits |
| `llms.txt` | small | Index of all of the above | Agents that look for it |

Every pack is plain Markdown served from `raw.githubusercontent.com`, the easiest format for an agent to read. Budgets are enforced item by item: when a section does not fit, the pack says what was left out and where to find it.

## Why it works for voice

- **A small first read.** The boot carries what a session needs to start, so one fetch is enough, and the rules sit at the top where they are least likely to fall out of a long conversation.
- **Progressive loading.** Topic pages and packs are loaded only when needed, the way a person checks their notes.
- **Written to be heard.** Items are short sentences, the recap is ready to speak, and the rules say never to read links, ids or code aloud.
- **Names survive speech recognition.** Every project, person and term can carry aliases ("Groc" for Grok). A `HEARD` line in a handoff teaches a new one. Search and topic linking use sound-alike keys for English and Spanish, so a misheard "groc tesla" still finds the Grok Tesla project.
- **It proves what it read.** The two-word memory code changes with every change. If the agent says the wrong code, it read an old or imagined copy.
- **Nothing depends on a screen.** Anything visual becomes a parked task for later.

## Automation

- **CI** (`.github/workflows/memory.yml`): runs the tests, `vmem doctor` and `vmem build --check` on every push and pull request. On `main`, if someone edited the ledger by hand (for example on github.com), CI rebuilds the generated files and commits them.
- **Handoff form** (`.github/ISSUE_TEMPLATE/handoff.yml` + `.github/workflows/handoff.yml`): the owner pastes or dictates a handoff from the phone; the workflow saves it, rebuilds, comments what changed and closes the issue. Only the repo owner's issues are saved, because anyone can open an issue on a public repo.
- **MCP** (`vmem mcp`): desktop agents get the same memory as tools, with the same validation.

## Safety and privacy

The repo is public, so `vmem` refuses to save emails, phone numbers, card numbers and secrets, and `vmem doctor` scans every file for them. Memory is read as instructions by the voice agent, so text that tries to give it new instructions ("ignore previous instructions…") is refused too: behavior rules live only in `core/`.

## Reproducible builds

Generated files depend only on the repo's content, never on the clock: dates come from the newest ledger event (the memory clock). The same repo builds byte for byte the same on any machine, which is what lets CI reject hand edits.
