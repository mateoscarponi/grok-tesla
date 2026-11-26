# Make your own voice memory

This works for any person and any voice agent that can open a public link: Grok in a car, a phone assistant with web access, a voice mode in a chat app. You need a GitHub account and Rust.

## 1. Install vmem

Install Rust from rustup.rs, then:

```
cargo install --git https://github.com/mateoscarponi/grok-tesla vmem
```

## 2. Create the repo

```
mkdir voice-memory
cd voice-memory
vmem init --owner "Ana" --agent "Gemini" --repo ana/voice-memory --speak "Spanish"
```

This creates `vmem.json`, `core/` (essentials, system prompt, handoff format), a README with the boot block, `AGENTS.md`, a ledger with two starter items, and every generated pack.

## 3. Make it yours

- Edit `core/essentials.md`: who the agent is for you, how it should talk, the safety rules. Keep it short; it goes into every boot.
- Edit `core/system-prompt.md` with the long version. Add files in `roles/` if you want modes, each with an activation section.
- Add memory:

```
vmem remember profile "Ana runs a bakery and is opening a second shop"
vmem remember project "Second shop" --title "Shop two" --state "Looking for a location"
vmem remember preference "Numbers rounded, no decimals"
vmem remember task "Visit the two shortlisted locations"
```

## 4. Publish

Create a **public** GitHub repo with the same name and push. Copy `.github/` from this repo for the checks and the Handoff issue form; the workflows read `vmem.json` and install `vmem` on their own. Create a `handoff` label in the repo so the form can apply it.

## 5. Use it

- Start a session: "Read github.com/ana/voice-memory and follow the instructions."
- Test it: "What's the memory code?" must match the code at the top of the README.
- End a session: "Save the session." Then save the handoff from the Handoff form or with `vmem ingest`.

## Tips

- Keep the boot small: pin only what the agent must always know.
- Prefer many short items over one long one; supersede instead of editing.
- Run `vmem doctor` once a week: it finds stale tasks, unverified facts and duplicates.
- Never put private data in a public repo. `vmem` refuses the obvious cases, but you are the last check.
