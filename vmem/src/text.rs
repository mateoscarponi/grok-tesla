//! Text helpers shared by every command: accent folding, words and search
//! tokens (English and Spanish), slugs, token estimates, similarity, and
//! cleanup so a line can be spoken aloud.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Lowercases and removes common Latin accents, so "Decisión" == "decision".
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        for l in c.to_lowercase() {
            out.push(match l {
                'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'ñ' => 'n',
                'ç' => 'c',
                'ý' | 'ÿ' => 'y',
                other => other,
            });
        }
    }
    out
}

/// Folded words: anything that is not a letter or a digit separates words.
pub fn words(s: &str) -> Vec<String> {
    fold(s)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

const STOPWORDS: &[&str] = &[
    // English
    "a", "about", "after", "all", "also", "an", "and", "any", "are", "as", "at", "be", "been",
    "but", "by", "can", "could", "did", "do", "does", "for", "from", "had", "has", "have", "he",
    "her", "here", "his", "how", "i", "if", "in", "into", "is", "it", "its", "just", "let", "me",
    "more", "most", "my", "no", "not", "now", "of", "on", "or", "our", "out", "she", "should",
    "so", "some", "such", "than", "that", "the", "their", "them", "then", "there", "these",
    "they", "this", "those", "to", "too", "up", "very", "was", "we", "were", "what", "when",
    "where", "which", "who", "why", "will", "with", "would", "you", "your",
    // Spanish
    "al", "como", "con", "cuando", "de", "del", "donde", "el", "ella", "en", "entre", "era",
    "es", "esa", "ese", "eso", "esta", "estan", "estas", "este", "esto", "estos", "fue", "hay",
    "la", "las", "le", "les", "lo", "los", "mas", "mi", "mis", "muy", "nos", "o", "para",
    "pero", "por", "porque", "que", "se", "ser", "si", "sin", "sobre", "son", "su", "sus",
    "tambien", "te", "tu", "tus", "un", "una", "unas", "uno", "unos", "vos", "y", "ya", "yo",
];

fn stopwords() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| STOPWORDS.iter().copied().collect())
}

pub fn is_stopword(w: &str) -> bool {
    stopwords().contains(w)
}

/// Light plural stripping for English and Spanish ("tasks" -> "task",
/// "decisiones" -> "decision"). Applied the same way to queries and documents.
pub fn stem(w: &str) -> String {
    if w.len() > 6 && w.ends_with("iones") {
        return w[..w.len() - 2].to_string();
    }
    if w.len() > 3
        && w.ends_with('s')
        && !w.ends_with("ss")
        && !w.ends_with("us")
        && !w.ends_with("is")
    {
        return w[..w.len() - 1].to_string();
    }
    w.to_string()
}

/// Search tokens: folded words without stopwords, lightly stemmed.
pub fn tokens(s: &str) -> Vec<String> {
    words(s)
        .into_iter()
        .filter(|w| !is_stopword(w))
        .map(|w| stem(&w))
        .collect()
}

/// "Open Road Trip!" -> "open-road-trip". At most 48 characters.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in fold(s).chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > 48 {
        out.truncate(48);
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

/// Rough token count for budgets: about four characters per token.
pub fn est_tokens(s: &str) -> usize {
    s.chars().count().div_ceil(4)
}

pub fn jaccard(a: &[String], b: &[String]) -> f64 {
    let sa: HashSet<&String> = a.iter().collect();
    let sb: HashSet<&String> = b.iter().collect();
    let union = sa.union(&sb).count();
    if union == 0 {
        return 0.0;
    }
    sa.intersection(&sb).count() as f64 / union as f64
}

/// Dice coefficient over character trigrams of the folded text.
pub fn trigram_dice(a: &str, b: &str) -> f64 {
    fn grams(s: &str) -> Vec<String> {
        let norm = words(s).join(" ");
        if norm.is_empty() {
            return Vec::new();
        }
        let padded: Vec<char> = format!("  {norm}  ").chars().collect();
        padded.windows(3).map(|g| g.iter().collect()).collect()
    }
    let ga = grams(a);
    let gb = grams(b);
    if ga.is_empty() || gb.is_empty() {
        return 0.0;
    }
    let mut counts: HashMap<&str, i32> = HashMap::new();
    for g in &ga {
        *counts.entry(g.as_str()).or_default() += 1;
    }
    let mut inter = 0usize;
    for g in &gb {
        if let Some(c) = counts.get_mut(g.as_str()) {
            if *c > 0 {
                *c -= 1;
                inter += 1;
            }
        }
    }
    2.0 * inter as f64 / (ga.len() + gb.len()) as f64
}

