//! The memory ledger: `memory/ledger.jsonl`.
//!
//! One JSON event per line, append-only. It is the single source of truth:
//! every pack, view and topic page is rebuilt from it. Nothing is ever
//! deleted. A wrong item is retired, a changed one is superseded, so the
//! history stays readable (`vmem history <id>`).
//!
//! Operations:
//! - `add`: a new item (`kind`, `text` and optional fields).
//! - `set`: changes fields of an item. Lists (aliases, topics, tags) are merged.
//! - `done`: a task finished or a question answered (`note` optional).
//! - `retire`: an item that is no longer true (`note` is the reason).
//! - `session`: one conversation: `summary`, `done` lines, `items` it
//!   created or touched, `topics`, `heard` corrections.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

pub const LEDGER_PATH: &str = "memory/ledger.jsonl";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub op: String,
    pub id: String,
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub say: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heard: Vec<String>,
}

impl Event {
    pub fn new(op: &str, id: &str, at: &str) -> Event {
        Event {
            op: op.to_string(),
            id: id.to_string(),
            at: at.to_string(),
            ..Event::default()
        }
    }

    pub fn add(id: &str, kind: &str, at: &str, text: &str) -> Event {
        Event {
            kind: Some(kind.to_string()),
            text: Some(text.to_string()),
            ..Event::new("add", id, at)
        }
    }

    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("an event always serializes")
    }
}

/// The ledger as read from disk: parsed events with their line numbers, and
/// lines that failed to parse.
#[derive(Debug, Default)]
pub struct Ledger {
    pub events: Vec<(usize, Event)>,
    pub errors: Vec<(usize, String)>,
    pub raw: String,
}

impl Ledger {
    pub fn load(path: &Path) -> Result<Ledger, String> {
        let raw = match fs::read_to_string(path) {
            Ok(s) => s.replace("\r\n", "\n"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
        };
        Ok(Ledger::parse(raw))
    }

    pub fn parse(raw: String) -> Ledger {
        let mut ledger = Ledger::default();
        for (i, line) in raw.lines().enumerate() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            match serde_json::from_str::<Event>(t) {
                Ok(ev) => ledger.events.push((i + 1, ev)),
                Err(e) => ledger.errors.push((i + 1, e.to_string())),
            }
        }
        ledger.raw = raw;
        ledger
    }
}

/// Appends events to the ledger file, one line each.
pub fn append(path: &Path, events: &[Event]) -> Result<(), String> {
    if events.is_empty() {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let needs_newline = fs::read(path)
        .map(|b| !b.is_empty() && !b.ends_with(b"\n"))
        .unwrap_or(false);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut buf = String::new();
    if needs_newline {
        buf.push('\n');
    }
    for e in events {
        buf.push_str(&e.to_line());
        buf.push('\n');
    }
    f.write_all(buf.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_skips_empty_fields() {
        let e = Event::add("pref-1", "preference", "2026-11-22", "Short answers.");
        let line = e.to_line();
        assert_eq!(
            line,
            r#"{"op":"add","id":"pref-1","at":"2026-11-22","kind":"preference","text":"Short answers."}"#
        );
        let back: Event = serde_json::from_str(&line).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn reports_bad_lines_with_numbers() {
        let l = Ledger::parse("{\"op\":\"add\",\"id\":\"a\",\"at\":\"2026-01-01\"}\n\nnot json\n{\"op\":\"x\",\"id\":\"b\",\"at\":\"2026-01-01\",\"typo\":1}\n".into());
        assert_eq!(l.events.len(), 1);
        assert_eq!(l.errors.len(), 2);
        assert_eq!(l.errors[0].0, 3);
        assert_eq!(l.errors[1].0, 4);
    }
}
