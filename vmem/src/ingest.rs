//! Turns a parsed handoff into ledger events.
//!
//! - DONE lines close the open task they match best.
//! - NEXT and PARKED lines become tasks, unless the same task is already open.
//! - DECIDED, REMEMBER, PREFER and OPEN become decisions, facts, preferences
//!   and questions, skipping duplicates.
//! - HEARD pairs teach memory how a name gets misheard (an alias), so the
//!   next session recognizes it.
//! - Every item is linked to the projects and people it mentions, even when
//!   the name was misheard ("groc tesla" still links to grok-tesla).
//! - A `session` event records the conversation itself.

use crate::date::Date;
use crate::handoff::Handoff;
use crate::ledger::Event;
use crate::phonetic;
use crate::state::{Ids, Kind, Record, State, Status};
use crate::text::{self, slugify, words};
use std::collections::HashSet;

/// DONE line -> open task.
const MATCH_DONE: f64 = 0.45;
/// NEXT line repeats an open task.
const SAME_TASK: f64 = 0.72;
/// Any other repeated item.
const SAME_ITEM: f64 = 0.85;
/// Misheard name strong enough to link a topic automatically.
const LINK_SCORE: f64 = 0.88;
/// Misheard name worth showing in `vmem resolve`.
pub const SHOW_SCORE: f64 = 0.75;

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub slug: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Mention {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub slug: String,
    pub heard: String,
    pub score: f64,
    pub exact: bool,
}

/// Every named thing in memory (projects, people, terms) with its aliases.
pub struct EntityIndex {
    pub entities: Vec<Entity>,
}

fn contains_phrase(hay: &[String], needle: &[String]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

impl EntityIndex {
    pub fn from_state(st: &State) -> EntityIndex {
        let entities = st
            .records
            .iter()
            .filter(|r| r.kind.is_entity() && r.status == Status::Active)
            .map(|r| {
                let mut names = vec![r.name().to_string()];
                names.extend(r.aliases.iter().cloned());
                Entity {
                    id: r.id.clone(),
                    kind: r.kind,
                    name: r.name().to_string(),
                    slug: slugify(r.name()),
                    names,
                }
            })
            .collect();
        EntityIndex { entities }
    }

    /// Names mentioned in `text`: exact (alias included) or sounding alike.
    pub fn resolve(&self, text: &str) -> Vec<Mention> {
        let ws = words(text);
        let mut out: Vec<Mention> = Vec::new();
        for e in &self.entities {
            let exact = e.names.iter().find(|n| contains_phrase(&ws, &words(n)));
            if let Some(n) = exact {
                out.push(Mention {
                    id: e.id.clone(),
                    kind: e.kind,
                    name: e.name.clone(),
                    slug: e.slug.clone(),
                    heard: n.clone(),
                    score: 1.0,
                    exact: true,
                });
                continue;
            }
            let mut best: Option<(f64, String)> = None;
            for n in &e.names {
                let len = words(n).len();
                if len == 0 {
                    continue;
                }
                for size in len.saturating_sub(1).max(1)..=(len + 1).min(4) {
                    for gram in ws.windows(size) {
                        if gram.iter().all(|w| text::is_stopword(w)) || gram.concat().len() < 3 {
                            continue;
                        }
                        let heard = gram.join(" ");
                        let score = phonetic::similarity(&heard, n);
                        if best.as_ref().map_or(true, |(b, _)| score > *b) {
                            best = Some((score, heard));
                        }
                    }
                }
            }
            if let Some((score, heard)) = best {
                if score >= SHOW_SCORE {
                    out.push(Mention {
                        id: e.id.clone(),
                        kind: e.kind,
                        name: e.name.clone(),
                        slug: e.slug.clone(),
                        heard,
                        score,
                        exact: false,
                    });
                }
            }
        }
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap().then(a.name.cmp(&b.name)));
        out
    }

    /// The entity a (possibly misheard) name refers to.
    pub fn best(&self, name: &str) -> Option<&Entity> {
        let f = text::fold(name.trim());
        if let Some(e) = self.entities.iter().find(|e| e.names.iter().any(|n| text::fold(n) == f)) {
            return Some(e);
        }
        self.entities
            .iter()
            .map(|e| {
                let s = e.names.iter().map(|n| phonetic::similarity(name, n)).fold(0.0, f64::max);
                (s, e)
            })
            .filter(|(s, _)| *s >= 0.8)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .map(|(_, e)| e)
    }

    /// Topic slugs (projects and people) a text should link to.
    pub fn topics_in(&self, text: &str) -> Vec<String> {
        let mut v: Vec<String> = self
            .resolve(text)
            .into_iter()
            .filter(|m| m.kind.is_anchor() && (m.exact || m.score >= LINK_SCORE))
            .map(|m| m.slug)
            .collect();
        v.sort();
        v.dedup();
        v
    }
}

