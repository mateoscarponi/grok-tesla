//! vmem: the voice memory toolkit.
//!
//! A stateless voice agent (for example Grok in a car) reads a public repo by
//! URL to get its memory back. `vmem` keeps what it reads small, fresh and
//! verifiable, and turns the end-of-session handoff back into memory.
//!
//! Map of the code (each file starts with a short explanation):
//! - `app.rs`: every command, one function each. Start here.
//! - `ledger.rs`: the append-only event log, `memory/ledger.jsonl`.
//! - `state.rs`: folds the log into current memory (statuses, validity, as-of).
//! - `build.rs`: generates packs, topic pages, views, README block, llms.txt.
//! - `handoff.rs` + `ingest.rs`: parse the end-of-session handoff into events.
//! - `search.rs`: BM25 + recency search, tolerant to misheard words.
//! - `phonetic.rs`: sound-alike keys for English and Spanish names.
//! - `doctor.rs`: privacy, injection, staleness, drift and link checks.
//! - `mcp.rs`: the same memory as MCP tools for desktop agents.
//! - `init.rs`: scaffolds a new memory repo for anyone.

mod app;
mod build;
mod config;
mod date;
mod doctor;
mod handoff;
mod ingest;
mod init;
mod ledger;
mod mcp;
mod phonetic;
mod search;
mod state;
mod text;
mod words;

use clap::{Args, Parser, Subcommand};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "vmem",
    version,
    about = "Voice memory toolkit: keeps a public repo readable as memory for stateless voice agents.",
    after_help = "Docs: docs/cli.md · Protocol: docs/protocol.md"
)]
struct Cli {
    /// Repo root. Defaults to the nearest folder that holds vmem.json.
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Write to the ledger without rebuilding the generated files.
    #[arg(long, global = true)]
    no_build: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a new memory repo in the current folder (or --root).
    Init {
        /// Whose memory it is, e.g. "Ana".
        #[arg(long)]
        owner: String,
        /// The voice agent that will read it, e.g. "Grok".
        #[arg(long, default_value = "your voice assistant")]
        agent: String,
        /// GitHub repo, as owner/name.
        #[arg(long)]
        repo: String,
        /// Language the agent speaks with the owner.
        #[arg(long, default_value = "English")]
        speak: String,
        #[arg(long)]
        title: Option<String>,
    },
    /// Regenerate packs, views, topic pages, the README block, llms.txt and the manifest.
    Build {
        /// Only check that generated files are up to date (for CI).
        #[arg(long)]
        check: bool,
    },
    /// Add one item to memory.
    Remember(RememberArgs),
    /// Mark a task done or a question answered.
    Done {
        id: String,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        at: Option<String>,
    },
    /// Retire an item that is no longer true (it stays in the history).
    Retire {
        id: String,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        at: Option<String>,
    },
    /// Change fields of an item.
    Set(SetArgs),
    /// Save an end-of-session handoff. Reads a file, or stdin with "-".
    Ingest {
        #[arg(default_value = "-")]
        input: String,
        /// Where the handoff came from (voice, issue #12, desktop...).
        #[arg(long, default_value = "desktop")]
        source: String,
        /// Show what would change without writing anything.
        #[arg(long)]
        dry_run: bool,
        /// Date of the session, if the handoff has none.
        #[arg(long)]
        at: Option<String>,
        /// Save even if the privacy screen raises an alarm.
        #[arg(long)]
        force: bool,
    },
    /// Show where the last session ended.
    Resume {
        /// A recap ready to be said aloud.
        #[arg(long)]
        spoken: bool,
    },
    /// Search memory (BM25 + recency, tolerant to misheard words).
    Search {
        #[arg(required = true)]
        query: Vec<String>,
        #[arg(short = 'k', long, default_value_t = 8)]
        limit: usize,
        #[arg(long)]
        kind: Option<String>,
        /// Include done, retired and replaced items.
        #[arg(long)]
        all: bool,
        /// Search memory as it was on this date.
        #[arg(long)]
        as_of: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Build a context pack for one task under a token budget.
    Context {
        #[arg(required = true)]
        goal: Vec<String>,
        #[arg(long, default_value_t = 1500)]
        budget: usize,
        /// Save it as packs/custom/<name>.md, rebuilt on every build.
        #[arg(long)]
        save: Option<String>,
        /// Remove a saved task pack instead.
        #[arg(long)]
        drop: bool,
    },
    /// Show the page of one project, person or topic.
    Topic {
        #[arg(required = true)]
        name: Vec<String>,
    },
    /// Show the full history of one item.
    History { id: String },
    /// Show which known names a text mentions, even misheard.
    Resolve {
        #[arg(required = true)]
        text: Vec<String>,
    },
    /// Check privacy, injection, staleness, duplicates, budgets, links and drift.
    Doctor {
        /// Fail on warnings too.
        #[arg(long)]
        strict: bool,
    },
    /// Show counts and pack sizes.
    Stats,
    /// Run as an MCP server over stdio for desktop agents.
    Mcp,
}

#[derive(Args)]
struct RememberArgs {
    /// profile, preference, rule, goal, project, person, decision, task, question, fact or term
    kind: String,
    /// What to remember. For projects, people and terms: a short description (use --title for the name).
    #[arg(required = true)]
    text: Vec<String>,
    #[arg(long)]
    title: Option<String>,
    #[arg(long = "topic")]
    topics: Vec<String>,
    #[arg(long = "tag")]
    tags: Vec<String>,
    /// How the name gets misheard (repeatable).
    #[arg(long = "alias")]
    aliases: Vec<String>,
    /// How to pronounce it.
    #[arg(long)]
    say: Option<String>,
    /// The reason, for decisions.
    #[arg(long)]
    why: Option<String>,
    /// Current stage, for projects and goals.
    #[arg(long)]
    state: Option<String>,
    /// Always show it in the boot pack.
    #[arg(long)]
    pin: bool,
    /// A task that needs a screen.
    #[arg(long)]
    parked: bool,
    /// high, medium or low.
    #[arg(long)]
    confidence: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    valid_from: Option<String>,
    #[arg(long)]
    valid_to: Option<String>,
    /// The id of the item this one replaces.
    #[arg(long)]
    supersedes: Option<String>,
    #[arg(long)]
    at: Option<String>,
    /// Save even if the privacy screen raises an alarm.
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct SetArgs {
    id: String,
    #[arg(long)]
    text: Option<String>,
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    state: Option<String>,
    #[arg(long)]
    why: Option<String>,
    #[arg(long)]
    say: Option<String>,
    #[arg(long)]
    confidence: Option<String>,
    #[arg(long)]
    valid_from: Option<String>,
    #[arg(long)]
    valid_to: Option<String>,
    #[arg(long = "alias")]
    aliases: Vec<String>,
    #[arg(long = "topic")]
    topics: Vec<String>,
    #[arg(long = "tag")]
    tags: Vec<String>,
    #[arg(long)]
    pin: bool,
    #[arg(long, conflicts_with = "pin")]
    unpin: bool,
    #[arg(long)]
    parked: bool,
    #[arg(long, conflicts_with = "parked")]
    unparked: bool,
    /// Confirm the item is still true today.
    #[arg(long)]
    verified: bool,
    #[arg(long)]
    note: Option<String>,
    #[arg(long)]
    at: Option<String>,
    #[arg(long)]
    force: bool,
}

fn flag(on: bool, off: bool) -> Option<bool> {
    if on {
        Some(true)
    } else if off {
        Some(false)
    } else {
        None
    }
}

fn read_input(input: &str) -> Result<String, String> {
    if input == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).map_err(|e| format!("cannot read stdin: {e}"))?;
        Ok(s)
    } else {
        std::fs::read_to_string(input).map_err(|e| format!("cannot read {input}: {e}"))
    }
}

