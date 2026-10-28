//! Visible text from a CommonMark/GFM parse.
//!
//! `pulldown-cmark` owns the markdown structure. This walk keeps `Text` and
//! `Code` as that parser emitted them, so a comment inside a code span or a
//! fence stays, and a character reference pulldown already decoded is not
//! decoded again. HTML events are the only place a comment is removed, and
//! the only place a character reference is decoded. The reference must end
//! in `;`. `<!-->` and `<!--->` are empty closed comments, and an unclosed
//! `<!--` consumes the rest of that HTML chunk. A block comment's chunk runs
//! through EOF, including a blank line. A `br` tag becomes U+2028, which
//! splits the word around it and still leaves the surrounding sentence on
//! one `str::lines` line. Link labels are the link's own text. Reference
//! definitions are not events. Footnotes are on: a referenced definition is
//! included, and an unreferenced one is omitted.

use std::collections::HashSet;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

const LINE_BREAK: char = '\u{2028}';

pub(crate) fn render_visible(input: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let events: Vec<Event> = Parser::new_ext(input, options).collect();
    let mut referenced = HashSet::new();
    for event in &events {
        if let Event::FootnoteReference(label) = event {
            referenced.insert(label.to_string());
        }
    }
    let mut out = String::new();
    let mut html_block: Option<String> = None;
    let mut skip_footnote = 0u32;
    for event in events {
        if skip_footnote > 0 {
            match &event {
                Event::Start(Tag::FootnoteDefinition(_)) => skip_footnote += 1,
                Event::End(TagEnd::FootnoteDefinition) => skip_footnote -= 1,
                _ => {}
            }
            continue;
        }
        if let Event::Start(Tag::FootnoteDefinition(label)) = &event {
            if !referenced.contains(label.as_ref()) {
                skip_footnote = 1;
                continue;
            }
        }
        if let Some(buf) = html_block.as_mut() {
            match event {
                Event::Html(html) => buf.push_str(&html),
                Event::End(TagEnd::HtmlBlock) => {
                    let chunk = html_block.take().unwrap_or_default();
                    push_html_visible(&mut out, &chunk);
                }
                other => {
                    let chunk = html_block.take().unwrap_or_default();
                    push_html_visible(&mut out, &chunk);
                    push_event(&mut out, &mut html_block, other);
                }
            }
            continue;
        }
        push_event(&mut out, &mut html_block, event);
    }
    if let Some(chunk) = html_block {
        push_html_visible(&mut out, &chunk);
    }
    out
}

fn push_event(out: &mut String, html_block: &mut Option<String>, event: Event<'_>) {
    match event {
        Event::Start(Tag::HtmlBlock) => {
            out.push('\n');
            *html_block = Some(String::new());
        }
        Event::Start(tag) if block_start(&tag) => out.push('\n'),
        Event::Start(_) | Event::End(_) => {}
        Event::Text(text)
        | Event::Code(text)
        | Event::InlineMath(text)
        | Event::DisplayMath(text) => {
            out.push_str(&text);
        }
        Event::Html(html) | Event::InlineHtml(html) => push_html_visible(out, &html),
        Event::SoftBreak | Event::HardBreak => out.push('\n'),
        Event::Rule => out.push('\n'),
        Event::TaskListMarker(_) | Event::FootnoteReference(_) => {}
    }
}

fn block_start(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::MetadataBlock(_)
    )
}

fn push_html_visible(out: &mut String, html: &str) {
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes.get(i..i + 4) == Some(b"<!--") {
            i = comment_end(bytes, i);
            continue;
        }
        if bytes[i] == b'&' {
            if let Some((text, next)) = decode_char_ref(bytes, i) {
                out.push_str(&text);
                i = next;
                continue;
            }
        }
        if bytes[i] == b'<' && tag_starter(bytes, i) {
            if let Some(end) = tag_end(bytes, i) {
                let tag = &html[i..end];
                if is_br_tag(tag) {
                    out.push(LINE_BREAK);
                } else {
                    push_alts(out, tag);
                }
                i = end;
                continue;
            }
            out.push('<');
            i += 1;
            continue;
        }
        let Some(ch) = html[i..].chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
}

/// Index just past a comment. An unclosed comment ends at the chunk's end.
fn comment_end(bytes: &[u8], i: usize) -> usize {
    if bytes.get(i + 4) == Some(&b'>') {
        return i + 5;
    }
    if bytes.get(i + 4..i + 6) == Some(b"->") {
        return i + 6;
    }
    let mut j = i + 4;
    while j + 2 < bytes.len() {
        if bytes[j] == b'-' && bytes[j + 1] == b'-' && bytes[j + 2] == b'>' {
            return j + 3;
        }
        j += 1;
    }
    bytes.len()
}

fn tag_starter(bytes: &[u8], i: usize) -> bool {
    matches!(
        bytes.get(i + 1),
        Some(b'A'..=b'Z' | b'a'..=b'z' | b'/' | b'!')
    )
}

