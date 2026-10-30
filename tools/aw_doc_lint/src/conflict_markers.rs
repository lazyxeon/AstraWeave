//! Detect leftover git merge/rebase conflict markers in prose files.
//!
//! Always enforced (even in `--mode warn`). Markers are reported in prose and
//! inside fenced code blocks. The only suppression is an intentional
//! documentation example: a fenced block whose opening info string is exactly
//! `diff`, `text`, or `console` (the whole trimmed string after the opening fence
//! run, with no extra tokens) and that contains a complete conflict sequence
//! (`<<<<<<<` … `=======` … `>>>>>>>`) before the fence closes, and the fence
//! ends with no unclosed conflict still open. Only the first balanced example in
//! such a fence is suppressed when the fence **closes** with a valid closing line;
//! an unclosed fence that runs to EOF never suppresses markers. Any later marker
//! in that fence is reported.
//!
//! Fence lines follow CommonMark: up to three spaces of indent, an opening run
//! of three or more `` ` `` or `~` characters (optional info string), and a
//! closing run of the same character with length at least the opener and only
//! whitespace after it.

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

#[derive(Clone, Debug)]
struct PendingFenceErrors {
    exempt_candidate: bool,
    saw_opener: bool,
    saw_separator: bool,
    saw_closer: bool,
    in_conflict: bool,
    /// Index in `errors` where the first exempt example's `<<<<<<<` will be recorded.
    exempt_example_start: Option<usize>,
    /// Inclusive start, exclusive end of the first balanced exempt example in `errors`.
    exempt_quiet_range: Option<(usize, usize)>,
    errors: Vec<String>,
}

/// Report `path:line: …` messages for conflict markers in tracked text.
pub fn conflict_marker_errors(path: &str, content: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut fence_stack: Vec<(OpenFence, PendingFenceErrors)> = Vec::new();
    let mut in_conflict = false;

    for (line_num, raw) in content.lines().enumerate() {
        let line = raw.trim_end();
        let parsed_fence = parse_code_fence_line(line);
        let opens_backtick_with_backtick_info = parsed_fence.is_some_and(|parsed| {
            fence_stack.is_empty()
                && parsed.marker == '`'
                && backtick_fence_info_has_backtick(line, parsed)
        });
        if let Some(parsed) = parsed_fence.filter(|_| !opens_backtick_with_backtick_info) {
            let closes_top = fence_stack.last().is_some_and(|(open, _)| {
                parsed.marker == open.marker
                    && parsed.len >= open.len
                    && parsed.only_whitespace_after
            });
            if closes_top {
                let (_, block) = fence_stack.pop().expect("closes_top implied a frame");
                in_conflict = block.in_conflict;
                flush_fence_block(&mut errors, block, true);
            } else if fence_stack.is_empty() && parsed.len >= 3 {
                in_conflict = false;
                let info = fence_info_string(line, parsed);
                fence_stack.push((
                    OpenFence {
                        marker: parsed.marker,
                        len: parsed.len,
                    },
                    PendingFenceErrors {
                        exempt_candidate: is_exempt_fence_info(info.as_deref()),
                        saw_opener: false,
                        saw_separator: false,
                        saw_closer: false,
                        in_conflict: false,
                        exempt_example_start: None,
                        exempt_quiet_range: None,
                        errors: Vec::new(),
                    },
                ));
            } else if let Some((_, block)) = fence_stack.last_mut() {
                apply_line_markers(path, line_num + 1, line, block);
            }
            continue;
        }

        if let Some((_, block)) = fence_stack.last_mut() {
            apply_line_markers(path, line_num + 1, line, block);
        } else {
            push_line_markers(path, line_num + 1, line, &mut in_conflict, &mut errors);
        }
    }

    while let Some((_, block)) = fence_stack.pop() {
        flush_fence_block(&mut errors, block, false);
    }

    errors
}

