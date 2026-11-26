//! The commands. Each public function here is one `vmem` command and returns
//! the text to print, so the CLI (`main.rs`) and the MCP server (`mcp.rs`)
//! behave exactly the same. Every write goes through `commit_events`, which
//! validates the new events against the whole ledger before appending them,
//! then rebuilds the packs.

use crate::build;
use crate::config::{Config, CustomPack};
use crate::date::Date;
use crate::doctor;
use crate::handoff;
use crate::ingest::{self, EntityIndex};
use crate::init;
use crate::ledger::{self, Event, Ledger, LEDGER_PATH};
use crate::search::{self, Options};
use crate::state::{Kind, State, KINDS};
use crate::text::{self, est_tokens, slugify};
use std::path::{Path, PathBuf};

/// `Check`: the command ran and found problems (exit code 1).
/// `Error`: the command could not run (exit code 2).
pub enum Failure {
    Check(String),
    Error(String),
}

impl From<String> for Failure {
    fn from(s: String) -> Self {
        Failure::Error(s)
    }
}

pub type Out = Result<String, Failure>;

pub struct Repo {
    pub root: PathBuf,
    pub cfg: Config,
    pub ledger: Ledger,
    pub state: State,
}

impl Repo {
    pub fn open(root: &Path) -> Result<Repo, String> {
        let cfg = Config::load(root)?;
        let ledger = Ledger::load(&root.join(LEDGER_PATH))?;
        let state = State::fold(&ledger.events, None);
        Ok(Repo { root: root.to_path_buf(), cfg, ledger, state })
    }
}

fn date_or_today(s: Option<&str>) -> Result<Date, String> {
    match s {
        Some(v) => Date::parse(v).ok_or_else(|| format!("bad date {v:?}, use YYYY-MM-DD")),
        None => Ok(Date::today()),
    }
}

fn problems(repo: &Repo) -> Option<String> {
    if repo.ledger.errors.is_empty() && repo.state.issues.is_empty() {
        return None;
    }
    let mut s = String::from("The ledger has problems. Fix them first (`vmem doctor` lists them):\n");
    for (l, e) in &repo.ledger.errors {
        s.push_str(&format!("- line {l}: {e}\n"));
    }
    for i in &repo.state.issues {
        s.push_str(&format!("- line {}: {}\n", i.line, i.message));
    }
    Some(s.trim_end().to_string())
}

fn rebuild(root: &Path) -> Result<String, String> {
    let repo = Repo::open(root)?;
    let out = build::generate(&repo)?;
    let changed = build::write(root, &out)?;
    Ok(format!(
        "Rebuilt memory {} ({} files changed).",
        build::version(&repo).label(),
        changed.len()
    ))
}

/// Validates new events against the whole ledger, appends them and rebuilds.
pub fn commit_events(repo: &Repo, events: Vec<Event>, no_build: bool) -> Result<String, String> {
    if let Some(p) = problems(repo) {
        return Err(p);
    }
    let mut all = repo.ledger.events.clone();
    let base = all.iter().map(|(l, _)| *l).max().unwrap_or(0);
    for (i, e) in events.iter().enumerate() {
        all.push((base + i + 1, e.clone()));
    }
    let trial = State::fold(&all, None);
    if !trial.issues.is_empty() {
        let msgs: Vec<String> = trial.issues.iter().map(|i| i.message.clone()).collect();
        return Err(format!("Not saved: {}", msgs.join("; ")));
    }
    ledger::append(&repo.root.join(LEDGER_PATH), &events)?;
    if no_build {
        return Ok("Saved to the ledger. Not rebuilt: run `vmem build`.".into());
    }
    rebuild(&repo.root)
}

fn screen_or_fail(text: &str, force: bool) -> Result<(), String> {
    if force {
        return Ok(());
    }
    let hits = doctor::screen(text);
    if hits.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Not saved: the text contains {}. This repo is public. Remove it, or pass --force if it is a false alarm.",
            hits.join(", ")
        ))
    }
}

