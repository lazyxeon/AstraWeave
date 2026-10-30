//! Detect leftover git merge/rebase conflict markers in prose files.
//!
//! Always enforced (even in `--mode warn`). Fenced code blocks are skipped so
//! documented marker examples do not fail the gate. Fence lines follow
//! CommonMark: up to three spaces of indent, an opening run of three or more
//! `` ` `` or `~` characters (optional info string), and a closing run of the
//! same character with length at least the opener and only whitespace after it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OpenFence {
    marker: char,
    len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedFence {
    marker: char,
    len: usize,
    only_whitespace_after: bool,
}

/// Report `path:line: …` messages for conflict markers outside fenced code blocks.
pub fn conflict_marker_errors(path: &str, content: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut open_fence: Option<OpenFence> = None;
    let mut in_conflict = false;

    for (line_num, raw) in content.lines().enumerate() {
        let line = raw.trim_end();
        if let Some(parsed) = parse_code_fence_line(line) {
            if let Some(open) = open_fence {
                if parsed.marker == open.marker
                    && parsed.len >= open.len
                    && parsed.only_whitespace_after
                {
                    open_fence = None;
                }
            } else if parsed.len >= 3 {
                open_fence = Some(OpenFence {
                    marker: parsed.marker,
                    len: parsed.len,
                });
            }
            continue;
        }
        if open_fence.is_some() {
            continue;
        }

        if starts_conflict_marker(line, "<<<<<<<") {
            errors.push(format!(
                "{path}:{}: git conflict marker `<<<<<<<`",
                line_num + 1
            ));
            in_conflict = true;
            continue;
        }
        if starts_conflict_marker(line, ">>>>>>>") {
            errors.push(format!(
                "{path}:{}: git conflict marker `>>>>>>>`",
                line_num + 1
            ));
            in_conflict = false;
            continue;
        }
        if starts_conflict_marker(line, "|||||||") {
            errors.push(format!(
                "{path}:{}: git conflict marker `|||||||`",
                line_num + 1
            ));
            continue;
        }
        if in_conflict && line == "=======" {
            errors.push(format!(
                "{path}:{}: git conflict marker `=======`",
                line_num + 1
            ));
        }
    }

    errors
}

/// A fenced code block line: up to three leading spaces, then a run of `` ` `` or `~`.
fn parse_code_fence_line(line: &str) -> Option<ParsedFence> {
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
        idx += 1;
    }
    if idx >= bytes.len() {
        return None;
    }
    let marker = line[idx..].chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let mut len = 0usize;
    for ch in line[idx..].chars() {
        if ch == marker {
            len += 1;
        } else {
            break;
        }
    }
    if len < 3 {
        return None;
    }
    let after = &line[idx + len..];
    let only_whitespace_after = after.chars().all(|ch| ch.is_whitespace());
    Some(ParsedFence {
        marker,
        len,
        only_whitespace_after,
    })
}

fn starts_conflict_marker(line: &str, prefix: &str) -> bool {
    if !line.starts_with(prefix) {
        return false;
    }
    match line.as_bytes().get(prefix.len()) {
        None => true,
        Some(b' ') => true,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_conflict_block_is_flagged() {
        let body = "\
before
<<<<<<< HEAD
ours
=======
theirs
>>>>>>> b3209cd25
after
";
        let errors = conflict_marker_errors("doc.md", body);
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert!(errors
            .iter()
            .any(|e| e.contains(":2:") && e.contains("<<<<<<<")));
        assert!(errors
            .iter()
            .any(|e| e.contains(":4:") && e.contains("=======")));
        assert!(errors
            .iter()
            .any(|e| e.contains(":6:") && e.contains(">>>>>>>")));
    }

    #[test]
    fn setext_heading_equals_is_not_flagged() {
        let body = "Title\n=======\nBody\n";
        let errors = conflict_marker_errors("doc.md", body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn markers_inside_backtick_fence_with_info_string_are_not_flagged() {
        let body = "\
```text
<<<<<<< HEAD
=======
>>>>>>> branch
```
";
        let errors = conflict_marker_errors("doc.md", body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn markers_inside_tilde_fence_are_not_flagged() {
        let body = "\
~~~ruby
<<<<<<< HEAD
=======
>>>>>>> branch
~~~
";
        let errors = conflict_marker_errors("doc.md", body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn outer_four_tick_fence_does_not_close_on_inner_three_ticks() {
        let body = "\
````markdown
```text
<<<<<<< HEAD
=======
>>>>>>> branch
```
````
";
        let errors = conflict_marker_errors("doc.md", body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn unterminated_open_marker_is_flagged() {
        let body = "ok\n<<<<<<< HEAD\nstill open\n";
        let errors = conflict_marker_errors("doc.md", body);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains(":2:"));
        assert!(errors[0].contains("<<<<<<<"));
    }

    #[test]
    fn parse_fence_accepts_three_space_indent() {
        let parsed = parse_code_fence_line("   ``` rust").unwrap();
        assert_eq!(parsed.marker, '`');
        assert_eq!(parsed.len, 3);
        assert!(!parsed.only_whitespace_after);
    }
}
