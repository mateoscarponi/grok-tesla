//! Word lists for the memory code: two easy words ("amber falcon") derived
//! from the content hash. A voice agent can say them, and the owner can ask
//! "what's the memory code?" to check it read the latest version instead of
//! a cached or imagined one.

pub const ADJECTIVES: [&str; 64] = [
    "amber", "azure", "bold", "brave", "bright", "brisk", "calm", "cedar", "clear", "cobalt",
    "copper", "coral", "cosmic", "crimson", "crisp", "dawn", "deep", "desert", "ember", "emerald",
    "fern", "golden", "granite", "green", "hazel", "honey", "indigo", "ivory", "jade", "jolly",
    "lunar", "maple", "marble", "mellow", "misty", "noble", "ocean", "olive", "onyx", "opal",
    "orange", "pearl", "polar", "quiet", "rapid", "river", "royal", "ruby", "rustic", "sandy",
    "scarlet", "silver", "solar", "sonic", "spring", "steady", "stone", "sunny", "swift", "teal",
    "tidal", "velvet", "violet", "wild",
];

pub const NOUNS: [&str; 64] = [
    "anchor", "arrow", "aspen", "badger", "beacon", "bison", "breeze", "canyon", "castle", "cloud",
    "comet", "condor", "coyote", "delta", "dolphin", "eagle", "falcon", "forest", "fox", "galaxy",
    "garden", "glacier", "harbor", "hawk", "heron", "horizon", "island", "jaguar", "lagoon",
    "lantern", "leopard", "lion", "lotus", "meadow", "mesa", "meteor", "moose", "mountain",
    "nebula", "oasis", "orbit", "otter", "panther", "pelican", "phoenix", "pine", "planet",
    "prairie", "quasar", "raven", "reef", "rocket", "sail", "sequoia", "sparrow", "summit",
    "thunder", "tiger", "trail", "tundra", "valley", "voyager", "willow", "zephyr",
];

/// Two words from the first two bytes of a hash.
pub fn code(hash: &[u8]) -> String {
    let a = hash.first().copied().unwrap_or(0) as usize % ADJECTIVES.len();
    let n = hash.get(1).copied().unwrap_or(0) as usize % NOUNS.len();
    format!("{} {}", ADJECTIVES[a], NOUNS[n])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_unique() {
        let mut a = ADJECTIVES.to_vec();
        a.sort();
        a.dedup();
        assert_eq!(a.len(), 64);
        let mut n = NOUNS.to_vec();
        n.sort();
        n.dedup();
        assert_eq!(n.len(), 64);
    }

    #[test]
    fn code_uses_two_words() {
        assert_eq!(code(&[0, 16]), "amber falcon");
    }
}