pub fn build_cmd(root: &Path, check: bool) -> Out {
    let repo = Repo::open(root)?;
    if let Some(p) = problems(&repo) {
        return Err(Failure::Error(p));
    }
    let out = build::generate(&repo)?;
    let v = build::version(&repo);
    if check {
        let drift = build::drift(root, &out);
        if drift.is_empty() {
            return Ok(format!("Up to date: memory {}.", v.label()));
        }
        let list: Vec<String> = drift.iter().map(|d| format!("- {d}")).collect();
        return Err(Failure::Check(format!(
            "Generated files are out of date. Run `vmem build` and commit the result.\n{}",
            list.join("\n")
        )));
    }
    let changed = build::write(root, &out)?;
    let mut s = format!(
        "Built memory {}: {} files generated, {} changed.",
        v.label(),
        out.files.len(),
        changed.len()
    );
    for c in changed.iter().take(40) {
        s.push_str(&format!("\n- {c}"));
    }
    Ok(s)
}

#[derive(Debug, Default, Clone)]
pub struct Remember {
    pub kind: String,
    pub text: String,
    pub title: Option<String>,
    pub topics: Vec<String>,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub say: Option<String>,
    pub why: Option<String>,
    pub state: Option<String>,
    pub pin: bool,
    pub parked: bool,
    pub confidence: Option<String>,
    pub source: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub supersedes: Option<String>,
    pub at: Option<String>,
    pub force: bool,
}

fn check_confidence(c: &Option<String>) -> Result<(), String> {
    match c.as_deref() {
        None | Some("high") | Some("medium") | Some("low") => Ok(()),
        Some(other) => Err(format!("confidence must be high, medium or low, not {other:?}")),
    }
}

fn check_dates(ds: &[&Option<String>]) -> Result<(), String> {
    for d in ds.iter().copied().flatten() {
        Date::parse(d).ok_or_else(|| format!("bad date {d:?}, use YYYY-MM-DD"))?;
    }
    Ok(())
}

pub fn remember(root: &Path, a: Remember, no_build: bool) -> Out {
    let repo = Repo::open(root)?;
    let kinds: Vec<&str> = KINDS.iter().map(|k| k.name()).collect();
    let kind = Kind::parse(&a.kind)
        .ok_or_else(|| format!("unknown kind {:?}; use one of: {}", a.kind, kinds.join(", ")))?;
    let text = a.text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Err(Failure::Error("nothing to remember: the text is empty".into()));
    }
    if a.parked && kind != Kind::Task {
        return Err(Failure::Error("--parked only applies to tasks".into()));
    }
    check_confidence(&a.confidence)?;
    check_dates(&[&a.valid_from, &a.valid_to, &a.at])?;
    let at = date_or_today(a.at.as_deref())?;
    let title = a.title.clone().or_else(|| kind.is_entity().then(|| text.clone()));
    screen_or_fail(
        &format!(
            "{} {} {} {} {}",
            title.as_deref().unwrap_or(""),
            text,
            a.why.as_deref().unwrap_or(""),
            a.state.as_deref().unwrap_or(""),
            a.aliases.join(" ")
        ),
        a.force,
    )?;
    let id = repo.state.next_id(kind);
    let mut e = Event::add(&id, kind.name(), &at.to_string(), &text);
    e.title = title;
    e.topics = a.topics;
    e.tags = a.tags;
    e.aliases = a.aliases;
    e.say = a.say;
    e.why = a.why;
    e.state = a.state;
    e.pin = a.pin.then_some(true);
    e.parked = a.parked.then_some(true);
    e.confidence = a.confidence;
    e.source = Some(a.source.unwrap_or_else(|| "desktop".into()));
    e.valid_from = a.valid_from;
    e.valid_to = a.valid_to;
    e.supersedes = a.supersedes;
    let report = commit_events(&repo, vec![e], no_build)?;
    Ok(format!("Remembered {id} ({}): {text}\n{report}", kind.name()))
}

/// `done` (task finished, question answered) or `retire` (no longer true).
pub fn close(root: &Path, op: &str, id: &str, note: Option<String>, at: Option<&str>, no_build: bool) -> Out {
    let repo = Repo::open(root)?;
    let r = repo.state.get(id).ok_or_else(|| format!("unknown id {id} (try `vmem search`)"))?;
    let label = text::clip(&r.text, 80);
    let at = date_or_today(at)?;
    let mut e = Event::new(op, id, &at.to_string());
    e.note = note;
    let report = commit_events(&repo, vec![e], no_build)?;
    let verb = if op == "done" { "Done" } else { "Retired" };
    Ok(format!("{verb} {id}: {label}\n{report}"))
}

