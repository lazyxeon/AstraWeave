use crate::IVec2;

/// Largest byte index `end` such that `end <= max_bytes` and `s.is_char_boundary(end)`.
pub fn floor_char_boundary(s: &str, max_bytes: usize) -> usize {
    let mut end = max_bytes.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// Prefix of `s` with byte length at most `max_bytes`, ending on a char boundary.
pub fn byte_prefix(s: &str, max_bytes: usize) -> &str {
    &s[..floor_char_boundary(s, max_bytes)]
}

/// Smallest byte index `start` such that `start >= min_bytes` and `s.is_char_boundary(start)`.
pub fn ceil_char_boundary(s: &str, min_bytes: usize) -> usize {
    let mut start = min_bytes.min(s.len());
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    start
}

/// Suffix of `s` with byte length at most `max_bytes`, starting on a char boundary.
pub fn byte_suffix(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    &s[ceil_char_boundary(s, s.len() - max_bytes)..]
}

pub fn manhattan(a: IVec2, b: IVec2) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manhattan_same_position() {
        let a = IVec2 { x: 5, y: 10 };
        let b = IVec2 { x: 5, y: 10 };
        assert_eq!(manhattan(a, b), 0);
    }

    #[test]
    fn test_manhattan_horizontal() {
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 5, y: 0 };
        assert_eq!(manhattan(a, b), 5);
    }

    #[test]
    fn test_manhattan_vertical() {
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 0, y: 7 };
        assert_eq!(manhattan(a, b), 7);
    }

    #[test]
    fn test_manhattan_diagonal() {
        let a = IVec2 { x: 0, y: 0 };
        let b = IVec2 { x: 3, y: 4 };
        assert_eq!(manhattan(a, b), 7); // 3 + 4 = 7
    }

    #[test]
    fn test_manhattan_negative_coords() {
        let a = IVec2 { x: -5, y: -10 };
        let b = IVec2 { x: 5, y: 10 };
        assert_eq!(manhattan(a, b), 30); // |5-(-5)| + |10-(-10)| = 10 + 20 = 30
    }

    #[test]
    fn test_manhattan_mixed_coords() {
        let a = IVec2 { x: -3, y: 5 };
        let b = IVec2 { x: 2, y: -1 };
        assert_eq!(manhattan(a, b), 11); // |2-(-3)| + |-1-5| = 5 + 6 = 11
    }

    #[test]
    fn test_manhattan_symmetric() {
        let a = IVec2 { x: 10, y: 20 };
        let b = IVec2 { x: 5, y: 15 };
        assert_eq!(manhattan(a, b), manhattan(b, a)); // Distance should be symmetric
    }

    #[test]
    fn floor_char_boundary_ascii_unchanged() {
        let s = "hello world";
        assert_eq!(floor_char_boundary(s, 5), 5);
        assert_eq!(byte_prefix(s, 5), "hello");
    }

    #[test]
    fn floor_char_boundary_mid_multibyte() {
        let s = "éX";
        assert_eq!(floor_char_boundary(s, 1), 0);
        assert_eq!(byte_prefix(s, 1), "");
    }

    #[test]
    fn floor_char_boundary_cjk() {
        let s = "日本語";
        assert_eq!(floor_char_boundary(s, 4), 3);
        assert_eq!(byte_prefix(s, 4), "日");
    }

    #[test]
    fn floor_char_boundary_emoji() {
        let s = "a🎮b";
        let emoji_start = "a".len() + 1;
        assert_eq!(floor_char_boundary(s, emoji_start), 1);
    }

    #[test]
    fn floor_char_boundary_combining_mark() {
        let s = "e\u{0301}X";
        let mid = "e".len() + 1;
        assert!(mid < s.len());
        assert_eq!(floor_char_boundary(s, mid), 1);
    }

    #[test]
    fn byte_suffix_ascii_one_byte_scalar() {
        let s = format!("{}world", "a".repeat(10));
        assert_eq!(byte_suffix(&s, 5), "world");
    }

    #[test]
    fn byte_suffix_two_byte_scalar_at_boundary() {
        let unit = "é";
        let head = "x".repeat(198);
        let s = format!("{head}{unit}{unit}");
        assert_eq!(byte_suffix(&s, 2), unit);
        let raw_start = s.len().saturating_sub(1);
        assert!(!s.is_char_boundary(raw_start));
        let suffix = byte_suffix(&s, 1);
        assert!(s.is_char_boundary(s.len() - suffix.len()));
    }

    #[test]
    fn byte_suffix_three_byte_scalar_at_boundary() {
        let unit = "日";
        let head = "x".repeat(199);
        let s = format!("{head}{unit}");
        assert_eq!(byte_suffix(&s, 3), unit);
    }

    #[test]
    fn byte_suffix_four_byte_scalar_at_boundary() {
        let unit = "🎮";
        let head = "x".repeat(198);
        let s = format!("{head}{unit}");
        assert_eq!(byte_suffix(&s, 4), unit);
    }

    #[test]
    fn byte_suffix_nihon_repeat_matches_ollama_tail_budget() {
        let s = "日".repeat(200);
        assert_eq!(s.len(), 600);
        let raw_start = s.len().saturating_sub(200);
        assert!(!s.is_char_boundary(raw_start));
        let suffix = byte_suffix(&s, 200);
        assert!(suffix.len() <= 200);
        assert!(s.is_char_boundary(s.len() - suffix.len()));
    }

    #[test]
    fn byte_suffix_ascii_tail() {
        let s = "hello world";
        assert_eq!(byte_suffix(s, 5), "world");
    }

    #[test]
    fn byte_suffix_multibyte_start_not_mid_scalar() {
        let head = "x".repeat(300);
        let tail_unit = "🎮";
        let rest = "y".repeat(197);
        let s = format!("{head}{tail_unit}{rest}");
        assert!(s.len() > 500);
        let raw_start = s.len().saturating_sub(200);
        assert!(!s.is_char_boundary(raw_start));
        let suffix = byte_suffix(&s, 200);
        assert!(suffix.len() <= 200);
        assert!(s.is_char_boundary(s.len() - suffix.len()));
    }
}
