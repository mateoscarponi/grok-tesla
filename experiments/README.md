# Experiments

Prompt variants to compare how Grok answers in the car.

## How to run an experiment

1. Copy the file you want to change to `experiments/YYYY-MM-DD-name.md`.
2. Write at the top what changes and what you expect to improve.
3. In the car, tell Grok to read that file instead of the original.
4. Try the same questions with both versions.
5. Write down the result below. If the variant wins, move it to `core/` or `roles/` and log it in `CHANGELOG.md`.

## Fixed test questions

Behavior:

1. "Build me a prompt so an agent summarizes my emails."
2. "I have three projects, which one do I start with this week?"
3. "I want to quit everything and start something new tomorrow."

Memory (the answers are in `packs/check.md`):

4. "What did we do last time?"
5. "What's the memory code?"
6. "What's the next step?"
7. Mention a project with a slightly wrong name, and see whether Grok still finds it.

## What to measure

- Did it answer briefly, with the conclusion first?
- Did it use the right role without being asked?
- Did it propose a useful next step?
- Did it make anything up?
- Did it greet with the last session and the next step?
- Did it say the right memory code?
- Did it open the topic page when the topic changed, and only then?
- Did it write a valid handoff at the end (`vmem ingest --dry-run` accepts it)?

## Results

| Date | Variant | Winner | Note |
|---|---|---|---|
| | | | |

## Variants

- [2026-11-22-grok-request.md](2026-11-22-grok-request.md): the request Grok made to redesign this repo. It is the origin of version 1.0.