#[derive(Debug, Default, Clone)]
pub struct SetFields {
    pub text: Option<String>,
    pub title: Option<String>,
    pub state: Option<String>,
    pub why: Option<String>,
    pub say: Option<String>,
    pub confidence: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub aliases: Vec<String>,
    pub topics: Vec<String>,
    pub tags: Vec<String>,
    pub pin: Option<bool>,
    pub parked: Option<bool>,
    pub verified: bool,
    pub note: Option<String>,
    pub at: Option<String>,
    pub force: bool,
}

pub fn set(root: &Path, id: &str, f: SetFields, no_build: bool) -> Out {
    let repo = Repo::open(root)?;
    repo.state.get(id).ok_or_else(|| format!("unknown id {id} (try `vmem search`)"))?;
    check_confidence(&f.confidence)?;
    check_dates(&[&f.valid_from, &f.valid_to, &f.at])?;
    let nothing = f.text.is_none()
        && f.title.is_none()
        && f.state.is_none()
        && f.why.is_none()
        && f.say.is_none()
        && f.confidence.is_none()
        && f.valid_from.is_none()
        && f.valid_to.is_none()
        && f.aliases.is_empty()
        && f.topics.is_empty()
        && f.tags.is_empty()
        && f.pin.is_none()
        && f.parked.is_none()
        && !f.verified;
    if nothing {
        return Err(Failure::Error("nothing to change: pass at least one field (see `vmem set --help`)".into()));
    }
    screen_or_fail(
        &format!(
            "{} {} {} {} {}",
            f.text.as_deref().unwrap_or(""),
            f.title.as_deref().unwrap_or(""),
            f.state.as_deref().unwrap_or(""),
            f.why.as_deref().unwrap_or(""),
            f.aliases.join(" ")
        ),
        f.force,
    )?;
    let at = date_or_today(f.at.as_deref())?;
    let mut e = Event::new("set", id, &at.to_string());
    e.text = f.text;
    e.title = f.title;
    e.state = f.state;
    e.why = f.why;
    e.say = f.say;
    e.confidence = f.confidence;
    e.valid_from = f.valid_from;
    e.valid_to = f.valid_to;
    e.aliases = f.aliases;
    e.topics = f.topics;
    e.tags = f.tags;
    e.pin = f.pin;
    e.parked = f.parked;
    e.verified = f.verified.then_some(true);
    e.note = f.note;
    let report = commit_events(&repo, vec![e], no_build)?;
    Ok(format!("Updated {id}.\n{report}"))
}

#[allow(clippy::too_many_arguments)]
pub fn ingest_cmd(
    root: &Path,
    input: &str,
    source: &str,
    dry_run: bool,
    at: Option<&str>,
    force: bool,
    no_build: bool,
) -> Out {
    let repo = Repo::open(root)?;
    if let Some(p) = problems(&repo) {
        return Err(Failure::Error(p));
    }
    let h = handoff::parse(input);
    if h.is_empty() {
        return Err(Failure::Error("no handoff found in the input".into()));
    }
    // A source with a number ("issue #12") identifies one handoff: never save it twice.
    if source.contains('#') {
        if let Some(s) = repo.state.sessions.iter().find(|s| s.source.as_deref() == Some(source)) {
            return Err(Failure::Error(format!("this handoff ({source}) was already saved as session {}", s.id)));
        }
    }
    screen_or_fail(input, force)?;
    let today = Date::today();
    let mut notes = Vec::new();
    let date = match (at, h.date) {
        (Some(explicit), _) => date_or_today(Some(explicit))?,
        (None, Some(d)) if d > today.add_days(1) => {
            notes.push(format!("the handoff was dated {d}, in the future; used {today} instead"));
            today
        }
        (None, Some(d)) => d,
        (None, None) => today,
    };
    let plan = ingest::plan(&repo.state, &h, date, source);
    let mut s = format!("Handoff for {date}: {} changes.\n", plan.events.len());
    for n in notes {
        s.push_str(&format!("- note: {n}\n"));
    }
    for line in &plan.report {
        s.push_str(&format!("- {line}\n"));
    }
    if dry_run {
        s.push_str("Dry run: nothing was written.");
        return Ok(s);
    }
    s.push_str(&commit_events(&repo, plan.events, no_build)?);
    Ok(s)
}

