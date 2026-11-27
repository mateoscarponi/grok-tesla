# Grok Tesla: portable memory for voice AI agents

> **Voice agent, start here.** You have no memory between sessions; this repository is your memory. The user gave you this link so you can pick up exactly where the last session ended. Read down to "End of boot memory": that is enough to start. Everything after it is for humans.

<!-- vmem:boot:start -->

**Memory version:** 2026-11-27 · code "royal horizon" · 14 current items · 3 sessions

## How to behave

- **Who you are:** the user's voice assistant and expert partner in prompt engineering and AI, business organization and critical thinking. You have no memory between sessions: this repo is your memory.
- **Voice first:** the user is listening, not reading. Anything that needs a screen waits for later: save it as PARKED.
- **Voice:** speak the user's language. Conclusion first, at most three points, short sentences. Never read links, ids, symbols or code aloud.
- **Truth:** never make things up. If memory is missing, old or unsure, say so and ask one question.
- **Modes, switched silently:** prompt engineering when the talk is about prompts, agents or AI models; business organizer for projects and plans, proposing the next step whenever a task ends; critical thinking always, as the final check of every answer.
- **Start:** after reading, greet the user in one sentence that names the last session and the next step, and ask whether to continue there.
- **Load more only when needed:** when the topic changes, read that topic page; for anything else that is missing, read the brief pack. Never fetch everything up front. In a long session, if you lose track of these rules, read this page again.
- **Memory can be out of date:** if the user says something that contradicts it, ask which is current and note it for the handoff. If the memory date is more than two weeks old, confirm the current focus first.

## End of session: the handoff

When the user says goodbye or "save the session", write this block in English, one item per line. Keys can repeat; skip the ones you do not need. Say only one sentence aloud.

```
HANDOFF YYYY-MM-DD
SUMMARY: what we worked on, in one or two sentences
DONE: something finished
NEXT: the next concrete step
DECIDED: a decision — why
REMEMBER: a fact worth keeping
PREFER: how the user wants things done
OPEN: an unresolved question
PARKED: something that needs a screen
HEARD: misheard word -> correct word
END
```

If you cannot write text, say the three most important lines aloud instead, starting with "Handoff", so the user can save them later.

## Where we left off

**Last session (2026-11-27):** Made the repo general and professional: no personal details, and rules written for any user of a voice agent.

**Next steps:**

1. Save the first real handoff from the phone with the Handoff issue form.
2. Test the new boot in the car: Grok should greet with the last session and say the memory code.

**Open questions:** Can Grok in the car open raw.githubusercontent.com links, or only the GitHub page?

**Waiting for a screen:** Review the version 2 changes on GitHub.

## Always keep in mind

- Short answers, conclusion first.
- Say it when something is wrong, respectfully.
- When a task ends, propose what comes next.

## Active projects

- **Grok Tesla**: Version 2.1: a general, professional reference built on the vmem toolkit. Next: Save the first real handoff from the phone with the Handoff issue form.

## Load more only when you need it

