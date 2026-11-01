//! Key-tip derivation (step 21.8, item U12).
//!
//! Key tips are derived from the existing spec ids — never invented labels.
//! The derivation is deterministic: the first ASCII alphanumeric character
//! (uppercased), with colliding entries numbered in spec order. The ribbon
//! overlay and the headless contract test therefore share one rule.

/// Derives one key tip per id, in list order.
pub fn key_tips(ids: &[&str]) -> Vec<String> {
    let mut tips: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        let base = id
            .chars()
            .find(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_uppercase().to_string())
            .unwrap_or_default();
        let mut tip = base.clone();
        let mut counter = 2;
        while tips.contains(&tip) {
            tip = format!("{base}{counter}");
            counter += 1;
        }
        tips.push(tip);
    }
    tips
}
