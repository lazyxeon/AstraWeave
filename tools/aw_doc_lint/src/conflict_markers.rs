//! Detect leftover git merge/rebase conflict markers in prose files.
//!
//! Always enforced (even in `--mode warn`). Fenced code blocks are skipped so
//! documented marker examples do not fail the gate.

/// Report `path:line: …` messages for conflict markers outside fenced code blocks.
pub fn conflict_marker_errors(path: &str, content: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut in_fence = false;
    let mut in_conflict = false;

    for (line_num, raw) in content.lines().enumerate() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
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
    fn markers_inside_fence_are_not_flagged() {
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
    fn unterminated_open_marker_is_flagged() {
        let body = "ok\n<<<<<<< HEAD\nstill open\n";
        let errors = conflict_marker_errors("doc.md", body);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains(":2:"));
        assert!(errors[0].contains("<<<<<<<"));
    }
}
