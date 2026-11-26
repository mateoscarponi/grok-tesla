//! Parses the end-of-session handoff a voice agent writes (format in
//! `core/handoff.md`):
//!
//! ```text
//! HANDOFF 2026-11-25
//! SUMMARY: We planned the v2 launch.
//! NEXT: Record the demo video
//! DECIDED: Launch on Monday — fewer people travel
//! HEARD: Matteo -> Mateo
//! END
//! ```
//!
//! It is tolerant on purpose, because the text may come from a transcript, a
//! phone dictation or another agent: English or Spanish keys, any case,
//! bullets, numbering, bold marks, a speaker label in front ("Grok: NEXT: …"),
//! several items in one line split with ";", lists under a bare key, and the
//! older one-line "For memory: …" form. If no key is found at all, the whole
//! text becomes the session summary.

use crate::date::Date;
use crate::text::{clip, fold};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Summary,
    Done,
    Next,
    Decided,
    Remember,
    Prefer,
    Open,
    Parked,
    Heard,
    Topics,
    Date,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Handoff {
    pub date: Option<Date>,
    pub summary: Vec<String>,
    pub done: Vec<String>,
    pub next: Vec<String>,
    pub decided: Vec<String>,
    pub remember: Vec<String>,
    pub prefer: Vec<String>,
    pub open: Vec<String>,
    pub parked: Vec<String>,
    pub heard: Vec<(String, String)>,
    pub topics: Vec<String>,
    pub had_block: bool,
}

impl Handoff {
    pub fn is_empty(&self) -> bool {
        self.summary.is_empty() && self.item_count() == 0
    }

    /// Items other than the summary.
    pub fn item_count(&self) -> usize {
        self.done.len()
            + self.next.len()
            + self.decided.len()
            + self.remember.len()
            + self.prefer.len()
            + self.open.len()
            + self.parked.len()
            + self.heard.len()
    }

    fn list(&mut self, k: Key) -> &mut Vec<String> {
        match k {
            Key::Summary => &mut self.summary,
            Key::Done => &mut self.done,
            Key::Next => &mut self.next,
            Key::Decided => &mut self.decided,
            Key::Remember => &mut self.remember,
            Key::Prefer => &mut self.prefer,
            Key::Open => &mut self.open,
            Key::Parked => &mut self.parked,
            Key::Topics => &mut self.topics,
            Key::Heard | Key::Date => unreachable!("handled separately"),
        }
    }

    fn push(&mut self, k: Key, value: &str) {
        let v = value.trim();
        if v.is_empty() || is_placeholder(v) {
            return;
        }
        match k {
            Key::Summary | Key::Decided => self.list(k).push(v.to_string()),
            Key::Heard => {
                for part in v.split([';', ',']) {
                    if let Some(pair) = parse_heard(part) {
                        self.heard.push(pair);
                    }
                }
            }
            Key::Topics => {
                for part in v.split([';', ',']) {
                    let p = part.trim();
                    if !p.is_empty() {
                        self.topics.push(p.to_string());
                    }
                }
            }
            Key::Date => {
                if let Some(d) = find_date(v) {
                    self.date = Some(d);
                }
            }
            _ => {
                for part in v.split(';') {
                    let p = part.trim();
                    if !p.is_empty() && !is_placeholder(p) {
                        self.list(k).push(p.to_string());
                    }
                }
            }
        }
    }

    /// A wrapped line continues the previous item of the same key.
    fn continue_last(&mut self, k: Key, line: &str) {
        if matches!(k, Key::Heard | Key::Topics | Key::Date) {
            return self.push(k, line);
        }
        let list = self.list(k);
        match list.last_mut() {
            Some(last) => {
                last.push(' ');
                last.push_str(line.trim());
            }
            None => list.push(line.trim().to_string()),
        }
    }
}

/// Maps a key label (English or Spanish) to its meaning.
pub fn key_for(label: &str) -> Option<Key> {
    let l = fold(label);
    let l: String = l
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Some(match l.as_str() {
        "summary" | "recap" | "resumen" | "sintesis" => Key::Summary,
        "done" | "did" | "completed" | "finished" | "hecho" | "hechos" | "hicimos" | "terminado"
        | "terminamos" | "listo" => Key::Done,
        "next" | "next step" | "next steps" | "todo" | "to do" | "siguiente" | "siguientes"
        | "siguiente paso" | "proximo" | "proximo paso" | "proximos pasos" | "pendiente"
        | "pendientes" | "tarea" | "tareas" => Key::Next,
        "decided" | "decision" | "decisions" | "decidido" | "decidimos" | "decisiones" => Key::Decided,
        "remember" | "for memory" | "memory" | "note" | "fact" | "recordar" | "recorda"
        | "recuerda" | "para la memoria" | "memoria" | "dato" | "nota" => Key::Remember,
        "prefer" | "prefers" | "preference" | "preferences" | "prefiere" | "preferencia"
        | "preferencias" => Key::Prefer,
        "open" | "question" | "questions" | "open question" | "open questions" | "pregunta"
        | "preguntas" | "duda" | "dudas" | "abierto" => Key::Open,
        "parked" | "later" | "for later" | "needs a screen" | "estacionado" | "para despues"
        | "despues" | "aparcado" => Key::Parked,
        "heard" | "misheard" | "asr" | "escuchado" | "oido" | "correccion" | "correcciones" => Key::Heard,
        "topics" | "topic" | "tags" | "temas" | "tema" => Key::Topics,
        "date" | "fecha" => Key::Date,
        _ => return None,
    })
}

