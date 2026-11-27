# Research notes: memory for voice agents

What limits the memory of voice agents today, and how each part of this repo answers it. Written in November 2026, after version 1.0 of this repo had been used with Grok in a Tesla.

## What limits voice agents

1. **Every session starts blank.** Voice agents run on language models that keep no state between conversations. A hands-on review of Grok in Tesla (March 2026) reported that it retains no memories between sessions, so picking up where you left off is impossible; inside one session it keeps the conversation. Chat apps add memory features, but in-car and embedded voice agents often do not get them.
2. **The first read must be small and fast.** In a voice conversation, silence is felt immediately. Research on streaming voice memory puts the real-time budget around 500 ms and argues that a voice model can use only a handful of high-density items, not the long retrieval lists text agents get. One page fetch at the start of a session is acceptable; a dozen are not.
3. **Context decays inside long sessions.** Rules read at the start can be pushed out of a long conversation. The rules that matter most must be short and sit at the top.
4. **No write access.** The agent can open a public link but cannot commit. Something outside the agent has to close the loop, and it has to work from a phone.
5. **Speech recognition mangles names.** Proper nouns are the most common error. In this repo's own tests, "Grok" comes back as "Groc" and "Grock".
6. **Agents can claim to have read what they did not.** An agent may answer from a cached copy, or improvise when a link fails, and the person cannot see what it read.
7. **The listener can't look at a screen.** Nothing can depend on reading, long lists are useless by ear, and links or ids read aloud are noise.
8. **A public link is public forever.** Anything committed is world-readable and stays in the git history.
9. **Memory becomes instructions.** Whatever the agent reads at boot shapes its behavior, so memory is also an attack surface for injected instructions.

## What good memory systems do

These are general techniques from agent-memory work, adapted here to static files that a voice agent reads by URL:

- **Typed memory** (profile, preferences, decisions, tasks, questions, facts) instead of a transcript dump. Selectivity matters more than volume: not every utterance belongs in memory.
- **Tiered context:** a one-screen summary first, details on demand. Here: boot, topic pages, brief pack, full pack.
- **An append-only log folded into current state,** with supersession instead of deletion, and two clocks: when something was recorded and when it is true.
- **Hybrid recall:** keyword relevance fused with recency, plus tolerance for misheard words.
- **Hard budgets:** pack the most important context under a token limit and say what was left out.
- **Hygiene:** stale tasks, unverified facts, duplicates and contradictions surface on their own.
- **Write gates:** preview before writing, refuse private data, and accept public submissions only from the owner.

## Design choices, problem by problem

| Problem | Choice in this repo |
|---|---|
| Blank sessions | The README itself is the memory: one link restores the context |
| Small, fast first read | A boot memory under 1,400 tokens; everything else loads on demand |
| Context decay | Essentials first and short; the boot is re-readable at any time |
| No write access | A fixed `HANDOFF` block the agent writes; saved from a phone issue form or a desktop agent |
| Misheard names | Aliases on every name, `HEARD` corrections, sound-alike keys for English and Spanish in search and linking |
| Claimed reading | A two-word memory code that changes with every change, plus a self-check page |
| No screen | A spoken recap, and parked tasks for anything visual |
| Public forever | Privacy screening on every write and on every file in CI |
| Memory as instructions | Instruction-like text is refused; behavior lives only in `core/` |
| Stale memory | A memory date, a two-week freshness rule, validity windows, doctor warnings |
| Lost history | Append-only ledger, `history` and `--as-of` |

## What we did not build, and why

- **No embeddings or vector database.** A static repo cannot run a search service for the voice agent, and for a personal memory of hundreds of items, topic pages plus keyword search with sound-alike matching are enough, offline and deterministic.
- **No server.** The point is a plain public link that any agent can open.
- **No language model inside the tool.** `vmem` is deterministic and testable and needs no API keys in CI. The language model is the voice agent itself, which writes the handoff.

## Open questions to test in the car

- How many links Grok opens per turn, and whether it follows raw links or only GitHub pages.
- Whether Grok reads the whole README or cuts it, and whether the boot block survives.
- Whether it says the memory code correctly, and how fast it notices a new version after a push.
- Whether the in-car session keeps a transcript from which the handoff can be copied, or the spoken handoff is the only path.

## Sources

- Carwow, review of Grok in Tesla (March 2026): carwow.co.uk/news/10443/grok-ai-tesla-review
- Tesla support page about Grok: tesla.com/support/grok
- "Streaming Dual-Brain Memory for Real-Time Interaction" (arXiv 2608.26005, August 2026), on latency budgets and top-5 retrieval for speech models
- The llms.txt proposal: llmstxt.org
- The Model Context Protocol specification: modelcontextprotocol.io
