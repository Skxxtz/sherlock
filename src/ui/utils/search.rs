pub trait SherlockSearch {
    /// Both self and substring should already be lowercased to increase performance
    fn fuzzy_match<'a>(&'a self, substring: &'a str) -> bool;
}

impl<T: AsRef<str>> SherlockSearch for T {
    fn fuzzy_match(&self, pattern: &str) -> bool {
        let text = self.as_ref();

        // empty pattern always matches
        if pattern.is_empty() {
            return true;
        }

        let t_bytes = text.as_bytes();
        let p_bytes = pattern.as_bytes();

        // pattern longer than text can never match
        if p_bytes.len() > t_bytes.len() {
            return false;
        }

        // exact contains check for single-segment patterns
        if !pattern.contains(';') {
            return fuzzy_match_single(t_bytes, p_bytes);
        }

        // match against any semicolon-separated segment
        for segment in text.split(';') {
            let seg_bytes = segment.as_bytes();
            if seg_bytes.len() >= p_bytes.len() && fuzzy_match_single(seg_bytes, p_bytes) {
                return true;
            }
        }
        false
    }
}

#[inline(always)]
fn fuzzy_match_single(text: &[u8], pattern: &[u8]) -> bool {
    let p_len = pattern.len();
    let t_len = text.len();

    // early return
    if t_len < p_len {
        return false;
    }

    // single char match for speed
    if p_len == 1 {
        return text.iter().any(|&b| b.eq_ignore_ascii_case(&pattern[0]));
    }

    // prefix match (fast path)
    // case insensitive prefix match
    if text.len() >= p_len {
        let prefix_match = text[..p_len]
            .iter()
            .zip(pattern.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));
        if prefix_match {
            return true;
        }
    }

    // early return if last char does not occur in haystack[pattern.len() - q..]
    let last_p = pattern[p_len - 1];
    let has_last = text[p_len - 1..]
        .iter()
        .any(|&b| b.eq_ignore_ascii_case(&last_p));
    if !has_last {
        return false;
    }

    let mut p_idx = 0;
    let mut t_idx = 0;

    while t_idx < t_len && p_idx < p_len {
        let remaining_text = t_len - t_idx;
        let remaining_pattern = p_len - p_idx;

        if remaining_text < remaining_pattern {
            return false;
        }

        if text[t_idx].eq_ignore_ascii_case(&pattern[p_idx]) {
            p_idx += 1;
            if p_idx == p_len {
                return true;
            }
        }
        t_idx += 1;
    }

    p_idx == p_len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_patterns() {
        // Empty pattern should always match
        assert!("anything".fuzzy_match(""));
        assert!("".fuzzy_match(""));
    }

    #[test]
    fn test_length_pruning() {
        // Pattern longer than text should never match
        assert!(!"abc".fuzzy_match("abcd"));
    }

    #[test]
    fn test_prefix_fast_path() {
        // Should hit the prefix logic
        assert!("Google Chrome".fuzzy_match("goo"));
        assert!("Firefox".fuzzy_match("Fire"));
    }

    #[test]
    fn test_case_insensitivity() {
        assert!("SHERLOCK".fuzzy_match("sher"));
        assert!("sherlock".fuzzy_match("SHER"));
    }

    #[test]
    fn test_semicolon_segments() {
        let text = "Terminal;alacritty;iterm2";
        // Match against different segments
        assert!(text.fuzzy_match("term")); // Matches "Terminal"
        assert!(text.fuzzy_match("ala")); // Matches "alacritty"
        assert!(text.fuzzy_match("iterm")); // Matches "iterm2"
        assert!(!text.fuzzy_match("ghostty")); // No match
    }

    #[test]
    fn test_fuzzy_subsequence() {
        // Standard fuzzy matching (letters in order but separated)
        assert!("vlc media player".fuzzy_match("vmp")); // v...m...p
        assert!("visual studio code".fuzzy_match("vsc"));
        assert!(!"visual studio code".fuzzy_match("vcs")); // Wrong order
    }

    #[test]
    fn test_single_char_fast_path() {
        assert!("Brave".fuzzy_match("b"));
        assert!("Brave".fuzzy_match("v"));
        assert!(!"Brave".fuzzy_match("z"));
    }

    #[test]
    fn test_last_char_heuristic() {
        // This pattern contains 'z'. The text "Chromium" does not.
        // Should be caught by the `has_last` check.
        assert!(!"Chromium".fuzzy_match("chz"));
    }

    #[test]
    fn test_remaining_text_pruning() {
        // "abc" has 3 chars. Pattern "abcd" is too long.
        // This tests the `remaining_text < remaining_pattern` logic.
        assert!(!"abc".fuzzy_match("abcd"));
    }
}
