# Design decisions

Why the repo is built this way. If you change any of this, write it down here with the date.

1. **The README is the entry point and stands on its own.** (2026-11-22)
   Grok in the car may not follow every link. That is why the README has the essentials (identity, driving rules, roles in summary) and the other files expand on it. If Grok only reads the README, it still works well.

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
