# Design decisions

Why the repo is built this way. If you change any of this, write it down here with the date.

1. **The README is the entry point and stands on its own.** (2026-11-22)
   Grok in the car may not follow every link. That is why the README has the essentials and the other files expand on it. If Grok only reads the README, it still works well.

2. **Links to the "raw" version of each file.** (2026-11-22)
   `raw.githubusercontent.com` URLs return plain text, which is easier for an agent to read than the GitHub page.

3. **Public repo, no private data.** (2026-11-22)
   Grok can only open public links. Anyone can read what is written here.

4. **Spanish and voice first.** (2026-11-22)
   Mateo speaks Spanish while driving. Every rule favors short answers that are easy to listen to.

5. **Driving safety is above any role.** (2026-11-22)

6. **Changes with traceability.** (2026-11-22)
   Every change goes to `CHANGELOG.md`; every prompt test goes to `experiments/`.

7. **The repo is written in English; Grok still speaks Mateo's language.** (2026-11-25)
   English makes the repo readable for any agent and any person, and it costs fewer tokens. The instructions still tell Grok to speak Spanish with Mateo.

8. **One ledger is the only memory; everything Grok reads is generated.** (2026-11-25)
   Hand-kept memory files drift apart and lose history. An append-only ledger keeps every change, and generated packs can never disagree with it.

9. **The boot memory lives inside the README, under a budget.** (2026-11-25)
   Opening the repo link is enough to start, with no second fetch. The budget (1,400 tokens) keeps the first read fast, and the rules go first so they do not fall out of a long conversation.

10. **Load more only on demand.** (2026-11-25)
    Topic pages, task packs, the brief pack and the full pack exist so the boot can stay small. Grok opens them when the conversation needs them.

11. **A memory code proves what was read.** (2026-11-25)
    Two easy words derived from a hash of all sources. A voice agent can claim to have read a link it could not open; asking for the code exposes it.

12. **The session writes back through a fixed handoff; Grok never writes to the repo.** (2026-11-25)
    Grok cannot commit. A short block with fixed keys is easy for Grok to write and for `vmem` to parse, in English or Spanish, and Mateo can save it from the phone.

13. **Names carry aliases, and matching is sound-alike.** (2026-11-25)
    Speech recognition fails most on names. Aliases, HEARD corrections and phonetic keys keep a misheard name pointing at the right memory.

14. **Builds are reproducible, and generated files are never edited by hand.** (2026-11-25)
    Output depends only on repo content, never on the clock, so CI can reject hand edits and rebuild after a web edit of the ledger.

15. **Screen every write; memory holds facts, not instructions.** (2026-11-25)
    The repo is public and Grok treats what it reads as instructions. Private data and instruction-like text are refused; behavior rules live only in `core/`. Only the owner's Handoff issues are saved.

16. **No embeddings, no server, no model calls in the toolkit.** (2026-11-25)
    A static public link is the whole point. Keyword search with sound-alike matching and topic pages are enough for a personal memory, and they stay offline, deterministic and testable.
