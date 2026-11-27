# vmem command reference

`vmem` finds the repo root by walking up to the folder that holds `vmem.json` (or use `--root <path>`). Every command that writes validates the new events against the whole ledger, appends them, and rebuilds the generated files; add `--no-build` to skip the rebuild.

Exit codes: `0` success, `1` a check found problems (`build --check`, `doctor`), `2` the command could not run.

## Install or run

```
cargo install --path vmem                                    # once, then: vmem <command>
cargo run --release --manifest-path vmem/Cargo.toml -- <command>   # without installing
```

## Reading

| Command | What it does |
|---|---|
| `vmem resume` | Where the last session ended: summary, next steps, open questions, parked items |
| `vmem resume --spoken` | The same, as a recap ready to be said aloud |
| `vmem search <words>` | BM25 relevance fused with recency; sound-alike words match too. `-k 5`, `--kind decision`, `--all` (include closed items), `--as-of 2026-11-01`, `--json` |
| `vmem topic <name>` | The page of a project, person or topic; misheard names work |
| `vmem context <goal> --budget 1500` | A pack of the memory most relevant to one task, under a token budget |
| `vmem history <id>` | Every change to one item, what it replaced, and the sessions that mentioned it |
| `vmem resolve <text>` | Which known names a text mentions, exact or sound-alike, with a score |
| `vmem stats` | Counts by kind and the size of every pack |

## Writing

| Command | What it does |
|---|---|
| `vmem remember <kind> <text>` | Add one item. Options: `--title`, `--topic` (repeatable), `--tag`, `--alias` (how the name gets misheard), `--say`, `--why`, `--state`, `--pin`, `--parked`, `--confidence high/medium/low`, `--valid-from`, `--valid-to`, `--supersedes <id>`, `--source`, `--at`, `--force` |
| `vmem ingest <file>` | Save a session handoff (`-` reads stdin). `--dry-run` previews, `--source "issue #12"` records the origin (a numbered source is never saved twice), `--at` sets the date when the handoff has none |
| `vmem done <id>` | Close a task or answer a question. `--note` |
| `vmem retire <id>` | Mark an item as no longer true; it stays in the history. `--reason` |
| `vmem set <id>` | Change fields: `--text`, `--title`, `--state`, `--why`, `--say`, `--confidence`, `--valid-from`, `--valid-to`, `--alias`, `--topic`, `--tag`, `--pin`/`--unpin`, `--parked`/`--unparked`, `--verified`, `--note` |
| `vmem context <goal> --save <name>` | Save a task pack as `packs/custom/<name>.md`, rebuilt on every build and listed in the boot. `--drop` removes it |

Kinds: `profile`, `preference`, `rule`, `goal`, `project`, `person`, `decision`, `task`, `question`, `fact`, `term`.

## Maintenance

| Command | What it does |
|---|---|
| `vmem build` | Regenerate packs, topic pages, task packs, views, sessions, the README block, `llms.txt` and the manifest. Removes stale generated files |
| `vmem build --check` | Only check that the generated files are up to date (CI) |
| `vmem doctor` | Ledger problems, private data in any file, instruction-like memory, stale tasks and questions, unverified facts, duplicates, Spanish text, too many pins, README markers, budgets, broken links and drift. `--strict` fails on warnings too |
| `vmem init --owner <name> --repo <owner/name>` | Create a new memory repo. Options: `--agent`, `--speak`, `--title` |
| `vmem mcp` | Serve the memory to desktop agents over MCP (stdio) |

## Examples

```
vmem remember project "Online course about prompt design" --title "Prompt Course" --alias "prom course" --state "Outline ready"
vmem remember decision "Launch on Monday" --why "fewer people travel" --topic prompt-course
vmem remember fact "The conference runs March 3 to 5" --valid-from 2027-03-03 --valid-to 2027-03-05
vmem remember preference "Very short answers" --supersedes pref-1
vmem set proj-1 --alias "groc tesla"
vmem done task-3 --note "recorded and sent"
vmem ingest handoff.txt --dry-run
vmem search "bank meeting" --kind task
vmem context "prepare the bank meeting" --save bank
```

## MCP tools

`vmem mcp` speaks JSON-RPC 2.0 over stdio (MCP protocol versions 2024-11-05 to 2025-11-25) and offers:

| Tool | Arguments |
|---|---|
| `memory_resume` | `spoken` |
| `memory_search` | `query`, `limit`, `kind`, `include_closed` |
| `memory_topic` | `name` |
| `memory_context` | `goal`, `budget` |
| `memory_remember` | `kind`, `text`, `title`, `why`, `topics`, `aliases`, `pin`, `parked` |
| `memory_complete` | `id`, `note` |
| `memory_retire` | `id`, `reason` |
| `memory_ingest_handoff` | `text`, `dry_run`, `source` |
| `memory_doctor` | none |

## Settings: vmem.json

| Field | Default | Meaning |
|---|---|---|
| `title`, `tagline` | | Shown at the top of every pack and in `llms.txt` |
| `owner`, `agent` | | Whose memory it is, and the voice agent that reads it |
| `repo`, `branch` | `main` | Used to build every raw link |
| `speak` | English | The language the agent speaks with the owner |
| `budgets.boot`, `budgets.brief` | 1400, 3200 | Token budgets |
| `budgets.full` | 16000 | Doctor warns above it |
| `budgets.spoken_words` | 90 | Length of the spoken recap |
| `stale_days` | task 30, question 30, project 60, fact 180 | When doctor flags items as stale |
| `packs` | `[]` | Saved task packs (managed by `vmem context --save`) |

Set `VMEM_TODAY=YYYY-MM-DD` to fix "today" (tests, replays).
