//! A tiny, dependency-free fuzzy matcher for the search box.
//!
//! Matching is a case-insensitive subsequence test with bonuses for matches at
//! the start of the haystack and for runs of contiguous characters, which is
//! enough to rank option label/path/description hits sensibly.

/// Score `query` against `haystack`. Returns `None` if `query` is not a
/// subsequence of `haystack`; a higher score is a better match.
#[must_use]
pub fn score(query: &str, haystack: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query = query.to_ascii_lowercase();
    let haystack = haystack.to_ascii_lowercase();
    let needle = query.as_bytes();

    let mut ni = 0usize;
    let mut total = 0i32;
    let mut previous_match: Option<usize> = None;

    for (hi, &hc) in haystack.as_bytes().iter().enumerate() {
        if ni < needle.len() && hc == needle[ni] {
            total += 1;
            if hi == 0 {
                total += 8;
            }
            if let Some(prev) = previous_match {
                if hi == prev + 1 {
                    total += 4;
                }
            }
            previous_match = Some(hi);
            ni += 1;
        }
    }

    (ni == needle.len()).then_some(total)
}

/// Best score of `query` across an option's fields, with field weighting
/// (label > path > description). `None` if it matches none of them.
#[must_use]
pub fn option_score(query: &str, label: &str, path: &str, description: &str) -> Option<i32> {
    let candidates = [
        score(query, label).map(|s| s + 20),
        score(query, path).map(|s| s + 10),
        score(query, description),
    ];
    candidates.into_iter().flatten().max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subsequence_matches() {
        assert!(score("gap", "gaps_in").is_some());
        assert!(score("gpsn", "gaps_in").is_some()); // non-contiguous subsequence
        assert!(score("xyz", "gaps_in").is_none());
    }

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(score("", "anything"), Some(0));
    }

    #[test]
    fn prefix_and_contiguous_score_higher() {
        let prefix = score("gap", "gaps_in").unwrap();
        let middle = score("gap", "x_gap").unwrap();
        assert!(
            prefix > middle,
            "prefix {prefix} should beat middle {middle}"
        );
    }

    #[test]
    fn label_is_weighted_above_description() {
        let via_label =
            option_score("round", "Rounding", "decoration:rounding", "corners").unwrap();
        let via_desc =
            option_score("corner", "Rounding", "decoration:rounding", "corners").unwrap();
        assert!(via_label > via_desc);
    }
}
