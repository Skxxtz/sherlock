use std::sync::LazyLock;

fn search_score(query: &str, match_in: &str) -> f32 {
    if match_in.is_empty() {
        return 1.0;
    }
    if query.is_empty() {
        return 0.8;
    }

    let query_lower = query.to_lowercase();
    let mut best_score: f32 = 1.0;

    for element in match_in.split(';') {
        if element.is_empty() {
            continue;
        }

        // perfect match
        if element == query {
            return 0.0;
        }

        let element_lower = element.to_lowercase();

        // case-insensitive perfect match
        if element_lower == query_lower {
            return 0.01;
        }

        // prefix match
        if element_lower.starts_with(&query_lower) {
            let coverage = query.len() as f32 / element.len() as f32;
            let score = 0.1 + (0.1 * (1.0 - coverage));
            best_score = best_score.min(score);
            continue;
        }

        // substring match (with position + coverage penalty)
        if let Some(pos) = element_lower.find(&query_lower) {
            let coverage = query.len() as f32 / element.len() as f32;
            let position_penalty = pos as f32 / element.len() as f32 * 0.1;
            let score = 0.25 + (0.1 * (1.0 - coverage)) + position_penalty;
            best_score = best_score.min(score);
            continue;
        }

        // levenshtein — window scales with query length
        let max_dist = (query.len() / 4 + 1).min(4);
        if (element.len() as isize - query.len() as isize).abs() < max_dist as isize {
            let dist = levenshtein::levenshtein(&query_lower, &element_lower);
            let normed = (dist as f32 / element.len() as f32).clamp(0.35, 1.0);
            best_score = best_score.min(normed);
        }
    }

    best_score
}

static DEBUG_SEARCH: LazyLock<bool> =
    LazyLock::new(|| std::env::var("DEBUG_SEARCH").map_or(false, |v| v == "true"));

pub fn make_prio(prio: f32, query: &str, match_in: &str) -> f32 {
    let score = search_score(query, match_in);

    // prio coming in: {base_int}.{exec_frac}
    // e.g. 10.80 → base=10, exec digits=80
    let base = prio.trunc();
    let exec_part = (prio.fract() * 100.0).round() as u32;

    // Encode: .SSEE
    let score_part = (score * 99.0).round().clamp(0.0, 99.0) as u32;
    let exec_clamped = exec_part.min(99);
    let frac = (score_part * 100 + exec_clamped) as f32 / 10_000.0;

    let result = base + frac;

    if cfg!(debug_assertions) && *DEBUG_SEARCH {
        let m = match_in.chars().take(30).collect::<String>();
        let q = query.chars().take(20).collect::<String>();
        println!(
            "[search] {:<30} | query: {:<20} | score: {:.3} ({:02}) | exec: {:02} | prio: {:.4} → {:.4}",
            m, q, score, score_part, exec_clamped, prio, result
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_relevance_ranking() {
        let query = "calc";

        // Lower score is better in search_score logic
        let perfect = search_score(query, "calc");
        let case_insensitive = search_score(query, "CALC");
        let prefix = search_score(query, "calculator");
        let substring = search_score(query, "my_calc_app");
        let fuzzy = search_score(query, "clac"); // Levenshtein 1
        let no_match = search_score(query, "firefox");

        assert!(
            perfect < case_insensitive,
            "Perfect should beat case-insensitive"
        );
        assert!(
            case_insensitive < prefix,
            "Case-insensitive should beat prefix"
        );
        assert!(prefix < substring, "Prefix should beat substring");
        assert!(substring < fuzzy, "Substring should beat fuzzy/Levenshtein");
        assert!(fuzzy < no_match, "Fuzzy match should beat no match");
    }

    #[test]
    fn test_make_prio_base_preservation() {
        let query = "term";
        let match_in = "terminal";

        // Base priority 10.0 and 5.0 should never swap, regardless of search match
        let high_prio = make_prio(10.50, query, match_in);
        let low_prio = make_prio(5.99, query, match_in);

        assert!(high_prio > 10.0 && high_prio < 11.0);
        assert!(low_prio > 5.0 && low_prio < 6.0);
        assert!(
            high_prio > low_prio,
            "Base priority must be the dominant sorting factor"
        );
    }

    #[test]
    fn test_make_prio_encoding_logic() {
        let query = "abc";
        let match_in = "abc"; // Perfect match, score = 0.0 -> score_part = 0
        let base_prio = 1.85; // Base = 1, Exec = 85

        let result = make_prio(base_prio, query, match_in);

        // Expected: 1.0085
        // (Score 00, Exec 85 -> .0085)
        let fractional_part = result.fract();

        // Use a small epsilon for float comparison
        assert!(
            (fractional_part - 0.0085).abs() < 0.0001,
            "Fractional part should be .SSEE encoded"
        );
    }

    #[test]
    fn test_exec_count_tiebreaker() {
        let query = "git";
        let match_a = "github";
        let match_b = "gitlab";

        // Both match_in strings will produce the exact same search_score.
        // The result should be decided by the input execution priority.
        let prio_a = make_prio(1.90, query, match_a); // More used
        let prio_b = make_prio(1.10, query, match_b); // Less used

        assert!(
            prio_a > prio_b,
            "Execution count should act as a tie-breaker within the same base"
        );
    }

    #[test]
    fn test_semicolon_alias_support() {
        let query = "code";
        let score = search_score(query, "Visual Studio;code;editor");
        assert_eq!(score, 0.00);
    }

    #[test]
    fn test_levenshtein_scaling() {
        // Query "vlc" (length 3). max_dist = (3/4 + 1) = 1.
        // "vlc" vs "vlb" is distance 1. Should match.
        let score_match = search_score("vlc", "vlb");
        assert!(score_match < 1.0);

        // "vlc" vs "vxxxx" is length diff 2. Should be ignored by Levenshtein.
        let score_no_match = search_score("vlc", "vxxxx");
        assert_eq!(score_no_match, 1.0);
    }
}