pub fn resume(root: &Path, spoken: bool) -> Out {
    let repo = Repo::open(root)?;
    Ok(if spoken { build::spoken_for(&repo)? } else { build::resume_for(&repo)? })
}

pub fn search_cmd(
    root: &Path,
    query: &str,
    limit: usize,
    kind: Option<&str>,
    all: bool,
    as_of: Option<&str>,
    json: bool,
) -> Out {
    let repo = Repo::open(root)?;
    let kind = match kind {
        Some(k) => Some(Kind::parse(k).ok_or_else(|| format!("unknown kind {k:?}"))?),
        None => None,
    };
    let st = match as_of {
        Some(d) => State::fold(&repo.ledger.events, Some(Date::parse(d).ok_or_else(|| format!("bad date {d:?}"))?)),
        None => repo.state,
    };
    let hits = search::search(&st, query, &Options { limit, kind, all });
    Ok(if json { search::hits_json(&hits) } else { search::format_hits(&hits) })
}

pub fn context_cmd(root: &Path, goal: &str, budget: usize, save: Option<&str>, no_build: bool) -> Out {
    let mut repo = Repo::open(root)?;
    if goal.trim().is_empty() {
        return Err(Failure::Error("describe the task, for example: vmem context \"meeting with the bank\"".into()));
    }
    let Some(slug) = save else {
        return Ok(build::context_for(&repo, goal, budget)?);
    };
    let slug = slugify(slug);
    if slug.is_empty() {
        return Err(Failure::Error("the pack name needs letters or digits".into()));
    }
    screen_or_fail(goal, false)?;
    repo.cfg.packs.retain(|p| p.slug != slug);
    repo.cfg.packs.push(CustomPack { slug: slug.clone(), goal: goal.trim().to_string(), budget });
    repo.cfg.packs.sort_by(|a, b| a.slug.cmp(&b.slug));
    repo.cfg.save(root)?;
    let report = if no_build { "Saved. Not rebuilt: run `vmem build`.".to_string() } else { rebuild(root)? };
    Ok(format!(
        "Saved task pack \"{slug}\". Once pushed, the voice agent can read it at:\n{}\n{report}",
        repo.cfg.raw_url(&format!("packs/custom/{slug}.md"))
    ))
}

pub fn drop_pack(root: &Path, slug: &str, no_build: bool) -> Out {
    let mut repo = Repo::open(root)?;
    let slug = slugify(slug);
    let before = repo.cfg.packs.len();
    repo.cfg.packs.retain(|p| p.slug != slug);
    if repo.cfg.packs.len() == before {
        return Err(Failure::Error(format!("no saved task pack called {slug:?}")));
    }
    repo.cfg.save(root)?;
    let report = if no_build { "Not rebuilt: run `vmem build`.".to_string() } else { rebuild(root)? };
    Ok(format!("Removed task pack \"{slug}\".\n{report}"))
}

pub fn topic_cmd(root: &Path, name: &str) -> Out {
    let repo = Repo::open(root)?;
    Ok(build::topic_for(&repo, name)?)
}