fn flush_fence_block(errors: &mut Vec<String>, block: PendingFenceErrors, fence_closed: bool) {
    if fence_closed {
        if let Some((start, end)) = block.exempt_quiet_range {
            errors.extend(block.errors[..start].iter().cloned());
            errors.extend(block.errors[end..].iter().cloned());
        } else {
            errors.extend(block.errors);
        }
    } else {
        // CommonMark: an unclosed fence runs to EOF; do not treat EOF as a clean close
        // for documentation-example suppression.
        errors.extend(block.errors);
    }
}

fn is_exempt_fence_info(info: Option<&str>) -> bool {
    matches!(info, Some("diff") | Some("text") | Some("console"))
}

fn fence_info_string(line: &str, parsed: ParsedFence) -> Option<String> {
    if parsed.only_whitespace_after {
        return None;
    }
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
        idx += 1;
    }
    let after = &line[idx + parsed.len..];
    let trimmed = after.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let info = trimmed.trim_end();
    if info.is_empty() {
        return None;
    }
    Some(info.to_string())
}

fn backtick_fence_info_has_backtick(line: &str, parsed: ParsedFence) -> bool {
    if parsed.marker != '`' {
        return false;
    }
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
        idx += 1;
    }
    line[idx + parsed.len..].trim_start().contains('`')
}

fn maybe_note_exempt_quiet_range(block: &mut PendingFenceErrors) {
    if block.exempt_candidate
        && block.saw_opener
        && block.saw_separator
        && block.saw_closer
        && !block.in_conflict
        && block.exempt_quiet_range.is_none()
    {
        if let Some(start) = block.exempt_example_start {
            block.exempt_quiet_range = Some((start, block.errors.len()));
            block.exempt_example_start = None;
            // Redundant with invalidate paths that clear saw_*; kept so stale flags cannot
            // widen a quiet range after the first balanced example is recorded.
            block.saw_opener = false;
            block.saw_separator = false;
            block.saw_closer = false;
        }
    }
}

fn invalidate_exempt_example_tracking(block: &mut PendingFenceErrors) {
    block.exempt_example_start = None;
    block.saw_opener = false;
    block.saw_separator = false;
    block.saw_closer = false;
}

fn apply_line_markers(path: &str, line_num: usize, line: &str, block: &mut PendingFenceErrors) {
    if block.exempt_candidate
        && block.in_conflict
        && block.exempt_quiet_range.is_none()
        && starts_conflict_marker(line, "<<<<<<<")
    {
        // Redundant once exempt_candidate is false; kept to clear a pending start index immediately.
        invalidate_exempt_example_tracking(block);
        block.exempt_candidate = false;
    }
    if block.exempt_candidate
        && !block.in_conflict
        && block.exempt_quiet_range.is_none()
        && block.exempt_example_start.is_none()
        && starts_conflict_marker(line, "<<<<<<<")
    {
        block.exempt_example_start = Some(block.errors.len());
    }
    let mut scratch = Vec::new();
    push_line_markers(path, line_num, line, &mut block.in_conflict, &mut scratch);
    for msg in &scratch {
        if msg.contains("<<<<<<<") {
            block.saw_opener = true;
            block.in_conflict = true;
        } else if msg.contains("=======") {
            block.saw_separator = true;
        } else if msg.contains(">>>>>>>") {
            block.saw_closer = true;
            block.in_conflict = false;
            if block.exempt_candidate && block.saw_opener && !block.saw_separator {
                invalidate_exempt_example_tracking(block);
            }
        } else if msg.contains("|||||||") {
            block.saw_opener = true;
        }
    }
    block.errors.extend(scratch);
    maybe_note_exempt_quiet_range(block);
}

