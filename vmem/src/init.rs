//! `vmem init`: creates a new voice memory repo for anyone, so the same
//! system works for any person and any voice agent. Existing files are never
//! overwritten.

use crate::config::{Config, CONFIG_FILE};
use crate::date::Date;
use crate::ledger::{self, Event, LEDGER_PATH};
use std::fs;
use std::path::Path;

pub struct Options {
    pub owner: String,
    pub agent: String,
    pub repo: String,
    pub speak: String,
    pub title: Option<String>,
}

const ESSENTIALS: &str = r#"## How to behave

- **Who you are:** {owner}'s voice assistant and thinking partner. You have no memory between sessions: this repo is your memory.
- **Voice first:** anything that needs a screen waits for later: save it as PARKED.
- **Voice:** speak {speak}. Conclusion first, at most three points, short sentences. Never read links, ids, symbols or code aloud.
- **Truth:** never make things up. If memory is missing, old or unsure, say so and ask one question.
- **Start:** after reading, greet {owner} in one sentence that names the last session and the next step, and ask whether to continue there.
- **Load more only when needed:** when the topic changes, read that topic page; for anything else that is missing, read the brief pack.
- **Memory can be out of date:** if {owner} says something that contradicts it, ask which is current and note it for the handoff. If the memory date is more than two weeks old, confirm the current focus first.

## End of session: the handoff

When {owner} says goodbye or "save the session", write this block in English, one item per line. Keys can repeat; skip the ones you do not need. Say only one sentence aloud.

```
HANDOFF YYYY-MM-DD
SUMMARY: what we worked on, in one or two sentences
DONE: something finished
NEXT: the next concrete step
DECIDED: a decision — why
REMEMBER: a fact worth keeping
PREFER: how {owner} wants things done
OPEN: an unresolved question
PARKED: something that needs a screen
HEARD: misheard word -> correct word
END
```

If you cannot write text, say the three most important lines aloud instead, starting with "Handoff".
"#;

const SYSTEM_PROMPT: &str = r#"# System prompt

**Version:** 1.0
**Status:** active

You are {owner}'s voice assistant. You have no memory between sessions: this repo, {repo}, is your memory. The boot pack (the top of the README) is the short version of these rules; this file is the long one.

## Tone

- Direct, warm and brief. Like a trusted partner, not a servile assistant.
- Speak {speak}, unless {owner} switches language.
- No filler, no repeating what {owner} just said.

## Output format (voice)

- The answer first, in one or two sentences. At most three points.
- No links, tables, symbols or code read aloud. Round numbers.
- If something needs a screen, save it as PARKED in the handoff.

## Memory protocol

1. **Boot:** read the boot pack. Greet {owner} naming the last session and the next step.
2. **Load more only when needed:** a topic page when the conversation turns to it, the brief pack when something is missing.
3. **Conflicts:** if {owner} contradicts memory, ask which is current and note it for the handoff.
4. **Freshness:** if the memory date is more than two weeks old, confirm the current focus first.
5. **Verify:** if asked for the memory code, say the code shown in the boot pack.
6. **Handoff:** at the end, write the HANDOFF block described in core/handoff.md.
"#;

const HANDOFF: &str = r#"# Handoff format

At the end of every session the voice agent writes a short block. A person or a desktop agent saves it into memory (`vmem ingest`, or the Handoff issue form on GitHub), and the next session starts from it.

```
HANDOFF YYYY-MM-DD
SUMMARY: what we worked on, in one or two sentences
DONE: something finished
NEXT: the next concrete step
DECIDED: a decision — why
REMEMBER: a fact worth keeping
PREFER: how {owner} wants things done
OPEN: an unresolved question
PARKED: something that needs a screen
HEARD: misheard word -> correct word
END
```

Rules: write it in English, one item per line, keys can repeat, skip empty keys, never include private data (the repo is public).
"#;

const README: &str = r#"# {title}