fn is_placeholder(v: &str) -> bool {
    let f: String = fold(v).chars().filter(|c| c.is_alphanumeric()).collect();
    matches!(f.as_str(), "" | "none" | "na" | "nothing" | "nada" | "ninguno" | "ninguna")
}

fn strip_bullet(line: &str) -> &str {
    let t = line.trim_start();
    for p in ["- ", "* ", "• ", "+ ", "> "] {
        if let Some(r) = t.strip_prefix(p) {
            return r.trim_start();
        }
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if (1..3).contains(&digits) {
        let rest = &t[digits..];
        if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return r.trim_start();
        }
    }
    t
}

fn trim_marks(s: &str) -> &str {
    s.trim().trim_matches(|c| c == '*' || c == '_').trim()
}

fn try_key(s: &str) -> Option<(Key, String)> {
    let s = s.trim_start_matches(|c| c == '*' || c == '_');
    let pos = s.find(':')?;
    if pos == 0 || pos > 40 {
        return None;
    }
    let key = key_for(&s[..pos])?;
    Some((key, trim_marks(&s[pos + 1..]).to_string()))
}

/// Finds "KEY: value" in a line, also behind a short speaker label.
fn split_key(line: &str) -> Option<(Key, String)> {
    let body = strip_bullet(line);
    if let Some(kv) = try_key(body) {
        return Some(kv);
    }
    let pos = body.find(':')?;
    let label = &body[..pos];
    if label.len() <= 20 && label.split_whitespace().count() <= 2 {
        return try_key(body[pos + 1..].trim_start());
    }
    None
}

fn first_word(line: &str) -> String {
    fold(strip_bullet(line))
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}

fn is_block_start(line: &str) -> bool {
    matches!(first_word(line).as_str(), "handoff" | "handover" | "traspaso" | "relevo")
        && split_key(line).is_none()
}

fn is_block_end(line: &str) -> bool {
    let f: String = fold(line).chars().filter(|c| c.is_alphanumeric() || *c == ' ').collect();
    matches!(f.trim(), "end" | "fin" | "end handoff" | "handoff end" | "fin traspaso")
}

/// The first `YYYY-MM-DD` date found anywhere in the text.
pub fn find_date(s: &str) -> Option<Date> {
    let b = s.as_bytes();
    if b.len() < 10 {
        return None;
    }
    (0..=b.len() - 10)
        .filter(|&i| s.is_char_boundary(i) && s.is_char_boundary(i + 10))
        .find_map(|i| Date::parse(&s[i..i + 10]))
}

fn parse_heard(s: &str) -> Option<(String, String)> {
    for sep in ["->", "→", "=>", " = ", " means ", " is ", " es "] {
        if let Some((a, b)) = s.split_once(sep) {
            let a = a.trim().trim_matches(|c| c == '"' || c == '\'' || c == '“' || c == '”').trim();
            let b = b.trim().trim_matches(|c| c == '"' || c == '\'' || c == '“' || c == '”').trim();
            if !a.is_empty() && !b.is_empty() {
                return Some((a.to_string(), b.to_string()));
            }
        }
    }
    None
}