fn tag_end(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    let mut quote: Option<u8> = None;
    while j < bytes.len() {
        let b = bytes[j];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
            j += 1;
            continue;
        }
        if b == b'"' || b == b'\'' {
            quote = Some(b);
            j += 1;
            continue;
        }
        if b == b'>' {
            return Some(j + 1);
        }
        j += 1;
    }
    None
}

fn is_br_tag(tag: &str) -> bool {
    let bytes = tag.as_bytes();
    let mut i = 1;
    if bytes.get(i) == Some(&b'/') {
        i += 1;
    }
    if bytes
        .get(i..i + 2)
        .is_none_or(|name| !name.eq_ignore_ascii_case(b"br"))
    {
        return false;
    }
    !bytes
        .get(i + 2)
        .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':'))
}

fn push_alts(out: &mut String, tag: &str) {
    let bytes = tag.as_bytes();
    if bytes.get(1) == Some(&b'/') {
        return;
    }
    let mut j = 1;
    while j < bytes.len() {
        if let Some((start, end)) = alt_at(bytes, j) {
            if start < end {
                out.push(' ');
                push_decoded_text(out, &tag[start..end]);
                out.push(' ');
            }
            j = end.max(j + 1);
            continue;
        }
        j += 1;
    }
}

fn alt_at(bytes: &[u8], j: usize) -> Option<(usize, usize)> {
    if !is_attr_name(bytes, j, b"alt") {
        return None;
    }
    let mut p = j + 3;
    while p < bytes.len() && bytes[p].is_ascii_whitespace() {
        p += 1;
    }
    if bytes.get(p) != Some(&b'=') {
        return None;
    }
    p += 1;
    while p < bytes.len() && bytes[p].is_ascii_whitespace() {
        p += 1;
    }
    let quote = bytes.get(p).copied()?;
    if quote == b'"' || quote == b'\'' {
        let start = p + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != quote {
            end += 1;
        }
        return Some((start, end));
    }
    let start = p;
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_whitespace() || b == b'>' {
            break;
        }
        if b == b'/' && bytes.get(end + 1) == Some(&b'>') {
            break;
        }
        end += 1;
    }
    if end == start {
        return None;
    }
    Some((start, end))
}

fn is_attr_name(bytes: &[u8], j: usize, name: &[u8]) -> bool {
    if j == 0 || !bytes[j - 1].is_ascii_whitespace() {
        return false;
    }
    let end = j + name.len();
    let Some(got) = bytes.get(j..end) else {
        return false;
    };
    if !got.eq_ignore_ascii_case(name) {
        return false;
    }
    match bytes.get(end) {
        None => true,
        Some(b) => !b.is_ascii_alphanumeric() && !matches!(*b, b'_' | b'-' | b':'),
    }
}

fn push_decoded_text(out: &mut String, raw: &str) {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some((text, next)) = decode_char_ref(bytes, i) {
                out.push_str(&text);
                i = next;
                continue;
            }
        }
        let Some(ch) = raw[i..].chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
}

/// A numeric or HTML5 named character reference that ends in `;`.
/// Leading zeros do not count toward the six hex or seven decimal digits.
/// A name without `;`, including `&nbsp`, is not a reference.
fn decode_char_ref(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    if bytes.get(i) != Some(&b'&') {
        return None;
    }
    let rest = &bytes[i + 1..];
    if rest.first() == Some(&b'#') {
        let hex = matches!(rest.get(1), Some(b'x' | b'X'));
        let digits_at = if hex { 2 } else { 1 };
        let base: u32 = if hex { 16 } else { 10 };
        let max_significant = if hex { 6 } else { 7 };
        let mut n: u32 = 0;
        let mut seen = 0usize;
        let mut significant = 0usize;
        for &b in rest.get(digits_at..)? {
            let dig = if hex {
                match b {
                    b'0'..=b'9' => b - b'0',
                    b'a'..=b'f' => b - b'a' + 10,
                    b'A'..=b'F' => b - b'A' + 10,
                    _ => break,
                }
            } else if b.is_ascii_digit() {
                b - b'0'
            } else {
                break;
            };
            seen += 1;
            if n == 0 && dig == 0 {
                continue;
            }
            significant += 1;
            if significant > max_significant {
                return None;
            }
            n = n.checked_mul(base)?.checked_add(u32::from(dig))?;
        }
        if seen == 0 {
            return None;
        }
        let end = i + 1 + digits_at + seen;
        if bytes.get(end) != Some(&b';') {
            return None;
        }
        Some((char::from_u32(n)?.to_string(), end + 1))
    } else {
        if !rest.first().is_some_and(|b| b.is_ascii_alphabetic()) {
            return None;
        }
        let mut len = 1usize;
        while rest.get(len).is_some_and(|b| b.is_ascii_alphanumeric()) {
            len += 1;
            if len > 64 {
                return None;
            }
        }
        if rest.get(len) != Some(&b';') {
            return None;
        }
        let name = std::str::from_utf8(&rest[..len]).ok()?.to_ascii_lowercase();
        let expanded = crate::html_entities::html_entity(&name)?;
        Some((expanded.to_string(), i + 1 + len + 1))
    }
}