/// How alike two short texts are, from 0 to 1 (used for duplicates and for
/// matching "DONE" lines to open tasks).
pub fn similarity(a: &str, b: &str) -> f64 {
    jaccard(&tokens(a), &tokens(b)).max(trigram_dice(a, b))
}

/// Cuts `s` to at most `max` characters at a word boundary, adding "…".
pub fn clip(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        return s;
    }
    let cut: String = s.chars().take(max).collect();
    let cut = match cut.rfind(' ') {
        Some(i) if i > max / 2 => cut[..i].to_string(),
        _ => cut,
    };
    let cut = cut.trim_end_matches(|c: char| ",;:.-—".contains(c) || c.is_whitespace());
    format!("{cut}…")
}

/// Adds a final period unless the text already ends with punctuation.
pub fn sentence(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return String::new();
    }
    match t.chars().last() {
        Some('.') | Some('!') | Some('?') | Some('…') | Some(':') => t.to_string(),
        _ => format!("{t}."),
    }
}

/// Removes Markdown (links, emphasis, code marks, headings) so text can be
/// read aloud.
pub fn plain(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '[' {
            if let Some(close) = chars[i + 1..].iter().position(|&x| x == ']') {
                let close = i + 1 + close;
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(end) = chars[close + 2..].iter().position(|&x| x == ')') {
                        out.extend(&chars[i + 1..close]);
                        i = close + 2 + end + 1;
                        continue;
                    }
                }
            }
        }
        if c == '*' || c == '`' || c == '#' {
            i += 1;
            continue;
        }
        if c == '_' {
            let inner = i > 0
                && chars[i - 1].is_alphanumeric()
                && chars.get(i + 1).is_some_and(|n| n.is_alphanumeric());
            if !inner {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A rough check used by `doctor`: does this text look Spanish rather than
/// English? The repo convention is English text, whatever language is spoken.
pub fn looks_spanish(s: &str) -> bool {
    const ES: &[&str] = &[
        "que", "de", "la", "el", "los", "las", "para", "con", "por", "una", "pero", "como",
        "mas", "tambien", "esta", "estan", "porque", "cuando", "hay", "del", "al", "muy", "sin",
        "sobre", "todo", "quiere", "hacer", "tiene", "proximo", "paso",
    ];
    const EN: &[&str] = &[
        "the", "and", "to", "of", "is", "are", "for", "with", "that", "this", "it", "on", "in",
        "be", "as", "at", "by", "not", "or", "from", "when", "what", "wants", "has", "have",
    ];
    let ws = words(s);
    let es = ws.iter().filter(|w| ES.contains(&w.as_str())).count();
    let en = ws.iter().filter(|w| EN.contains(&w.as_str())).count();
    es >= 2 && es > en
}

/// Numeric part of an id such as "task-12", for natural ordering.
pub fn id_number(id: &str) -> u64 {
    id.rsplit('-').next().and_then(|n| n.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_accents_and_case() {
        assert_eq!(fold("Decisión Próxima AÑO"), "decision proxima ano");
    }

    #[test]
    fn tokens_drop_stopwords_and_plurals() {
        assert_eq!(tokens("The tasks for the decisiones"), vec!["task", "decision"]);
        assert_eq!(tokens("Ana quiere respuestas cortas"), vec!["ana", "quiere", "respuesta", "corta"]);
    }

    #[test]
    fn slugs_are_stable() {
        assert_eq!(slugify("Open Road Trip!"), "open-road-trip");
        assert_eq!(slugify("  grok-tesla  "), "grok-tesla");
        assert_eq!(slugify("Diseño & Código"), "diseno-codigo");
    }

    #[test]
    fn similarity_scores() {
        assert!(similarity("Complete the user profile", "complete user profile") > 0.9);
        assert!(similarity("Call the bank", "Write the landing page copy") < 0.3);
    }

    #[test]
    fn clip_and_sentence() {
        assert_eq!(clip("one two three four five", 12), "one two…");
        assert_eq!(sentence("Next step"), "Next step.");
        assert_eq!(sentence("Ready?"), "Ready?");
    }

    #[test]
    fn plain_removes_markdown() {
        assert_eq!(
            plain("**Next:** read [the brief](https://x.y/brief.md) and `vmem build`"),
            "Next: read the brief and vmem build"
        );
        assert_eq!(plain("keep snake_case words"), "keep snake_case words");
    }

    #[test]
    fn detects_spanish() {
        assert!(looks_spanish("Ana quiere que el próximo paso sea para la web"));
        assert!(!looks_spanish("Ana wants the next step to be the website"));
    }
}