pub fn history_cmd(root: &Path, id: &str) -> Out {
    let repo = Repo::open(root)?;
    let st = &repo.state;
    let r = st.get(id).ok_or_else(|| format!("unknown id {id}"))?;
    let mut s = format!("{} · {} · {}\n{}\n", r.id, r.kind.name(), r.status.name(), r.text);
    if let Some(t) = &r.title {
        s.push_str(&format!("Title: {t}\n"));
    }
    if let Some(w) = &r.why {
        s.push_str(&format!("Why: {w}\n"));
    }
    if let Some(x) = &r.state {
        s.push_str(&format!("Stage: {x}\n"));
    }
    if !r.aliases.is_empty() {
        s.push_str(&format!("Also heard as: {}\n", r.aliases.join(", ")));
    }
    if !r.topics.is_empty() {
        s.push_str(&format!("Topics: {}\n", r.topics.join(", ")));
    }
    if let Some(src) = &r.source {
        s.push_str(&format!("Source: {src}\n"));
    }
    s.push_str(&format!("Ledger line: {}\n", r.line));
    s.push_str("\nHistory:\n");
    for h in &r.history {
        s.push_str(&format!("- {h}\n"));
    }
    if let Some(old) = r.supersedes.as_deref().and_then(|o| st.get(o)) {
        s.push_str(&format!("\nReplaces {}: {}\n", old.id, old.text));
    }
    if let Some(new) = r.superseded_by.as_deref().and_then(|n| st.get(n)) {
        s.push_str(&format!("\nReplaced by {}: {}\n", new.id, new.text));
    }
    let sessions: Vec<String> = st
        .sessions
        .iter()
        .filter(|x| x.items.iter().any(|i| i == id))
        .map(|x| format!("- {} ({}, ledger line {}): {}", x.at, x.id, x.line, x.summary))
        .collect();
    if !sessions.is_empty() {
        s.push_str(&format!("\nSessions:\n{}\n", sessions.join("\n")));
    }
    Ok(s.trim_end().to_string())
}

pub fn resolve_cmd(root: &Path, text: &str) -> Out {
    let repo = Repo::open(root)?;
    let idx = EntityIndex::from_state(&repo.state);
    let found = idx.resolve(text);
    if found.is_empty() {
        return Ok("No known names found.".into());
    }
    let lines: Vec<String> = found
        .iter()
        .map(|m| {
            let how = if m.exact { "exact".to_string() } else { format!("sounds alike, {:.0}%", m.score * 100.0) };
            format!("- \"{}\" → {} ({}, {}) · {how}", m.heard, m.name, m.kind.name(), m.id)
        })
        .collect();
    Ok(lines.join("\n"))
}

pub fn doctor_cmd(root: &Path, strict: bool) -> Out {
    let repo = Repo::open(root)?;
    let report = doctor::run(&repo);
    let text = report.render();
    if report.errors() > 0 || (strict && report.warnings() > 0) {
        Err(Failure::Check(text))
    } else {
        Ok(text)
    }
}

pub fn stats_cmd(root: &Path) -> Out {
    let repo = Repo::open(root)?;
    let out = build::generate(&repo)?;
    let v = build::version(&repo);
    let st = &repo.state;
    let mut s = format!(
        "Memory {}\nEvents: {} · sessions: {} · current items: {}\n\nBy kind (current / all):\n",
        v.label(),
        st.events,
        st.sessions.len(),
        st.current().len()
    );
    for k in KINDS {
        let all = st.records.iter().filter(|r| r.kind == k).count();
        if all > 0 {
            s.push_str(&format!("- {}: {} / {all}\n", k.name(), st.current_of(k).len()));
        }
    }
    s.push_str("\nPacks (estimated tokens):\n");
    for p in ["packs/boot.md", "packs/brief.md", "packs/full.md", "packs/spoken.txt", "packs/check.md", "llms.txt"] {
        if let Some(c) = out.files.get(p) {
            s.push_str(&format!("- {p}: {}\n", est_tokens(c)));
        }
    }
    let topics = out.files.keys().filter(|k| k.starts_with("packs/topics/")).count();
    s.push_str(&format!("- topic pages: {topics}\n- budgets: boot {}, brief {}", repo.cfg.budgets.boot, repo.cfg.budgets.brief));
    Ok(s)
}

pub fn init_cmd(root: &Path, opts: &init::Options) -> Out {
    let created = init::scaffold(root, opts)?;
    let report = rebuild(root)?;
    let mut s = format!("Created a voice memory repo for {} in {}.\n", opts.owner, root.display());
    for c in &created {
        s.push_str(&format!("- {c}\n"));
    }
    s.push_str(&report);
    s.push_str(&format!(
        "\nNext: push it to GitHub as a public repo named {}, then tell {}: \"read github.com/{} and follow the instructions\".",
        opts.repo, opts.agent, opts.repo
    ));
    Ok(s)
}