fn run(cli: Cli) -> app::Out {
    let nb = cli.no_build;
    if let Cmd::Init { owner, agent, repo, speak, title } = &cli.cmd {
        let root = match &cli.root {
            Some(r) => r.clone(),
            None => std::env::current_dir().map_err(|e| e.to_string())?,
        };
        let opts = init::Options {
            owner: owner.clone(),
            agent: agent.clone(),
            repo: repo.clone(),
            speak: speak.clone(),
            title: title.clone(),
        };
        return app::init_cmd(&root, &opts);
    }
    let root = config::find_root(cli.root.as_deref())?;
    match cli.cmd {
        Cmd::Init { .. } => unreachable!("handled above"),
        Cmd::Build { check } => app::build_cmd(&root, check),
        Cmd::Remember(a) => app::remember(
            &root,
            app::Remember {
                kind: a.kind,
                text: a.text.join(" "),
                title: a.title,
                topics: a.topics,
                tags: a.tags,
                aliases: a.aliases,
                say: a.say,
                why: a.why,
                state: a.state,
                pin: a.pin,
                parked: a.parked,
                confidence: a.confidence,
                source: a.source,
                valid_from: a.valid_from,
                valid_to: a.valid_to,
                supersedes: a.supersedes,
                at: a.at,
                force: a.force,
            },
            nb,
        ),
        Cmd::Done { id, note, at } => app::close(&root, "done", &id, note, at.as_deref(), nb),
        Cmd::Retire { id, reason, at } => app::close(&root, "retire", &id, reason, at.as_deref(), nb),
        Cmd::Set(a) => app::set(
            &root,
            &a.id,
            app::SetFields {
                text: a.text,
                title: a.title,
                state: a.state,
                why: a.why,
                say: a.say,
                confidence: a.confidence,
                valid_from: a.valid_from,
                valid_to: a.valid_to,
                aliases: a.aliases,
                topics: a.topics,
                tags: a.tags,
                pin: flag(a.pin, a.unpin),
                parked: flag(a.parked, a.unparked),
                verified: a.verified,
                note: a.note,
                at: a.at,
                force: a.force,
            },
            nb,
        ),
        Cmd::Ingest { input, source, dry_run, at, force } => {
            let text = read_input(&input)?;
            app::ingest_cmd(&root, &text, &source, dry_run, at.as_deref(), force, nb)
        }
        Cmd::Resume { spoken } => app::resume(&root, spoken),
        Cmd::Search { query, limit, kind, all, as_of, json } => {
            app::search_cmd(&root, &query.join(" "), limit, kind.as_deref(), all, as_of.as_deref(), json)
        }
        Cmd::Context { goal, budget, save, drop } => {
            if drop {
                app::drop_pack(&root, &goal.join(" "), nb)
            } else {
                app::context_cmd(&root, &goal.join(" "), budget, save.as_deref(), nb)
            }
        }
        Cmd::Topic { name } => app::topic_cmd(&root, &name.join(" ")),
        Cmd::History { id } => app::history_cmd(&root, &id),
        Cmd::Resolve { text } => app::resolve_cmd(&root, &text.join(" ")),
        Cmd::Doctor { strict } => app::doctor_cmd(&root, strict),
        Cmd::Stats => app::stats_cmd(&root),
        Cmd::Mcp => mcp::serve(root).map(|_| String::new()).map_err(app::Failure::Error),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            ExitCode::SUCCESS
        }
        Err(app::Failure::Check(out)) => {
            println!("{out}");
            ExitCode::from(1)
        }
        Err(app::Failure::Error(msg)) => {
            eprintln!("vmem: {msg}");
            ExitCode::from(2)
        }
    }
}
