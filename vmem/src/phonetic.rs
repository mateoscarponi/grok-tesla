//! Approximate phonetic keys for English and Spanish names.
//!
//! Speech recognition mishears names: "Grok" becomes "Groc", "Mateo" becomes
//! "Matteo", "Hunter" becomes "Junter". Two spellings that sound alike get the
//! same (or a very close) key, so `vmem` can still find the right entry.
//!
//! The key keeps consonant sounds and drops vowels after the first letter:
//! b/v/p merge (Spanish b and v sound alike), d/t and g/k merge (voiced and
//! voiceless pairs that recognizers confuse), soft c/z/s merge, h and j are
//! dropped, and sh/ch/ll/consonant y merge (Rioplatense Spanish pronounces
//! "ll" and "y" like "sh").

use crate::text::fold;

/// Phonetic key of a phrase: one key per word, separated by spaces.
pub fn key(s: &str) -> String {
    fold(s)
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(word_key)
        .filter(|k| !k.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

fn word_key(w: &str) -> String {
    let c: Vec<char> = w.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let n = c.len();
    let at = |i: usize| if i < n { c[i] } else { ' ' };
    let mut out = String::new();
    let mut i = 0;
    if n >= 2 && matches!((c[0], c[1]), ('k', 'n') | ('w', 'r') | ('p', 's') | ('p', 'n') | ('g', 'n')) {
        i = 1;
    }
    while i < n {
        let ch = c[i];
        let nx = at(i + 1);
        if ch.is_ascii_digit() {
            out.push(ch);
            i += 1;
            continue;
        }
        let mut step = 1;
        let code: &str = match ch {
            'a' | 'e' | 'i' | 'o' | 'u' | 'w' => {
                if out.is_empty() {
                    "a"
                } else {
                    ""
                }
            }
            'y' => {
                let starts_syllable = i == 0 || is_vowel(at(i.wrapping_sub(1)));
                if is_vowel(nx) && starts_syllable {
                    "x"
                } else if out.is_empty() {
                    "a"
                } else {
                    ""
                }
            }
            'p' if nx == 'h' => {
                step = 2;
                "f"
            }
            'b' | 'v' | 'p' => "p",
            'f' => "f",
            'd' => "t",
            't' => {
                if nx == 'h' {
                    step = 2;
                }
                "t"
            }
            'q' => {
                if nx == 'u' {
                    step = 2;
                }
                "k"
            }
            'k' => "k",
            'c' => match nx {
                'e' | 'i' | 'y' => "s",
                'h' => {
                    step = 2;
                    "x"
                }
                'k' => {
                    step = 2;
                    "k"
                }
                _ => "k",
            },
            'g' => match nx {
                'e' | 'i' | 'y' => "",
                'u' if matches!(at(i + 2), 'e' | 'i') => {
                    step = 2;
                    "k"
                }
                'h' => {
                    step = 2;
                    "k"
                }
                _ => "k",
            },
            'j' | 'h' => "",
            's' => {
                if nx == 'h' {
                    step = 2;
                    "x"
                } else {
                    "s"
                }
            }
            'z' => "s",
            'x' => {
                if out.is_empty() {
                    "s"
                } else {
                    "ks"
                }
            }
            'l' => {
                if nx == 'l' {
                    step = 2;
                    "x"
                } else {
                    "l"
                }
            }
            'r' => "r",
            'm' => "m",
            'n' => "n",
            _ => "",
        };
        out.push_str(code);
        i += step;
    }
    let mut k = String::with_capacity(out.len());
    for ch in out.chars() {
        if !k.ends_with(ch) {
            k.push(ch);
        }
    }
    k
}

/// Edit distance between two character sequences.
pub fn lev(a: &[char], b: &[char]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn ratio(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0.0;
    }
    1.0 - lev(&a, &b) as f64 / longest as f64
}

/// How alike two names sound, from 0 to 1. Blends the phonetic key with the
/// spelling, and is stricter for very short keys, which match too easily.
pub fn similarity(a: &str, b: &str) -> f64 {
    let ka = key(a).replace(' ', "");
    let kb = key(b).replace(' ', "");
    if ka.is_empty() || kb.is_empty() {
        return 0.0;
    }
    let sa = crate::text::words(a).join(" ");
    let sb = crate::text::words(b).join(" ");
    let phon = ratio(&ka, &kb);
    let spell = ratio(&sa, &sb);
    if ka.len().max(kb.len()) <= 2 {
        (phon + spell) / 2.0
    } else {
        0.6 * phon + 0.4 * spell
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn misheard_names_share_keys() {
        assert_eq!(key("Grok"), key("Groc"));
        assert_eq!(key("Grok"), key("Grock"));
        assert_eq!(key("Mateo"), key("Matteo"));
        assert_eq!(key("Hunter"), key("Junter"));
        assert_eq!(key("Built by Oak"), key("build by oke"));
        assert_eq!(key("Kai"), key("Kye"));
        assert_eq!(key("llamas"), key("yamas"));
    }

    #[test]
    fn similarity_separates_names() {
        assert!(similarity("Groc", "Grok") >= 0.85);
        assert!(similarity("Matteo", "Mateo") >= 0.85);
        assert!(similarity("Escarponi", "Scarponi") >= 0.8);
        assert!(similarity("Mario", "Mateo") < 0.6);
        assert!(similarity("work", "Grok") < 0.7);
        assert!(similarity("Tesla", "Texla") >= 0.75);
    }

    #[test]
    fn edit_distance() {
        let a: Vec<char> = "kitten".chars().collect();
        let b: Vec<char> = "sitting".chars().collect();
        assert_eq!(lev(&a, &b), 3);
    }
}