fn push_line_markers(
    path: &str,
    line_num: usize,
    line: &str,
    in_conflict: &mut bool,
    errors: &mut Vec<String>,
) {
    if starts_conflict_marker(line, "<<<<<<<") {
        errors.push(format!("{path}:{line_num}: git conflict marker `<<<<<<<`"));
        *in_conflict = true;
        return;
    }
    if starts_conflict_marker(line, ">>>>>>>") {
        errors.push(format!("{path}:{line_num}: git conflict marker `>>>>>>>`"));
        *in_conflict = false;
        return;
    }
    if starts_conflict_marker(line, "|||||||") {
        errors.push(format!("{path}:{line_num}: git conflict marker `|||||||`"));
        return;
    }
    if *in_conflict && line == "=======" {
        errors.push(format!("{path}:{line_num}: git conflict marker `=======`"));
    }
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

    fn opener() -> &'static str {
        "<<<<<<< HEAD"
    }

    fn separator() -> &'static str {
        "======="
    }

    fn closer(branch: &str) -> String {
        format!(">>>>>>> {branch}")
    }

    #[test]
    fn real_conflict_block_is_flagged() {
        let body = format!(
            "before\n{}\noirs\n{}\ntheirs\n{}\nafter\n",
            opener(),
            separator(),
            closer("b3209cd25")
        );
        let errors = conflict_marker_errors("doc.md", &body);
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
        let body = format!("Title\n{}\nBody\n", separator());
        let errors = conflict_marker_errors("doc.md", &body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn intentional_diff_fence_example_is_not_flagged() {
        let body = format!(
            "```diff\n{}\noirs\n{}\ntheirs\n{}\n```\n",
            opener(),
            separator(),
            closer("branch")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn conflict_inside_non_exempt_fence_is_flagged() {
        let body = format!(
            "Intro\n```rust\nlet a = 1;\n{}\nlet b = 2;\n{}\nlet b = 3;\n{}\n```\nafter\n",
            opener(),
            separator(),
            closer("feature")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    #[test]
    fn conflict_splitting_fence_is_flagged_in_prose_and_fence() {
        let body = format!(
            "```rust\nold\n{}\n```\n{}\nnew\n```\n{}\nprose\n{}\nx\n{}\ny\n{}\n",
            opener(),
            separator(),
            closer("b"),
            opener(),
            separator(),
            closer("c")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 6, "{errors:?}");
        assert!(errors
            .iter()
            .any(|e| e.contains(":5:") && e.contains("=======")));
        assert!(errors
            .iter()
            .any(|e| e.contains(":10:") && e.contains("<<<<<<<")));
    }

    #[test]
    fn unclosed_fence_before_prose_conflict_is_flagged() {
        let body = format!(
            "```\nexample never closed\n\n## Later section\n{}\nx\n{}\ny\n{}\n",
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    fn error_lines(errors: &[String]) -> Vec<usize> {
        errors
            .iter()
            .filter_map(|e| e.split(':').nth(1).and_then(|n| n.parse().ok()))
            .collect()
    }

    #[test]
    fn closing_fence_requires_matching_marker_char() {
        let body = format!(
            "```text\n{}\n~~~\n{}\n{}\n```\n{}\n{}\n{}\n",
            opener(),
            separator(),
            closer("branch"),
            opener(),
            separator(),
            closer("tail")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [7, 8, 9], "{errors:?}");
    }

    #[test]
    fn closing_fence_requires_minimum_length() {
        let body = format!(
            "````text\n{}\n```\n{}\n{}\n````\n{}\n{}\n{}\n",
            opener(),
            separator(),
            closer("branch"),
            opener(),
            separator(),
            closer("tail")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [7, 8, 9], "{errors:?}");
    }

    #[test]
    fn closing_fence_requires_no_info_string() {
        let body = format!(
            "```text\n{}\n{}\n``` rust\n{}\n```\n{}\n{}\n{}\n",
            opener(),
            separator(),
            closer("branch"),
            opener(),
            separator(),
            closer("tail")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [7, 8, 9], "{errors:?}");
    }

    #[test]
    fn markers_inside_tilde_fence_with_non_exempt_info_are_flagged() {
        let body = format!(
            "~~~ruby\n{}\n{}\n{}\n~~~\n",
            opener(),
            separator(),
            closer("branch")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    #[test]
    fn outer_four_tick_fence_does_not_close_on_inner_three_ticks() {
        let body = "````markdown\n``` sample\nstill inside outer\n```\n````\n";
        let errors = conflict_marker_errors("doc.md", body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn extra_info_string_tokens_are_not_exempt() {
        let body = format!(
            "```diff title=x\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("branch")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    #[test]
    fn intentional_text_fence_example_is_not_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("branch")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn stray_marker_after_exempt_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("b"),
            opener()
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert!(!errors.is_empty(), "{errors:?}");
        assert!(errors
            .iter()
            .any(|e| e.contains(":5:") && e.contains("<<<<<<<")));
    }

    #[test]
    fn stray_marker_before_exempt_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n```\n",
            closer("x"),
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2], "{errors:?}");
    }

    #[test]
    fn stray_markers_before_and_after_exempt_example_are_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n{}\n{}\n```\n",
            closer("x"),
            opener(),
            separator(),
            closer("b"),
            opener(),
            closer("y")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 6, 7], "{errors:?}");
    }

    #[test]
    fn stray_closer_after_exempt_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("b"),
            closer("stray")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [5], "{errors:?}");
    }

    #[test]
    fn stray_diff3_parent_after_exempt_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n||||||| parent\n```\n",
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [5], "{errors:?}");
    }

    #[test]
    fn unclosed_opener_before_balanced_example_is_flagged_when_second_opener_arrives() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 3, 4, 5], "{errors:?}");
    }

    #[test]
    fn orphan_closer_after_exempt_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("b"),
            separator(),
            closer("c")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [6], "{errors:?}");
    }

    #[test]
    fn second_conflict_after_exempt_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("b"),
            opener(),
            separator(),
            closer("c")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [5, 6, 7], "{errors:?}");
    }

    #[test]
    fn setext_heading_inside_markdown_fence_is_not_flagged() {
        let body = "```markdown\nTitle\n=======\n```\n";
        let errors = conflict_marker_errors("doc.md", body);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn backtick_in_backtick_fence_info_is_not_exempt() {
        let body = format!(
            "```text look `here`\n{}\n{}\n{}\n```\n",
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    #[test]
    fn unterminated_open_marker_is_flagged() {
        let body = format!("ok\n{}\nstill open\n", opener());
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains(":2:"));
        assert!(errors[0].contains("<<<<<<<"));
    }

    #[test]
    fn partial_opener_closer_without_separator_then_balanced_example_is_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            closer("x"),
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 3], "{errors:?}");
    }

    #[test]
    fn diff3_without_separator_then_balanced_example_is_flagged() {
        let body = format!(
            "```text\n{}\n||||||| parent\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            closer("x"),
            opener(),
            separator(),
            closer("b")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 3, 4], "{errors:?}");
    }

    #[test]
    fn second_conflict_after_misaligned_openers_is_fully_flagged() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n\n{}\n{}\n{}\n```\n",
            opener(),
            opener(),
            separator(),
            closer("b"),
            opener(),
            separator(),
            closer("c")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 3, 4, 5, 7, 8, 9], "{errors:?}");
    }

    #[test]
    fn nested_opener_clears_exempt_candidate() {
        let body = format!(
            "```text\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n```\n",
            opener(),
            opener(),
            separator(),
            closer("b"),
            opener(),
            separator(),
            closer("c")
        );
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 3, 4, 5, 6, 7, 8], "{errors:?}");
    }

    #[test]
    fn balanced_example_in_unclosed_exempt_fence_is_flagged_at_eof() {
        let body = format!("```text\n{}\n{}\n{}\n", opener(), separator(), closer("b"));
        let errors = conflict_marker_errors("doc.md", &body);
        assert_eq!(error_lines(&errors), [2, 3, 4], "{errors:?}");
    }

    #[test]
    fn parse_fence_accepts_three_space_indent() {
        let parsed = parse_code_fence_line("   ``` rust").unwrap();
        assert_eq!(parsed.marker, '`');
        assert_eq!(parsed.len, 3);
        assert!(!parsed.only_whitespace_after);
    }
}