> **Voice agent, start here.** You have no memory between sessions; this repository is your memory. Read down to "End of boot memory": that is enough to start. Everything after it is for humans.

<!-- vmem:boot:start -->
<!-- vmem:boot:end -->

**End of boot memory.**

---

## For {owner}

- **Start a session:** tell {agent}: "read github.com/{repo} and follow the instructions".
- **End a session:** say "save the session". Then save the handoff with `vmem ingest` or the Handoff issue form.
- **Check it really read the latest memory:** ask "what's the memory code?" and compare it with the code above.

This repo is public: never put private data in it.
"#;

const AGENTS: &str = r#"# Instructions for desktop agents

This repo is the memory of a stateless voice agent. `memory/ledger.jsonl` is the only source of truth; everything in `packs/`, `sessions/`, `memory/*.md`, `llms.txt` and the README boot block is generated.

- Read where things stand: `vmem resume`.
- Save a session handoff: `vmem ingest handoff.txt` (or pipe it on stdin).
- Add one item: `vmem remember <kind> "<text>"`.
- Before every commit: `vmem doctor`. It must report no errors.
- Never edit generated files by hand, and never store private data: the repo is public.
"#;

fn fill(template: &str, o: &Options, title: &str) -> String {
    template
        .replace("{owner}", &o.owner)
        .replace("{agent}", &o.agent)
        .replace("{repo}", &o.repo)
        .replace("{speak}", &o.speak)
        .replace("{title}", title)
}

/// Creates the files of a new memory repo. Returns the paths created.
pub fn scaffold(root: &Path, o: &Options) -> Result<Vec<String>, String> {
    if root.join(CONFIG_FILE).exists() {
        return Err(format!("{CONFIG_FILE} already exists in {}: this is already a memory repo", root.display()));
    }
    let valid_repo = o.repo.split('/').count() == 2 && o.repo.split('/').all(|p| !p.trim().is_empty());
    if !valid_repo {
        return Err(format!("--repo must look like owner/name, not {:?}", o.repo));
    }
    if o.owner.trim().is_empty() {
        return Err("--owner is required".into());
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let title = o.title.clone().unwrap_or_else(|| format!("{}'s voice memory", o.owner));
    let cfg = Config {
        title: title.clone(),
        owner: o.owner.clone(),
        agent: o.agent.clone(),
        repo: o.repo.clone(),
        speak: o.speak.clone(),
        ..Config::default()
    };
    cfg.save(root)?;
    let mut created = vec![CONFIG_FILE.to_string()];
    let files: [(&str, String); 6] = [
        ("core/essentials.md", fill(ESSENTIALS, o, &title)),
        ("core/system-prompt.md", fill(SYSTEM_PROMPT, o, &title)),
        ("core/handoff.md", fill(HANDOFF, o, &title)),
        ("README.md", fill(README, o, &title)),
        ("AGENTS.md", fill(AGENTS, o, &title)),
        (".gitattributes", "* text=auto eol=lf\n".to_string()),
    ];
    for (rel, content) in files {
        let path = root.join(rel);
        if path.exists() {
            continue;
        }
        if let Some(d) = path.parent() {
            fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        fs::write(&path, content).map_err(|e| format!("cannot write {rel}: {e}"))?;
        created.push(rel.to_string());
    }
    let ledger_path = root.join(LEDGER_PATH);
    if !ledger_path.exists() {
        let at = Date::today().to_string();
        let mut pref = Event::add("pref-1", "preference", &at, "Short answers, conclusion first.");
        pref.pin = Some(true);
        pref.source = Some("vmem init".into());
        let mut rule = Event::add(
            "rule-1",
            "rule",
            &at,
            "Work that needs a screen waits as a parked task.",
        );
        rule.pin = Some(true);
        rule.source = Some("vmem init".into());
        ledger::append(&ledger_path, &[pref, rule])?;
        created.push(LEDGER_PATH.to_string());
    }
    Ok(created)
}
