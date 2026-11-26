//! Folds the ledger into the current memory: every item with its status,
//! dates and links, plus the list of sessions.
//!
//! Two clocks matter:
//! - recorded time: the `at` of each event. `fold(.., Some(date))` replays the
//!   ledger only up to that date, to show what memory looked like back then.
//! - valid time: `valid_from` / `valid_to` on an item ("Mateo is in Panama
//!   from January 2 to January 10"). An item outside its window is not current.
//!
//! The memory clock is the date of the newest event. Packs are built against
//! it, never against the wall clock, so builds are reproducible.

use crate::date::Date;
use crate::ledger::Event;
use crate::text;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Profile,
    Preference,
    Rule,
    Goal,
    Project,
    Person,
    Decision,
    Task,
    Question,
    Fact,
    Term,
}

pub const KINDS: [Kind; 11] = [
    Kind::Profile,
    Kind::Preference,
    Kind::Rule,
    Kind::Goal,
    Kind::Project,
    Kind::Person,
    Kind::Decision,
    Kind::Task,
    Kind::Question,
    Kind::Fact,
    Kind::Term,
];

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match text::fold(s.trim()).as_str() {
            "profile" | "me" => Kind::Profile,
            "preference" | "preferences" | "pref" => Kind::Preference,
            "rule" | "rules" => Kind::Rule,
            "goal" | "goals" => Kind::Goal,
            "project" | "projects" | "proj" => Kind::Project,
            "person" | "people" => Kind::Person,
            "decision" | "decisions" | "dec" => Kind::Decision,
            "task" | "tasks" | "todo" => Kind::Task,
            "question" | "questions" | "q" => Kind::Question,
            "fact" | "facts" | "note" => Kind::Fact,
            "term" | "terms" | "glossary" | "name" => Kind::Term,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Profile => "profile",
            Kind::Preference => "preference",
            Kind::Rule => "rule",
            Kind::Goal => "goal",
            Kind::Project => "project",
            Kind::Person => "person",
            Kind::Decision => "decision",
            Kind::Task => "task",
            Kind::Question => "question",
            Kind::Fact => "fact",
            Kind::Term => "term",
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Kind::Profile => "prof",
            Kind::Preference => "pref",
            Kind::Rule => "rule",
            Kind::Goal => "goal",
            Kind::Project => "proj",
            Kind::Person => "pers",
            Kind::Decision => "dec",
            Kind::Task => "task",
            Kind::Question => "q",
            Kind::Fact => "fact",
            Kind::Term => "term",
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            Kind::Profile => "Profile",
            Kind::Preference => "Preferences",
            Kind::Rule => "Rules",
            Kind::Goal => "Goals",
            Kind::Project => "Projects",
            Kind::Person => "People",
            Kind::Decision => "Decisions",
            Kind::Task => "Tasks",
            Kind::Question => "Open questions",
            Kind::Fact => "Facts",
            Kind::Term => "Glossary",
        }
    }

    /// Projects, people and terms are named things with aliases.
    pub fn is_entity(self) -> bool {
        matches!(self, Kind::Project | Kind::Person | Kind::Term)
    }

    /// Projects and people get their own topic page.
    pub fn is_anchor(self) -> bool {
        matches!(self, Kind::Project | Kind::Person)
    }

    /// Kinds that can be closed with `done`.
    pub fn can_finish(self) -> bool {
        matches!(self, Kind::Task | Kind::Question | Kind::Goal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    Done,
    Retired,
    Superseded,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Done => "done",
            Status::Retired => "retired",
            Status::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Record {
    pub id: String,
    pub kind: Kind,
    pub text: String,
    pub title: Option<String>,
    pub state: Option<String>,
    pub why: Option<String>,
    pub say: Option<String>,
    pub aliases: Vec<String>,
    pub topics: Vec<String>,
    pub tags: Vec<String>,
    pub pin: bool,
    pub parked: bool,
    pub confidence: Option<String>,
    pub source: Option<String>,
    pub valid_from: Option<Date>,
    pub valid_to: Option<Date>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub status: Status,
    pub created: Date,
    pub updated: Date,
    /// Last time a session mentioned this item.
    pub touched: Date,
    pub closed: Option<Date>,
    pub verified: Option<Date>,
    pub note: Option<String>,
    pub line: usize,
    pub history: Vec<String>,
}

impl Record {
    /// The display name: the title for named things, otherwise the text.
    pub fn name(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.text)
    }

    /// Everything searchable about the item.
    pub fn search_text(&self) -> String {
        let mut s = String::new();
        for part in [self.title.as_deref(), Some(self.text.as_str()), self.state.as_deref(), self.why.as_deref()]
            .into_iter()
            .flatten()
        {
            s.push_str(part);
            s.push(' ');
        }
        for list in [&self.aliases, &self.topics, &self.tags] {
            for a in list {
                s.push_str(a);
                s.push(' ');
            }
        }
        s
    }

    /// Most recent sign of life: an edit or a session that mentioned it.
    pub fn last_seen(&self) -> Date {
        self.updated.max(self.touched)
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub at: Date,
    pub summary: String,
    pub done: Vec<String>,
    pub items: Vec<String>,
    pub topics: Vec<String>,
    pub heard: Vec<String>,
    pub source: Option<String>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct State {
    pub records: Vec<Record>,
    pub sessions: Vec<Session>,
    pub issues: Vec<Issue>,
    pub memory_date: Option<Date>,
    pub events: usize,
    index: HashMap<String, usize>,
}

impl State {
    /// Replays the ledger. With `as_of`, events after that date are ignored.
    pub fn fold(events: &[(usize, Event)], as_of: Option<Date>) -> State {
        let mut st = State::default();
        for (line, e) in events {
            let Some(at) = Date::parse(&e.at) else {
                st.issue(*line, format!("{} {}: bad date {:?}, use YYYY-MM-DD", e.op, e.id, e.at));
                continue;
            };
            if as_of.is_some_and(|limit| at > limit) {
                continue;
            }
            st.events += 1;
            st.memory_date = Some(st.memory_date.map_or(at, |m| m.max(at)));
            match e.op.as_str() {
                "add" => st.apply_add(*line, e, at),
                "set" => st.apply_set(*line, e, at),
                "done" => st.apply_close(*line, e, at, Status::Done),
                "retire" => st.apply_close(*line, e, at, Status::Retired),
                "session" => st.apply_session(*line, e, at),
                other => st.issue(*line, format!("unknown op {other:?} (use add, set, done, retire or session)")),
            }
        }
        st
    }

    fn issue(&mut self, line: usize, message: String) {
        self.issues.push(Issue { line, message });
    }

    fn opt_date(&mut self, line: usize, id: &str, field: &str, v: &Option<String>) -> Option<Date> {
        let s = v.as_ref()?;
        let d = Date::parse(s);
        if d.is_none() {
            self.issue(line, format!("{id}: bad {field} {s:?}, use YYYY-MM-DD"));
        }
        d
    }

    fn apply_add(&mut self, line: usize, e: &Event, at: Date) {
        let Some(kind) = e.kind.as_deref().and_then(Kind::parse) else {
            return self.issue(line, format!("add {}: missing or unknown kind {:?}", e.id, e.kind));
        };
        if e.id.trim().is_empty() {
            return self.issue(line, "add: missing id".into());
        }
        if self.index.contains_key(&e.id) {
            return self.issue(line, format!("add {}: id already used", e.id));
        }
        let text = e.text.clone().unwrap_or_default();
        if text.trim().is_empty() {
            return self.issue(line, format!("add {}: empty text", e.id));
        }
        let valid_from = self.opt_date(line, &e.id, "valid_from", &e.valid_from);
        let valid_to = self.opt_date(line, &e.id, "valid_to", &e.valid_to);
        if let Some(old) = &e.supersedes {
            match self.index.get(old).copied() {
                Some(i) => {
                    let o = &mut self.records[i];
                    o.status = Status::Superseded;
                    o.superseded_by = Some(e.id.clone());
                    o.updated = at;
                    o.history.push(format!("{at} superseded by {}", e.id));
                }
                None => self.issue(line, format!("add {}: supersedes unknown id {old}", e.id)),
            }
        }
        let mut history = vec![format!(
            "{at} added{}",
            e.source.as_ref().map(|s| format!(" (source: {s})")).unwrap_or_default()
        )];
        if let Some(old) = &e.supersedes {
            history.push(format!("{at} replaces {old}"));
        }
        let rec = Record {
            id: e.id.clone(),
            kind,
            text,
            title: e.title.clone(),
            state: e.state.clone(),
            why: e.why.clone(),
            say: e.say.clone(),
            aliases: e.aliases.clone(),
            topics: e.topics.iter().map(|t| text::slugify(t)).filter(|t| !t.is_empty()).collect(),
            tags: e.tags.clone(),
            pin: e.pin.unwrap_or(false),
            parked: e.parked.unwrap_or(false),
            confidence: e.confidence.clone(),
            source: e.source.clone(),
            valid_from,
            valid_to,
            supersedes: e.supersedes.clone(),
            superseded_by: None,
            status: Status::Active,
            created: at,
            updated: at,
            touched: at,
            closed: None,
            verified: if e.verified == Some(true) { Some(at) } else { None },
            note: e.note.clone(),
            line,
            history,
        };
        self.index.insert(e.id.clone(), self.records.len());
        self.records.push(rec);
    }

    fn apply_set(&mut self, line: usize, e: &Event, at: Date) {
        let Some(i) = self.index.get(&e.id).copied() else {
            return self.issue(line, format!("set {}: unknown id", e.id));
        };
        if e.kind.is_some() {
            self.issue(line, format!("set {}: kind cannot change (retire it and add a new item)", e.id));
        }
        let valid_from = self.opt_date(line, &e.id, "valid_from", &e.valid_from);
        let valid_to = self.opt_date(line, &e.id, "valid_to", &e.valid_to);
        let r = &mut self.records[i];
        let mut changes: Vec<&str> = Vec::new();
        if let Some(t) = &e.text {
            r.text = t.clone();
            changes.push("text");
        }
        if let Some(t) = &e.title {
            r.title = Some(t.clone());
            changes.push("title");
        }
        if let Some(t) = &e.state {
            r.state = Some(t.clone());
            changes.push("state");
        }
        if let Some(t) = &e.why {
            r.why = Some(t.clone());
            changes.push("why");
        }
        if let Some(t) = &e.say {
            r.say = Some(t.clone());
            changes.push("say");
        }
        if let Some(c) = &e.confidence {
            r.confidence = Some(c.clone());
            changes.push("confidence");
        }
        if let Some(p) = e.pin {
            r.pin = p;
            changes.push(if p { "pinned" } else { "unpinned" });
        }
        if let Some(p) = e.parked {
            r.parked = p;
            changes.push(if p { "parked" } else { "unparked" });
        }
        if valid_from.is_some() {
            r.valid_from = valid_from;
            changes.push("valid_from");
        }
        if valid_to.is_some() {
            r.valid_to = valid_to;
            changes.push("valid_to");
        }
        let mut merged = false;
        for a in &e.aliases {
            if !r.aliases.iter().any(|x| text::fold(x) == text::fold(a)) {
                r.aliases.push(a.clone());
                merged = true;
            }
        }
        if merged {
            changes.push("aliases");
        }
        let before = r.topics.len();
        for t in &e.topics {
            let slug = text::slugify(t);
            if !slug.is_empty() && !r.topics.contains(&slug) {
                r.topics.push(slug);
            }
        }
        if r.topics.len() != before {
            changes.push("topics");
        }
        let before = r.tags.len();
        for t in &e.tags {
            if !r.tags.contains(t) {
                r.tags.push(t.clone());
            }
        }
        if r.tags.len() != before {
            changes.push("tags");
        }
        if e.verified == Some(true) {
            r.verified = Some(at);
            changes.push("verified");
        }
        r.updated = at;
        let note = e.note.as_ref().map(|n| format!(" ({n})")).unwrap_or_default();
        if changes.is_empty() {
            r.history.push(format!("{at} touched{note}"));
        } else {
            r.history.push(format!("{at} changed {}{note}", changes.join(", ")));
        }
    }

    fn apply_close(&mut self, line: usize, e: &Event, at: Date, status: Status) {
        let Some(i) = self.index.get(&e.id).copied() else {
            return self.issue(line, format!("{} {}: unknown id", e.op, e.id));
        };
        let (current, kind) = (self.records[i].status, self.records[i].kind);
        if current != Status::Active {
            return self.issue(line, format!("{} {}: item is already {}", e.op, e.id, current.name()));
        }
        if status == Status::Done && !kind.can_finish() {
            return self.issue(line, format!("done {}: a {} cannot be done, retire it instead", e.id, kind.name()));
        }
        let r = &mut self.records[i];
        r.status = status;
        r.closed = Some(at);
        r.updated = at;
        r.note = e.note.clone();
        let note = e.note.as_ref().map(|n| format!(": {n}")).unwrap_or_default();
        r.history.push(format!("{at} {}{note}", status.name()));
    }

    fn apply_session(&mut self, line: usize, e: &Event, at: Date) {
        if self.sessions.iter().any(|s| s.id == e.id) {
            return self.issue(line, format!("session {}: id already used", e.id));
        }
        let summary = e.summary.clone().unwrap_or_default();
        if summary.trim().is_empty() {
            return self.issue(line, format!("session {}: empty summary", e.id));
        }
        for id in &e.items {
            match self.index.get(id).copied() {
                Some(i) => {
                    let r = &mut self.records[i];
                    if at > r.touched {
                        r.touched = at;
                    }
                }
                None => self.issue(line, format!("session {}: unknown item {id}", e.id)),
            }
        }
        self.sessions.push(Session {
            id: e.id.clone(),
            at,
            summary,
            done: e.done.clone(),
            items: e.items.clone(),
            topics: e.topics.iter().map(|t| text::slugify(t)).filter(|t| !t.is_empty()).collect(),
            heard: e.heard.clone(),
            source: e.source.clone(),
            line,
        });
    }

    pub fn get(&self, id: &str) -> Option<&Record> {
        self.index.get(id).map(|&i| &self.records[i])
    }

    /// The memory clock: date of the newest event.
    pub fn clock(&self) -> Date {
        self.memory_date.unwrap_or(Date { y: 2000, m: 1, d: 1 })
    }

    /// Active and inside its validity window on the memory date.
    pub fn is_current(&self, r: &Record) -> bool {
        let now = self.clock();
        r.status == Status::Active
            && r.valid_from.map_or(true, |f| f <= now)
            && r.valid_to.map_or(true, |t| t >= now)
    }

    /// Active but only valid from a later date.
    pub fn is_upcoming(&self, r: &Record) -> bool {
        r.status == Status::Active && r.valid_from.is_some_and(|f| f > self.clock())
    }

    pub fn current(&self) -> Vec<&Record> {
        self.records.iter().filter(|r| self.is_current(r)).collect()
    }

    /// Current items of one kind: pinned first, then newest first.
    pub fn current_of(&self, kind: Kind) -> Vec<&Record> {
        let mut v: Vec<&Record> = self.records.iter().filter(|r| r.kind == kind && self.is_current(r)).collect();
        sort_pinned_newest(&mut v);
        v
    }

    pub fn last_session(&self) -> Option<&Session> {
        self.sessions.last()
    }

    /// Open tasks in the order to resume them: what the last session left
    /// next, then pinned tasks, then the rest, newest first.
    pub fn open_tasks(&self, parked: bool) -> Vec<&Record> {
        let mut v: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| r.kind == Kind::Task && r.parked == parked && self.is_current(r))
            .collect();
        let last: Vec<&str> = self
            .last_session()
            .map(|s| s.items.iter().map(String::as_str).collect())
            .unwrap_or_default();
        v.sort_by(|a, b| {
            let pa = last.iter().position(|x| *x == a.id).unwrap_or(usize::MAX);
            let pb = last.iter().position(|x| *x == b.id).unwrap_or(usize::MAX);
            pa.cmp(&pb)
                .then(b.pin.cmp(&a.pin))
                .then(b.created.cmp(&a.created))
                .then(text::id_number(&b.id).cmp(&text::id_number(&a.id)))
        });
        v
    }

    /// Items closed in the last `days` days of the memory clock.
    pub fn recently_closed(&self, kind: Kind, days: i64) -> Vec<&Record> {
        let since = self.clock().add_days(-days);
        let mut v: Vec<&Record> = self
            .records
            .iter()
            .filter(|r| r.kind == kind && r.status == Status::Done && r.closed.is_some_and(|c| c >= since))
            .collect();
        v.sort_by(|a, b| b.closed.cmp(&a.closed).then(text::id_number(&b.id).cmp(&text::id_number(&a.id))));
        v
    }

    /// The next free id for a kind, such as "task-4".
    pub fn next_id(&self, kind: Kind) -> String {
        Ids::new(self).take(kind)
    }
}

/// Pinned first, then newest first, then by id (stable and deterministic).
pub fn sort_pinned_newest(v: &mut [&Record]) {
    v.sort_by(|a, b| {
        b.pin
            .cmp(&a.pin)
            .then(b.created.cmp(&a.created))
            .then(text::id_number(&b.id).cmp(&text::id_number(&a.id)))
    });
}

/// Hands out new ids for several events planned at once.
pub struct Ids {
    next: HashMap<String, u64>,
}

impl Ids {
    pub fn new(st: &State) -> Ids {
        let mut next: HashMap<String, u64> = HashMap::new();
        for r in &st.records {
            let prefix = r.id.rsplit_once('-').map(|(p, _)| p.to_string()).unwrap_or_default();
            let n = text::id_number(&r.id);
            let e = next.entry(prefix).or_insert(1);
            if n + 1 > *e {
                *e = n + 1;
            }
        }
        Ids { next }
    }

    pub fn take(&mut self, kind: Kind) -> String {
        let e = self.next.entry(kind.prefix().to_string()).or_insert(1);
        let id = format!("{}-{}", kind.prefix(), e);
        *e += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Ledger;

    fn st(lines: &str) -> State {
        let l = Ledger::parse(lines.to_string());
        assert!(l.errors.is_empty(), "{:?}", l.errors);
        State::fold(&l.events, None)
    }

    #[test]
    fn folds_lifecycle() {
        let s = st(concat!(
            r#"{"op":"add","id":"task-1","at":"2026-11-20","kind":"task","text":"Write the README"}"#, "\n",
            r#"{"op":"add","id":"pref-1","at":"2026-11-20","kind":"preference","text":"Short answers","pin":true}"#, "\n",
            r#"{"op":"set","id":"task-1","at":"2026-11-21","parked":true}"#, "\n",
            r#"{"op":"done","id":"task-1","at":"2026-11-22","note":"shipped"}"#, "\n",
            r#"{"op":"add","id":"pref-2","at":"2026-11-23","kind":"preference","text":"Very short answers","supersedes":"pref-1"}"#, "\n",
        ));
        assert!(s.issues.is_empty(), "{:?}", s.issues);
        assert_eq!(s.get("task-1").unwrap().status, Status::Done);
        assert!(s.get("task-1").unwrap().parked);
        assert_eq!(s.get("pref-1").unwrap().status, Status::Superseded);
        assert_eq!(s.get("pref-1").unwrap().superseded_by.as_deref(), Some("pref-2"));
        assert_eq!(s.current_of(Kind::Preference).len(), 1);
        assert_eq!(s.clock().to_string(), "2026-11-23");
        assert_eq!(s.next_id(Kind::Preference), "pref-3");
        assert_eq!(s.next_id(Kind::Fact), "fact-1");
    }

    #[test]
    fn replays_as_of_a_date() {
        let l = Ledger::parse(concat!(
            r#"{"op":"add","id":"fact-1","at":"2026-11-01","kind":"fact","text":"Old"}"#, "\n",
            r#"{"op":"retire","id":"fact-1","at":"2026-11-10","note":"wrong"}"#, "\n",
        ).to_string());
        let then = State::fold(&l.events, Date::parse("2026-11-05"));
        assert_eq!(then.get("fact-1").unwrap().status, Status::Active);
        let now = State::fold(&l.events, None);
        assert_eq!(now.get("fact-1").unwrap().status, Status::Retired);
    }

    #[test]
    fn validity_windows() {
        let s = st(concat!(
            r#"{"op":"add","id":"fact-1","at":"2026-11-25","kind":"fact","text":"Trip","valid_from":"2027-01-02","valid_to":"2027-01-10"}"#, "\n",
            r#"{"op":"add","id":"fact-2","at":"2026-11-25","kind":"fact","text":"Expired","valid_to":"2026-11-01"}"#, "\n",
        ));
        assert!(s.is_upcoming(s.get("fact-1").unwrap()));
        assert!(!s.is_current(s.get("fact-1").unwrap()));
        assert!(!s.is_current(s.get("fact-2").unwrap()));
    }

    #[test]
    fn reports_problems_instead_of_failing() {
        let s = st(concat!(
            r#"{"op":"set","id":"nope","at":"2026-11-25","pin":true}"#, "\n",
            r#"{"op":"add","id":"x-1","at":"2026-11-25","kind":"wizard","text":"?"}"#, "\n",
            r#"{"op":"add","id":"fact-1","at":"bad","kind":"fact","text":"?"}"#, "\n",
            r#"{"op":"add","id":"fact-2","at":"2026-11-25","kind":"fact","text":"ok"}"#, "\n",
            r#"{"op":"done","id":"fact-2","at":"2026-11-25"}"#, "\n",
        ));
        assert_eq!(s.issues.len(), 4, "{:?}", s.issues);
        assert_eq!(s.issues[0].line, 1);
    }

    #[test]
    fn sessions_touch_items_and_order_next_steps() {
        let s = st(concat!(
            r#"{"op":"add","id":"task-1","at":"2026-11-20","kind":"task","text":"Old task"}"#, "\n",
            r#"{"op":"add","id":"task-2","at":"2026-11-21","kind":"task","text":"Newer task"}"#, "\n",
            r#"{"op":"session","id":"s-2026-11-24-1","at":"2026-11-24","summary":"Talked","items":["task-1"]}"#, "\n",
        ));
        let open: Vec<&str> = s.open_tasks(false).iter().map(|r| r.id.as_str()).collect();
        assert_eq!(open, vec!["task-1", "task-2"]);
        assert_eq!(s.get("task-1").unwrap().last_seen().to_string(), "2026-11-24");
    }
}
