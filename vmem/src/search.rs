//! Keyword search over memory, with no embeddings and no server.
//!
//! Relevance is BM25 over every item and session. It is fused with recency
//! (reciprocal rank fusion), so among equally relevant results the newest
//! wins. A query word that appears nowhere in memory is expanded to the
//! words that sound like it, so a misheard "groc" still finds "Grok".

use crate::date::Date;
use crate::phonetic;
use crate::state::{Kind, State, Status};
use crate::text::{clip, tokens};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Hit {
    pub id: String,
    pub kind: String,
    pub date: Date,
    pub text: String,
    pub status: String,
    pub score: f64,
}

pub struct Options {
    pub limit: usize,
    pub kind: Option<Kind>,
    /// Include done, retired and superseded items.
    pub all: bool,
}

struct Doc {
    id: String,
    kind: String,
    date: Date,
    text: String,
    snippet: String,
    status: String,
    pin: bool,
}

fn docs(st: &State, opt: &Options) -> Vec<Doc> {
    let mut out = Vec::new();
    for r in &st.records {
        if opt.kind.is_some_and(|k| k != r.kind) {
            continue;
        }
        let live = st.is_current(r) || st.is_upcoming(r);
        if !live && !opt.all {
            continue;
        }
        let snippet = match (&r.title, r.kind.is_entity()) {
            (Some(t), true) => format!("{t}: {}", r.text),
            _ => r.text.clone(),
        };
        out.push(Doc {
            id: r.id.clone(),
            kind: r.kind.name().to_string(),
            date: r.last_seen(),
            text: r.search_text(),
            snippet,
            status: if r.status == Status::Active { "active".into() } else { r.status.name().into() },
            pin: r.pin,
        });
    }
    if opt.kind.is_none() {
        for s in &st.sessions {
            let mut t = s.summary.clone();
            for d in &s.done {
                t.push(' ');
                t.push_str(d);
            }
            for topic in &s.topics {
                t.push(' ');
                t.push_str(topic);
            }
            out.push(Doc {
                id: s.id.clone(),
                kind: "session".into(),
                date: s.at,
                text: t,
                snippet: s.summary.clone(),
                status: "session".into(),
                pin: false,
            });
        }
    }
    out
}

pub fn search(st: &State, query: &str, opt: &Options) -> Vec<Hit> {
    let docs = docs(st, opt);
    if docs.is_empty() {
        return Vec::new();
    }
    let toks: Vec<Vec<String>> = docs.iter().map(|d| tokens(&d.text)).collect();
    let n = docs.len() as f64;
    let avgdl = (toks.iter().map(Vec::len).sum::<usize>() as f64 / n).max(1.0);
    let mut df: HashMap<&str, usize> = HashMap::new();
    for t in &toks {
        let mut uniq: Vec<&str> = t.iter().map(String::as_str).collect();
        uniq.sort_unstable();
        uniq.dedup();
        for w in uniq {
            *df.entry(w).or_default() += 1;
        }
    }
    let mut q = tokens(query);
    q.sort();
    q.dedup();
    // Exact words count fully; words that sound alike count a bit less. Both
    // are always used, because a misheard spelling can end up in memory too
    // (a session summary that says "groc" must not hide the "Grok" project).
    let mut terms: Vec<(String, f64)> = Vec::new();
    for t in &q {
        if df.contains_key(t.as_str()) {
            terms.push((t.clone(), 1.0));
        }
        let mut sounds: Vec<&str> = df
            .keys()
            .copied()
            .filter(|v| *v != t.as_str() && v.len() >= 3 && phonetic::similarity(t, v) >= 0.85)
            .collect();
        sounds.sort_unstable();
        for v in sounds {
            terms.push((v.to_string(), 0.7));
        }
    }
    if terms.is_empty() {
        return Vec::new();
    }
    let (k1, b) = (1.2, 0.75);
    let mut scored: Vec<(usize, f64)> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let dl = t.len() as f64;
        let mut s = 0.0;
        for (term, w) in &terms {
            let tf = t.iter().filter(|x| *x == term).count() as f64;
            if tf == 0.0 {
                continue;
            }
            let d = *df.get(term.as_str()).unwrap_or(&0) as f64;
            let idf = (1.0 + (n - d + 0.5) / (d + 0.5)).ln();
            s += w * idf * tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * dl / avgdl));
        }
        if s > 0.0 {
            scored.push((i, s));
        }
    }
    let mut by_rel = scored.clone();
    by_rel.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    let mut by_date: Vec<usize> = scored.iter().map(|(i, _)| *i).collect();
    by_date.sort_by(|a, b| docs[*b].date.cmp(&docs[*a].date).then(a.cmp(b)));
    let mut fused: Vec<(usize, f64)> = by_rel
        .iter()
        .enumerate()
        .map(|(rank, (i, _))| {
            let rec = by_date.iter().position(|x| x == i).unwrap_or(0);
            let mut f = 1.0 / (60.0 + rank as f64 + 1.0) + 0.5 / (60.0 + rec as f64 + 1.0);
            if docs[*i].pin {
                f += 0.002;
            }
            if docs[*i].status != "active" && docs[*i].status != "session" {
                f -= 0.003;
            }
            (*i, f)
        })
        .collect();
    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
    fused
        .into_iter()
        .take(opt.limit)
        .map(|(i, f)| Hit {
            id: docs[i].id.clone(),
            kind: docs[i].kind.clone(),
            date: docs[i].date,
            text: clip(&docs[i].snippet, 200),
            status: docs[i].status.clone(),
            score: (f * 10_000.0).round() / 10_000.0,
        })
        .collect()
}