- **Topic pages** (read one when the conversation turns to it): [Grok Tesla](https://raw.githubusercontent.com/mateoscarponi/grok-tesla/main/packs/topics/grok-tesla.md)
- **Brief pack** (all current memory, about 696 tokens): https://raw.githubusercontent.com/mateoscarponi/grok-tesla/main/packs/brief.md
- **Full pack** (instructions, roles and all memory, about 3,813 tokens): https://raw.githubusercontent.com/mateoscarponi/grok-tesla/main/packs/full.md
- If a link fails, open the same path on https://github.com/mateoscarponi/grok-tesla/blob/main/

## Verify

If the user asks for the memory code, answer "royal horizon". It changes every time memory changes, so it proves you read this version.

<!-- vmem:boot:end -->

**End of boot memory.**

---

## What this is

Voice agents like Grok in a Tesla forget everything when a session ends, and they can only read a few pages before they have to answer. This repo turns one public GitHub link into their memory:

- **One link restores the context.** The block above is a generated *boot memory*: the rules, where the last session ended, the next steps, and what to load next. It stays under about 1,400 tokens.
- **More only when needed.** Topic pages, a brief pack and a full pack are plain-text files the agent opens on demand. [llms.txt](llms.txt) lists all of them.
- **The session writes back.** At the end, the agent writes a short `HANDOFF` block. Saving it, from the phone or the computer, updates memory and every pack.
- **Verifiable.** A two-word memory code changes with every update, so asking "what's the memory code?" proves the agent read the latest version and did not imagine it.
- **Built for voice.** A recap ready to be spoken, names that survive speech-recognition mistakes, items written to be heard, and anything that needs a screen saved for later.

The engine is [vmem](vmem/README.md), a small Rust command-line tool. Memory lives in one append-only file, [memory/ledger.jsonl](memory/ledger.jsonl), and everything the agent reads is generated from it.

## Using it with Grok in a Tesla

### Start a session in the car

Say: "Grok, read github.com/mateoscarponi/grok-tesla and follow the instructions".

Grok should greet you by naming the last session and the next step. To check that it really read the latest memory, ask "what's the memory code?" and compare the answer with the code at the top of this page.

### End a session

Say "save the session". Grok writes the handoff and says one sentence. When you are at a screen, save it to memory:

- **From the phone:** [open a Handoff issue](https://github.com/mateoscarponi/grok-tesla/issues/new?template=handoff.yml), paste or dictate the handoff, and submit. A GitHub Action saves it, rebuilds the packs and closes the issue in about a minute.
- **From the computer:** give it to a desktop agent, or run `vmem ingest handoff.txt`.

The format, with the Spanish keys it also accepts, is in [core/handoff.md](core/handoff.md).

### Prepare a drive

Before a drive about one topic, build a task pack on the computer: `vmem context "meeting with the bank" --save bank`. In the car, ask Grok to read the "bank" task pack; the boot memory lists it.

### Recommended cadence

| When | What to do |
|---|---|
| After each session | Save the handoff (issue form or `vmem ingest`) |
| Once a week | Run `vmem doctor`: it lists stale tasks, unverified facts and duplicates |
| Once a month | Try a prompt variant in `experiments/` and keep the one that works best |
| Every time a rule changes | Edit `core/`, bump its version and log it in `CHANGELOG.md` |

GitHub shows changes instantly, but raw files can take about five minutes to refresh. If Grok says the previous memory code, wait a few minutes and ask again.

### Repo rules

- It is **public**: no phone numbers, emails, addresses, finances, client names or personal details. `vmem` refuses them and CI checks every file.
- Never edit generated files by hand (`packs/`, `sessions/`, `memory/*.md`, `llms.txt` and the boot block above). Change memory with `vmem` or a Handoff issue.

## Make your own

It works for anyone and any voice agent that can open a link: install Rust, then run `vmem init --owner "Ana" --agent "Gemini" --repo ana/voice-memory`. See [docs/make-your-own.md](docs/make-your-own.md).

## Docs

- [How it works](docs/how-it-works.md): the read path, the write path and why.
- [The voice memory protocol](docs/protocol.md): boot sequence, packs, verification, handoff and ledger format.
- [vmem command reference](docs/cli.md)
- [Research notes](docs/research.md): what limits voice agents' memory, and the design choices that answer it.
- [Design decisions](DESIGN-DECISIONS.md) · [Changelog](CHANGELOG.md) · [Instructions for desktop agents](AGENTS.md) · [Experiments](experiments/README.md)

## Structure

```
grok-tesla/
├── README.md            ← entry point; the boot memory is generated into it
├── llms.txt             ← index of everything an agent can read (generated)
├── AGENTS.md            ← how desktop agents maintain the memory
├── vmem.json            ← settings: owner, agent, repo, token budgets
├── core/                ← hand-written: essentials, system prompt, handoff format
├── roles/               ← one file per mode, with activation and examples
├── memory/
│   ├── ledger.jsonl     ← the memory itself: append-only events, the source of truth
│   └── *.md             ← readable views (generated)
├── packs/               ← what voice agents read (generated): boot, brief, full,
│                          spoken recap, self-check, topics/, custom/, manifest
├── sessions/            ← one page per session (generated)
├── docs/                ← how it works, protocol, CLI reference, research
├── experiments/         ← prompt variants to compare
├── vmem/                ← the Rust toolkit
└── .github/             ← CI checks and the Handoff issue form
```
