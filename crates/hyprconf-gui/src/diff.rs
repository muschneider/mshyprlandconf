//! A small LCS-based line diff for the before/after save preview.
//!
//! Config files are short, so the `O(n·m)` table is fine and keeps the crate
//! dependency-free.

/// The role of a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    /// Unchanged line (context).
    Equal,
    /// Line present only in the new text.
    Insert,
    /// Line present only in the old text.
    Delete,
}

/// A single diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Whether the line was kept, added or removed.
    pub tag: Tag,
    /// The line text (without a trailing newline).
    pub text: String,
}

/// Compute a line diff between `before` and `after`.
#[must_use]
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    let (n, m) = (a.len(), b.len());

    // dp[i][j] = LCS length of a[i..] and b[j..].
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(DiffLine {
                tag: Tag::Equal,
                text: a[i].to_string(),
            });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(DiffLine {
                tag: Tag::Delete,
                text: a[i].to_string(),
            });
            i += 1;
        } else {
            out.push(DiffLine {
                tag: Tag::Insert,
                text: b[j].to_string(),
            });
            j += 1;
        }
    }
    while i < n {
        out.push(DiffLine {
            tag: Tag::Delete,
            text: a[i].to_string(),
        });
        i += 1;
    }
    while j < m {
        out.push(DiffLine {
            tag: Tag::Insert,
            text: b[j].to_string(),
        });
        j += 1;
    }
    out
}

/// `(added, removed)` line counts.
#[must_use]
pub fn summary(diff: &[DiffLine]) -> (usize, usize) {
    let added = diff.iter().filter(|d| d.tag == Tag::Insert).count();
    let removed = diff.iter().filter(|d| d.tag == Tag::Delete).count();
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs_a_single_changed_line() {
        let before = "a\nb\nc\n";
        let after = "a\nB\nc\n";
        let diff = diff_lines(before, after);
        assert_eq!(summary(&diff), (1, 1));
        // The changed line shows as one delete + one insert, surrounded by equals.
        assert!(diff.iter().any(|d| d.tag == Tag::Delete && d.text == "b"));
        assert!(diff.iter().any(|d| d.tag == Tag::Insert && d.text == "B"));
        assert_eq!(diff.iter().filter(|d| d.tag == Tag::Equal).count(), 2);
    }

    #[test]
    fn identical_text_has_no_changes() {
        let diff = diff_lines("x\ny\n", "x\ny\n");
        assert_eq!(summary(&diff), (0, 0));
    }

    #[test]
    fn pure_additions() {
        let diff = diff_lines("a\n", "a\nb\nc\n");
        assert_eq!(summary(&diff), (2, 0));
    }
}