/// Search hits as a readable list.
pub fn format_hits(hits: &[Hit]) -> String {
    if hits.is_empty() {
        return "No matches.".into();
    }
    let mut s = String::new();
    for h in hits {
        let status = if h.status == "active" || h.status == "session" { String::new() } else { format!(" [{}]", h.status) };
        s.push_str(&format!("- {} ({}, {}){}: {}\n", h.id, h.kind, h.date, status, h.text));
    }
    s.trim_end().to_string()
}

pub fn hits_json(hits: &[Hit]) -> String {
    let v: Vec<serde_json::Value> = hits
        .iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id, "kind": h.kind, "date": h.date.to_string(),
                "status": h.status, "text": h.text, "score": h.score
            })
        })
        .collect();
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// The words in memory a query word sounds like.
#[cfg(test)]
pub fn sounds_like(st: &State, word: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for r in &st.records {
        for t in tokens(&r.search_text()) {
            if t.len() >= 3 && phonetic::similarity(word, &t) >= 0.85 && !v.contains(&t) {
                v.push(t);
            }
        }
    }
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Ledger;

    fn st() -> State {
        let l = Ledger::parse(concat!(
            r#"{"op":"add","id":"proj-1","at":"2026-11-20","kind":"project","title":"Grok Tesla","text":"Voice memory repository"}"#, "\n",
            r#"{"op":"add","id":"dec-1","at":"2026-11-21","kind":"decision","text":"Pricing stays at the old level"}"#, "\n",
            r#"{"op":"add","id":"dec-2","at":"2026-11-24","kind":"decision","text":"New pricing page for the studio"}"#, "\n",
            r#"{"op":"add","id":"fact-1","at":"2026-11-24","kind":"fact","text":"The bank meeting is on Friday"}"#, "\n",
            r#"{"op":"retire","id":"dec-1","at":"2026-11-24","note":"replaced"}"#, "\n",
        ).to_string());
        State::fold(&l.events, None)
    }

    #[test]
    fn finds_by_keyword_and_skips_retired() {
        let s = st();
        let hits = search(&s, "pricing", &Options { limit: 5, kind: None, all: false });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "dec-2");
        let all = search(&s, "pricing", &Options { limit: 5, kind: None, all: true });
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "dec-2");
    }

    #[test]
    fn misheard_words_still_match() {
        let s = st();
        let hits = search(&s, "groc", &Options { limit: 5, kind: None, all: false });
        assert_eq!(hits.first().map(|h| h.id.as_str()), Some("proj-1"));
        assert_eq!(sounds_like(&s, "groc"), vec!["grok"]);
    }

    #[test]
    fn filters_by_kind() {
        let s = st();
        let hits = search(&s, "bank pricing", &Options { limit: 5, kind: Some(Kind::Fact), all: false });
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "fact-1");
    }
}
