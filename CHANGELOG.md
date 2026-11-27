# Changelog

Newest version on top.

## 2.1 (2026-11-27)

- **General and professional.** The repo now describes a generic user and holds no personal details: the instructions, roles, examples, docs, memory and tests were rewritten. `vmem.json` sets the owner to "the user" and the language to the user's own.
- **Voice first.** Anything that needs a screen waits as a parked task for later.
- **`vmem init`** writes the same general rules into new repos.

## 2.0 (2026-11-25)

- **Memory is now a ledger.** `memory/ledger.jsonl` is the single source of truth: append-only events (add, set, done, retire, session) for eleven kinds of items. The v1.0 memory files were migrated into it; `memory/*.md` are now generated views.
- **Boot memory in the README.** The top of the README is generated: the rules, where the last session ended, next steps, items waiting for a screen, pinned memory, active projects, what to load next and the memory code, under a token budget.
- **Packs for voice agents.** Brief and full packs, a spoken recap, a self-check, one page per topic, task packs prepared for one session, a manifest with hashes, and `llms.txt`.
- **The session writes back.** A fixed HANDOFF block, with English or Spanish keys (`core/handoff.md`). It is saved from the phone with a Handoff issue form, or from the computer.
- **Memory code.** Two words that change with every change, so the user can check that Grok read the latest version.
- **Names survive speech recognition.** Aliases, HEARD corrections, and sound-alike matching for English and Spanish names.
- **vmem**, the Rust toolkit: init, build, remember, ingest, done, retire, set, resume, search, context, topic, history, resolve, doctor, stats, and an MCP server for desktop agents.
- **Automation.** CI runs the tests, the doctor and a drift check, and rebuilds the generated files on main. The Handoff workflow saves owner handoffs only.
- **Core.** New `core/essentials.md` (the rules in every boot) and `core/handoff.md`. The system prompt is now 2.0, with a memory protocol section. Roles mention memory where it matters.
- **Docs.** How it works, the protocol, the CLI reference, research notes and a guide to make your own.

## 1.0.1 (2026-11-25)

- The whole repo was translated to English, file names included. Grok still speaks the user's language.

## 1.0 (2026-11-22)

- The repo was reorganized into folders: `core/`, `roles/`, `memory/` and `experiments/`. Grok asked for it (see `experiments/2026-11-22-grok-request.md`).
- New system prompt with identity, tone, voice restrictions, voice format and an escalation protocol.
- Each role has its own file, with an activation header and examples.
- Memory templates: context, preferences and decisions.
- The README explains how to sync the repo with Grok.

## 0.1 (2026-11-22)

- First README with the three roles and the voice rules.