/// Parses handoff text. Uses the last HANDOFF block if there is one,
/// otherwise scans every line for keys.
pub fn parse(input: &str) -> Handoff {
    // GitHub issue forms add "### Label" headings and "_No response_"
    // placeholders. Drop them, but keep a heading that carries a date.
    let text: String = input
        .replace("\r\n", "\n")
        .lines()
        .filter_map(|l| {
            let t = l.trim();
            if t == "_No response_" {
                None
            } else if t.starts_with('#') {
                find_date(t).map(|_| t.trim_start_matches('#').trim().to_string())
            } else {
                Some(l.to_string())
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let lines: Vec<&str> = text.lines().collect();
    let mut h = Handoff::default();
    let block_end = |s: usize| {
        lines[s + 1..]
            .iter()
            .position(|l| is_block_end(l) || is_block_start(l))
            .map(|p| s + 1 + p)
            .unwrap_or(lines.len())
    };
    // The last block that really holds keys ("Handoff ready." after the
    // block is not a block).
    let chosen = (0..lines.len())
        .rev()
        .filter(|&i| is_block_start(lines[i]))
        .map(|s| (s, block_end(s)))
        .find(|&(s, end)| lines[s + 1..end].iter().any(|l| split_key(l).is_some()));
    let (from, to) = match chosen {
        Some((s, end)) => {
            h.had_block = true;
            h.date = find_date(lines[s]);
            (s + 1, end)
        }
        None => (0, lines.len()),
    };
    let mut current: Option<Key> = None;
    for line in &lines[from..to] {
        let t = line.trim();
        if t.is_empty() {
            current = None;
            continue;
        }
        if t.starts_with("```") {
            continue;
        }
        if let Some((k, v)) = split_key(t) {
            h.push(k, &v);
            current = if k == Key::Date { None } else { Some(k) };
            continue;
        }
        match current {
            Some(k) => {
                let bullet = strip_bullet(t);
                if bullet.len() < t.len() {
                    h.push(k, trim_marks(bullet));
                } else {
                    h.continue_last(k, t);
                }
            }
            None if h.had_block => h.push(Key::Summary, t),
            None => {}
        }
    }
    if h.is_empty() {
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if !flat.is_empty() {
            h.summary.push(clip(&flat, 600));
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_full_block() {
        let h = parse(
            "Some chatter before.\nHANDOFF 2026-11-25\nSUMMARY: We planned the launch.\nDONE: wrote the README\nNEXT: record the demo; send it to Kai\nDECIDED: Launch Monday — fewer people travel\nREMEMBER: The demo must be under two minutes\nPREFER: Numbers rounded\nOPEN: Which price?\nPARKED: Review the slides\nHEARD: Matteo -> Mateo, Groc → Grok\nTOPICS: grok-tesla\nEND\nAfter the block.",
        );
        assert!(h.had_block);
        assert_eq!(h.date.unwrap().to_string(), "2026-11-25");
        assert_eq!(h.summary, vec!["We planned the launch."]);
        assert_eq!(h.done, vec!["wrote the README"]);
        assert_eq!(h.next, vec!["record the demo", "send it to Kai"]);
        assert_eq!(h.decided, vec!["Launch Monday — fewer people travel"]);
        assert_eq!(h.remember.len(), 1);
        assert_eq!(h.prefer, vec!["Numbers rounded"]);
        assert_eq!(h.open, vec!["Which price?"]);
        assert_eq!(h.parked, vec!["Review the slides"]);
        assert_eq!(
            h.heard,
            vec![("Matteo".to_string(), "Mateo".to_string()), ("Groc".to_string(), "Grok".to_string())]
        );
        assert_eq!(h.topics, vec!["grok-tesla"]);
    }

    #[test]
    fn accepts_spanish_keys_bullets_and_lists() {
        let h = parse(
            "traspaso\n- **Resumen:** Armamos el plan\n- Siguiente:\n  - llamar al banco\n  - enviar la propuesta\n- Para la memoria: le gusta el café\n- Pendiente: n/a\nfin",
        );
        assert_eq!(h.summary, vec!["Armamos el plan"]);
        assert_eq!(h.next, vec!["llamar al banco", "enviar la propuesta"]);
        assert_eq!(h.remember, vec!["le gusta el café"]);
    }

    #[test]
    fn finds_keys_in_a_transcript_without_block() {
        let h = parse("Mateo: let's stop here\nGrok: For memory: Mateo wants rounded numbers.\nGrok: Next step: call the bank\nwith the new numbers");
        assert!(!h.had_block);
        assert_eq!(h.remember, vec!["Mateo wants rounded numbers."]);
        assert_eq!(h.next, vec!["call the bank with the new numbers"]);
    }

    #[test]
    fn free_text_becomes_the_summary() {
        let h = parse("We talked about pricing and nothing else.");
        assert_eq!(h.summary, vec!["We talked about pricing and nothing else."]);
        assert_eq!(h.item_count(), 0);
    }

    #[test]
    fn picks_the_block_with_keys() {
        let h = parse("HANDOFF 2026-11-24\nNEXT: old draft\nEND\nHANDOFF 2026-11-25\nNEXT: final item\nEND\nGrok: Handoff ready.");
        assert_eq!(h.date.unwrap().to_string(), "2026-11-25");
        assert_eq!(h.next, vec!["final item"]);
    }

    #[test]
    fn reads_github_issue_form_bodies() {
        let h = parse("### Handoff\n\nSUMMARY: From the phone\nNEXT: call the bank\n\n### Notes\n\n_No response_");
        assert_eq!(h.summary, vec!["From the phone"]);
        assert_eq!(h.next, vec!["call the bank"]);
        let h = parse("### Handoff\n\nWe only talked about prices.\n\n### Notes\n\n_No response_");
        assert_eq!(h.summary, vec!["We only talked about prices."]);
        let h = parse("## HANDOFF 2026-11-24\nNEXT: x\nEND");
        assert_eq!(h.date.unwrap().to_string(), "2026-11-24");
    }

    #[test]
    fn ignores_placeholders_and_code_fences() {
        let h = parse("```\nHANDOFF\nSUMMARY: Short call\nOPEN: none\nPARKED: -\n```");
        assert_eq!(h.summary, vec!["Short call"]);
        assert!(h.open.is_empty() && h.parked.is_empty());
    }
}
