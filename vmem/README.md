# vmem: the voice memory toolkit

A small Rust command-line tool that keeps a public Git repo readable as memory for stateless voice agents. It turns an append-only ledger into token-budgeted packs a voice agent reads by URL, and turns the agent's end-of-session handoff back into memory.

```
cargo install --path vmem        # from the repo root
vmem resume                      # where the last session ended
vmem ingest handoff.txt          # save a session
vmem doctor                      # check everything before a commit
```

The full command reference is in [docs/cli.md](../docs/cli.md); the protocol it implements is in [docs/protocol.md](../docs/protocol.md).

## Map of the code

Each file starts with a comment that explains it. To change a behavior, read the file in this table first.

| File | What it does |
|---|---|
| `src/main.rs` | Command-line parsing (clap) and exit codes |
| `src/app.rs` | Every command, one function each. Start here |
| `src/ledger.rs` | The event format and the append-only `memory/ledger.jsonl` |
| `src/state.rs` | Folds events into current memory: statuses, validity windows, as-of replays, ids |
| `src/build.rs` | Generates the boot, brief, full, spoken, self-check, topic, task and view files, the README block, `llms.txt` and the manifest |
| `src/handoff.rs` | Parses handoff text: English or Spanish keys, bullets, transcripts, issue forms |
| `src/ingest.rs` | Turns a handoff into events: closes tasks, skips duplicates, learns aliases, links topics |
| `src/search.rs` | BM25 search fused with recency, with sound-alike expansion |
| `src/phonetic.rs` | Sound-alike keys for English and Spanish names |
| `src/text.rs` | Accent folding, tokens, slugs, similarity, token estimates, speech cleanup |
| `src/doctor.rs` | Privacy, injection, staleness, duplicate, budget, link and drift checks |
| `src/mcp.rs` | The MCP server over stdio |
| `src/init.rs` | Scaffolds a new memory repo |
| `src/config.rs` | `vmem.json` |
| `src/date.rs`, `src/words.rs` | Calendar math without dependencies; the word lists for the memory code |
| `tests/cli.rs` | End-to-end tests that run the real binary in a temporary repo |

## Principles

- **One source of truth.** Only `memory/ledger.jsonl` is memory; everything else is generated.
- **Reproducible.** Output depends on repo content only, never on the clock, so CI can reject hand edits.
- **Refuse, don't repair.** Invalid events, private data and instruction-like text are rejected before anything is written.
- **Few dependencies.** clap, serde, serde_json, regex and sha2. No network, no database, no model calls.

## Development

```
cargo test --manifest-path vmem/Cargo.toml
```

On Windows with the GNU toolchain, the repo's `.cargo/config.toml` switches to Rust's bundled linker, so a user folder with a space in its name does not break the build.
