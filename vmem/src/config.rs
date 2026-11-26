//! `vmem.json`: settings for one memory repo. Every field has a default, so a
//! minimal file only needs `owner` and `repo`. `vmem` finds the repo root by
//! walking up from the current folder until it sees this file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "vmem.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub schema: String,
    /// Name shown at the top of every pack.
    pub title: String,
    pub tagline: String,
    /// The person the memory belongs to.
    pub owner: String,
    /// The voice agent that reads it.
    pub agent: String,
    /// GitHub "owner/name".
    pub repo: String,
    pub branch: String,
    /// The language the agent speaks with the owner.
    pub speak: String,
    pub budgets: Budgets,
    pub stale_days: StaleDays,
    /// Saved task packs (`vmem context "<goal>" --save <slug>`).
    pub packs: Vec<CustomPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Budgets {
    /// Token budget of the boot pack (the README block).
    pub boot: usize,
    pub brief: usize,
    /// Above this the full pack gets a doctor warning.
    pub full: usize,
    pub spoken_words: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct StaleDays {
    pub task: i64,
    pub question: i64,
    pub project: i64,
    pub fact: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CustomPack {
    pub slug: String,
    pub goal: String,
    pub budget: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            schema: "vmem-config/1".into(),
            title: "Voice memory".into(),
            tagline: "Portable memory for voice AI agents".into(),
            owner: "the owner".into(),
            agent: "the voice agent".into(),
            repo: "owner/repo".into(),
            branch: "main".into(),
            speak: "English".into(),
            budgets: Budgets::default(),
            stale_days: StaleDays::default(),
            packs: Vec::new(),
        }
    }
}

impl Default for Budgets {
    fn default() -> Self {
        Budgets { boot: 1400, brief: 3200, full: 16_000, spoken_words: 90 }
    }
}

impl Default for StaleDays {
    fn default() -> Self {
        StaleDays { task: 30, question: 30, project: 60, fact: 180 }
    }
}

impl Config {
    pub fn load(root: &Path) -> Result<Config, String> {
        let path = root.join(CONFIG_FILE);
        let raw = fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save(&self, root: &Path) -> Result<(), String> {
        let path = root.join(CONFIG_FILE);
        let mut s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        s.push('\n');
        fs::write(&path, s).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// Plain-text URL of a file, the easiest form for an agent to read.
    pub fn raw_url(&self, path: &str) -> String {
        format!("https://raw.githubusercontent.com/{}/{}/{}", self.repo, self.branch, path)
    }

    /// GitHub page of a file, the fallback when raw links fail.
    pub fn blob_url(&self, path: &str) -> String {
        format!("https://github.com/{}/blob/{}/{}", self.repo, self.branch, path)
    }

    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo)
    }
}

/// The repo root: `explicit`, or the nearest parent folder holding vmem.json.
pub fn find_root(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return if p.join(CONFIG_FILE).is_file() {
            Ok(p.to_path_buf())
        } else {
            Err(format!("no {CONFIG_FILE} in {}", p.display()))
        };
    }
    let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        if dir.join(CONFIG_FILE).is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(format!(
                "no {CONFIG_FILE} found here or in any parent folder (run `vmem init` to create a memory repo, or pass --root)"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_file_gets_defaults() {
        let c: Config = serde_json::from_str(r#"{"owner":"Ana","repo":"ana/mem"}"#).unwrap();
        assert_eq!(c.owner, "Ana");
        assert_eq!(c.branch, "main");
        assert_eq!(c.budgets.boot, 1400);
        assert_eq!(c.raw_url("packs/boot.md"), "https://raw.githubusercontent.com/ana/mem/main/packs/boot.md");
    }

    #[test]
    fn rejects_typos() {
        assert!(serde_json::from_str::<Config>(r#"{"ownr":"Ana"}"#).is_err());
    }
}