pub struct Plan {
    pub events: Vec<Event>,
    pub report: Vec<String>,
}

fn split_why(s: &str) -> (String, Option<String>) {
    for sep in [" — ", " -- ", " because ", " porque ", "; why: ", " (why: "] {
        if let Some((a, b)) = s.split_once(sep) {
            let why = b.trim().trim_end_matches(')').trim();
            if !a.trim().is_empty() && !why.is_empty() {
                return (a.trim().to_string(), Some(text::sentence(why)));
            }
        }
    }
    (s.trim().to_string(), None)
}

fn best_match<'a>(candidates: &[&'a Record], line: &str, min: f64) -> Option<(f64, &'a Record)> {
    candidates
        .iter()
        .map(|r| (text::similarity(line, &r.text), *r))
        .filter(|(s, _)| *s >= min)
        .max_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap()
                .then(text::id_number(&b.1.id).cmp(&text::id_number(&a.1.id)))
        })
}

/// Plans the events for one handoff. Nothing is written here.
pub fn plan(st: &State, h: &Handoff, at: Date, source: &str) -> Plan {
    let at_s = at.to_string();
    let mut ids = Ids::new(st);
    let index = EntityIndex::from_state(st);
    let explicit: Vec<String> = h.topics.iter().map(|t| slugify(t)).filter(|s| !s.is_empty()).collect();
    let mut all_text = h.summary.join(" ");
    for list in [&h.done, &h.next, &h.decided, &h.remember, &h.prefer, &h.open, &h.parked] {
        for x in list {
            all_text.push(' ');
            all_text.push_str(x);
        }
    }
    let mut session_topics = explicit.clone();
    session_topics.extend(index.topics_in(&all_text));
    session_topics.sort();
    session_topics.dedup();
    // An item links to what it names. If it names nothing and the whole
    // session was about one topic, it belongs to that topic.
    let link = |line: &str| -> Vec<String> {
        let mut t = index.topics_in(line);
        if t.is_empty() && session_topics.len() == 1 {
            t.push(session_topics[0].clone());
        }
        t
    };
    let mut events: Vec<Event> = Vec::new();
    let mut report: Vec<String> = Vec::new();
    let mut items: Vec<String> = Vec::new();
    let mut added: Vec<(Kind, String)> = Vec::new();

    // DONE: close the best matching open task.
    let open: Vec<&Record> = st
        .records
        .iter()
        .filter(|r| r.kind == Kind::Task && r.status == Status::Active)
        .collect();
    let mut closed: HashSet<String> = HashSet::new();
    for d in &h.done {
        let pool: Vec<&Record> = open.iter().copied().filter(|r| !closed.contains(&r.id)).collect();
        if let Some((_, r)) = best_match(&pool, d, MATCH_DONE) {
            let mut e = Event::new("done", &r.id, &at_s);
            e.note = Some(format!("Reported done: {d}"));
            events.push(e);
            closed.insert(r.id.clone());
            items.push(r.id.clone());
            report.push(format!("closed {} ({})", r.id, r.text));
        } else {
            report.push(format!("done, no matching task: {d}"));
        }
    }

    // NEXT and PARKED: new tasks unless already open.
    for (list, parked) in [(&h.next, false), (&h.parked, true)] {
        for t in list {
            let pool: Vec<&Record> = open.iter().copied().filter(|r| !closed.contains(&r.id)).collect();
            if let Some((_, r)) = best_match(&pool, t, SAME_TASK) {
                if parked && !r.parked {
                    let mut e = Event::new("set", &r.id, &at_s);
                    e.parked = Some(true);
                    events.push(e);
                }
                items.push(r.id.clone());
                report.push(format!("already open {} ({})", r.id, r.text));
                continue;
            }
            if added.iter().any(|(k, x)| *k == Kind::Task && text::similarity(x, t) >= SAME_TASK) {
                continue;
            }
            let id = ids.take(Kind::Task);
            let mut e = Event::add(&id, Kind::Task.name(), &at_s, t);
            e.topics = link(t);
            if parked {
                e.parked = Some(true);
            }
            e.source = Some(source.to_string());
            events.push(e);
            items.push(id.clone());
            added.push((Kind::Task, t.clone()));
            report.push(format!("new {id}{}: {t}", if parked { " (parked)" } else { "" }));
        }
    }

    // DECIDED, REMEMBER, PREFER, OPEN.
    let groups: [(&Vec<String>, Kind); 4] = [
        (&h.decided, Kind::Decision),
        (&h.remember, Kind::Fact),
        (&h.prefer, Kind::Preference),
        (&h.open, Kind::Question),
    ];
    for (list, kind) in groups {
        let existing = st.current_of(kind);
        for line in list {
            let (what, why) = if kind == Kind::Decision { split_why(line) } else { (line.trim().to_string(), None) };
            if let Some((_, r)) = best_match(&existing, &what, SAME_ITEM) {
                items.push(r.id.clone());
                report.push(format!("already known {} ({})", r.id, r.text));
                continue;
            }
            if added.iter().any(|(k, x)| *k == kind && text::similarity(x, &what) >= SAME_ITEM) {
                continue;
            }
            let id = ids.take(kind);
            let mut e = Event::add(&id, kind.name(), &at_s, &what);
            e.why = why;
            e.topics = link(&what);
            e.source = Some(source.to_string());
            events.push(e);
            items.push(id.clone());
            added.push((kind, what.clone()));
            report.push(format!("new {id}: {what}"));
        }
    }

    // HEARD: learn aliases.
    let mut heard_lines: Vec<String> = Vec::new();
    for (wrong, right) in &h.heard {
        heard_lines.push(format!("{wrong} → {right}"));
        match index.best(right) {
            Some(ent) => {
                let known = st
                    .get(&ent.id)
                    .map(|r| r.aliases.iter().any(|a| text::fold(a) == text::fold(wrong)))
                    .unwrap_or(false)
                    || text::fold(&ent.name) == text::fold(wrong);
                if known {
                    report.push(format!("alias already known: \"{wrong}\" means {}", ent.name));
                } else {
                    let mut e = Event::new("set", &ent.id, &at_s);
                    e.aliases = vec![wrong.clone()];
                    e.note = Some("learned from a misheard name".into());
                    events.push(e);
                    items.push(ent.id.clone());
                    report.push(format!("learned: \"{wrong}\" means {} ({})", ent.name, ent.id));
                }
            }
            None => {
                let id = ids.take(Kind::Term);
                let mut e = Event::add(&id, Kind::Term.name(), &at_s, &format!("Name to recognize: {right}."));
                e.title = Some(right.clone());
                e.aliases = vec![wrong.clone()];
                e.source = Some(source.to_string());
                events.push(e);
                items.push(id.clone());
                report.push(format!("new {id}: \"{wrong}\" means {right}"));
            }
        }
    }

    // The session itself.
    let mut topics = session_topics.clone();
    for e in &events {
        topics.extend(e.topics.iter().cloned());
    }
    topics.sort();
    topics.dedup();
    let summary = if h.summary.is_empty() {
        let first = h.next.first().or(h.done.first()).or(h.decided.first()).or(h.remember.first());
        match first {
            Some(x) => format!("Short session. First item: {}", text::clip(x, 120)),
            None => "Short session with no summary.".to_string(),
        }
    } else {
        text::clip(&h.summary.join(" "), 600)
    };
    let n = st.sessions.iter().filter(|s| s.at == at).count() + 1;
    let session_id = format!("s-{at_s}-{n}");
    let mut seen = HashSet::new();
    items.retain(|i| seen.insert(i.clone()));
    let mut s = Event::new("session", &session_id, &at_s);
    s.summary = Some(text::sentence(&summary));
    s.done = h.done.clone();
    s.items = items;
    s.topics = topics;
    s.heard = heard_lines;
    s.source = Some(source.to_string());
    events.push(s);
    report.push(format!("session {session_id} recorded"));
    Plan { events, report }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoff;
    use crate::ledger::Ledger;

    fn state() -> State {
        let l = Ledger::parse(concat!(
            r#"{"op":"add","id":"proj-1","at":"2026-11-20","kind":"project","title":"Grok Tesla","text":"Voice memory repo","aliases":["grok-tesla"]}"#, "\n",
            r#"{"op":"add","id":"task-1","at":"2026-11-20","kind":"task","text":"Complete the profile in memory"}"#, "\n",
            r#"{"op":"add","id":"task-2","at":"2026-11-21","kind":"task","text":"Record a demo video"}"#, "\n",
            r#"{"op":"add","id":"pref-1","at":"2026-11-20","kind":"preference","text":"Short answers, conclusion first"}"#, "\n",
            r#"{"op":"add","id":"pers-1","at":"2026-11-20","kind":"person","title":"Ana","text":"Owner"}"#, "\n",
        ).to_string());
        State::fold(&l.events, None)
    }

    #[test]
    fn plans_a_session() {
        let st = state();
        let h = handoff::parse(
            "HANDOFF 2026-11-25\nSUMMARY: Worked on groc tesla.\nDONE: completed the profile\nNEXT: record a demo video\nNEXT: publish the demo on LinkedIn\nDECIDED: Use English in the repo — any agent can read it\nPREFER: Short answers, conclusion first\nHEARD: Anna -> Ana; Jon -> John\nEND",
        );
        let p = plan(&st, &h, Date::parse("2026-11-25").unwrap(), "test");
        let ops: Vec<String> = p.events.iter().map(|e| format!("{} {}", e.op, e.id)).collect();
        assert_eq!(
            ops,
            vec![
                "done task-1",
                "add task-3",
                "add dec-1",
                "set pers-1",
                "add term-1",
                "session s-2026-11-25-1",
            ],
            "{:#?}",
            p.report
        );
        let dec = &p.events[2];
        assert_eq!(dec.text.as_deref(), Some("Use English in the repo"));
        assert_eq!(dec.why.as_deref(), Some("any agent can read it."));
        let session = p.events.last().unwrap();
        assert!(session.topics.contains(&"grok-tesla".to_string()), "{:?}", session.topics);
        assert!(session.items.contains(&"task-2".to_string()));
        assert!(session.items.contains(&"pref-1".to_string()));
    }

    #[test]
    fn resolves_misheard_names() {
        let st = state();
        let idx = EntityIndex::from_state(&st);
        let m = idx.resolve("we talked about groc tesla with anna");
        let names: Vec<&str> = m.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"Grok Tesla"), "{m:?}");
        assert!(names.contains(&"Ana"), "{m:?}");
        assert_eq!(idx.best("Anna").map(|e| e.id.as_str()), Some("pers-1"));
    }
}
