//! Visible text from a CommonMark/GFM parse.
//!
//! `pulldown-cmark` owns the markdown structure. `Code` stays as that parser
//! emitted it, so a comment or a character reference inside a code span or a
//! fence stays. A fenced or indented code block is `Text` inside `CodeBlock`,
//! and those characters stay too. Other `Text` decodes a numeric character
//! reference that ends in `;`, including any run of leading zeros, and
//! leaves a named reference as pulldown emitted it.
//! A named reference pulldown already decoded is not decoded again. HTML
//! events are the only place a comment is removed. There a numeric reference
//! follows the HTML tokenizer: the semicolon is optional, 0 is U+FFFD, and
//! 0x80–0x9F use the Windows-1252 row. A semicolon name is decoded, and so
//! is an HTML5 legacy name with no semicolon. Alt text uses that numeric
//! rule and that legacy set, except a legacy name is not consumed when the
//! next byte is `=` or an ASCII alphanumeric. `<!-->` and `<!--->` are empty closed comments, and an
//! unclosed `<!--` consumes the rest of that HTML chunk. A block comment's
//! chunk runs through EOF, including a blank line. A `br` tag becomes
//! U+2028, which splits the word around it and still leaves the surrounding
//! sentence on one `str::lines` line. Link labels are the link's own text.
//! Reference definitions are not events. Footnotes are on: a referenced
//! definition is included, an unreferenced one is omitted, and a defined
//! reference writes its digits as superscripts (or U+00B9 when the label has
//! no digit) so the letters around it do not join.

use std::collections::HashSet;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const LINE_BREAK: char = '\u{2028}';

pub(crate) fn render_visible(input: &str) -> String {
    drive(input, false).body
}

/// Visible text from the rendered h2 whose folded text is exactly
/// `in progress` through the next rendered h2. Setext and ATX both count.
/// A heading that only contains those words does not open the section. A
/// `## ` line inside a fence or a comment is not a heading.
pub(crate) fn in_progress_section(input: &str) -> Option<String> {
    let walked = drive(input, true);
    walked.found.then_some(walked.body)
}

fn drive(input: &str, track: bool) -> Walk {
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
    let mut walk = Walk {
        referenced,
        discard: String::new(),
        heading: String::new(),
        body: String::new(),
        sink: if track { Sink::Discard } else { Sink::Body },
        html_block: None,
        code_depth: 0,
        skip_footnote: 0,
        found: false,
        done: false,
        track,
    };
    for event in events {
        walk.feed(event);
    }
    walk.finish();
    walk
}

enum Sink {
    Discard,
    Heading,
    Body,
}

struct Walk {
    referenced: HashSet<String>,
    discard: String,
    heading: String,
    body: String,
    sink: Sink,
    html_block: Option<String>,
    code_depth: u32,
    skip_footnote: u32,
    found: bool,
    done: bool,
    track: bool,
}

impl Walk {
    fn buf(&mut self) -> &mut String {
        if !self.track {
            return &mut self.body;
        }
        match self.sink {
            Sink::Discard => &mut self.discard,
            Sink::Heading => &mut self.heading,
            Sink::Body => &mut self.body,
        }
    }

    fn feed(&mut self, event: Event<'_>) {
        if self.done {
            return;
        }
        if self.skip_footnote > 0 {
            match &event {
                Event::Start(Tag::FootnoteDefinition(_)) => self.skip_footnote += 1,
                Event::End(TagEnd::FootnoteDefinition) => self.skip_footnote -= 1,
                _ => {}
            }
            return;
        }
        if let Event::Start(Tag::FootnoteDefinition(label)) = &event {
            if !self.referenced.contains(label.as_ref()) {
                self.skip_footnote = 1;
                return;
            }
        }
        let ended_h2 = matches!(event, Event::End(TagEnd::Heading(HeadingLevel::H2)));
        if self.track {
            if let Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) = &event
            {
                if self.found {
                    self.done = true;
                    return;
                }
                self.heading.clear();
                self.sink = Sink::Heading;
            }
        }
        if self.html_block.is_some() {
            match event {
                Event::Html(html) => {
                    self.html_block.as_mut().unwrap().push_str(&html);
                }
                Event::End(TagEnd::HtmlBlock) => {
                    let chunk = self.html_block.take().unwrap_or_default();
                    push_html_visible(self.buf(), &chunk);
                }
                other => {
                    let chunk = self.html_block.take().unwrap_or_default();
                    push_html_visible(self.buf(), &chunk);
                    self.push_event(other);
                }
            }
        } else {
            self.push_event(event);
        }
        if self.track && ended_h2 && matches!(self.sink, Sink::Heading) {
            let folded = crate::fold_visible_text(&self.heading);
            if folded.trim().eq_ignore_ascii_case("in progress") {
                self.body.push_str(&self.heading);
                self.sink = Sink::Body;
                self.found = true;
            } else {
                self.heading.clear();
                self.sink = Sink::Discard;
            }
        }
    }

    fn finish(&mut self) {
        if self.done {
            return;
        }
        if let Some(chunk) = self.html_block.take() {
            push_html_visible(self.buf(), &chunk);
        }
    }

    fn push_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::HtmlBlock) => {
                self.buf().push('\n');
                self.html_block = Some(String::new());
            }
            Event::Start(Tag::CodeBlock(_)) => {
                self.code_depth += 1;
                self.buf().push('\n');
            }
            Event::End(TagEnd::CodeBlock) => {
                self.code_depth = self.code_depth.saturating_sub(1);
            }
            Event::Start(tag) if block_start(&tag) => self.buf().push('\n'),
            Event::Start(_) | Event::End(_) => {}
            Event::Text(text) => {
                if self.code_depth > 0 {
                    self.buf().push_str(&text);
                } else {
                    push_text_numeric(self.buf(), &text);
                }
            }
            Event::Code(text) | Event::InlineMath(text) | Event::DisplayMath(text) => {
                self.buf().push_str(&text);
            }
            Event::Html(html) | Event::InlineHtml(html) => push_html_visible(self.buf(), &html),
            Event::SoftBreak | Event::HardBreak => self.buf().push('\n'),
            Event::Rule => self.buf().push('\n'),
            Event::TaskListMarker(_) => {}
            Event::FootnoteReference(label) => push_footnote_mark(self.buf(), &label),
        }
    }
}

fn push_text_numeric(out: &mut String, raw: &str) {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some((text, next)) = decode_numeric_ref(bytes, i) {
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

fn push_footnote_mark(out: &mut String, label: &str) {
    let mut wrote = false;
    for ch in label.chars() {
        if let Some(sup) = superscript_digit(ch) {
            out.push(sup);
            wrote = true;
        }
    }
    if !wrote {
        out.push('\u{00B9}');
    }
}

fn superscript_digit(ch: char) -> Option<char> {
    Some(match ch {
        '0' => '\u{2070}',
        '1' => '\u{00B9}',
        '2' => '\u{00B2}',
        '3' => '\u{00B3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        _ => return None,
    })
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
            if let Some(next) = push_char_ref(out, bytes, i, false) {
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
            if let Some(next) = push_char_ref(out, bytes, i, true) {
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

/// An HTML numeric reference (semicolon optional), a semicolon name, or an
/// HTML5 legacy name with no semicolon. In an attribute, a legacy name is
/// not consumed when the next byte is `=` or an ASCII alphanumeric.
fn push_char_ref(out: &mut String, bytes: &[u8], i: usize, in_attribute: bool) -> Option<usize> {
    if bytes.get(i + 1) == Some(&b'#') {
        let (text, next) = decode_html_numeric_ref(bytes, i)?;
        out.push_str(&text);
        return Some(next);
    }
    if let Some((text, next)) = decode_char_ref(bytes, i) {
        out.push_str(&text);
        return Some(next);
    }
    let (text, len) = crate::html_legacy::legacy_match(bytes.get(i + 1..)?)?;
    let next = bytes.get(i + 1 + len);
    if in_attribute && next.is_some_and(|b| *b == b'=' || b.is_ascii_alphanumeric()) {
        return None;
    }
    out.push_str(text);
    Some(i + 1 + len)
}

/// An HTML5 named character reference that ends in `;`.
fn decode_char_ref(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    if bytes.get(i) != Some(&b'&') {
        return None;
    }
    let rest = &bytes[i + 1..];
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

/// HTML numeric character reference. The semicolon is optional. Zero and a
/// value that is not a scalar become U+FFFD. 0x80–0x9F use the Windows-1252
/// row from the HTML numeric end state.
fn decode_html_numeric_ref(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    if bytes.get(i) != Some(&b'&') || bytes.get(i + 1) != Some(&b'#') {
        return None;
    }
    let rest = &bytes[i + 1..];
    let hex = matches!(rest.get(1), Some(b'x' | b'X'));
    let digits_at = if hex { 2 } else { 1 };
    let base: u32 = if hex { 16 } else { 10 };
    let mut n: u32 = 0;
    let mut seen = 0usize;
    let mut overflow = false;
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
        if overflow {
            continue;
        }
        match n
            .checked_mul(base)
            .and_then(|value| value.checked_add(u32::from(dig)))
        {
            Some(next) if next <= 0x10_FFFF => n = next,
            _ => overflow = true,
        }
    }
    if seen == 0 {
        return None;
    }
    let mut end = i + 1 + digits_at + seen;
    if bytes.get(end) == Some(&b';') {
        end += 1;
    }
    let ch = if overflow {
        '\u{FFFD}'
    } else {
        html_numeric_char(n)
    };
    Some((ch.to_string(), end))
}

fn html_numeric_char(n: u32) -> char {
    const WINDOWS_1252: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];
    if n == 0 || (0xD800..=0xDFFF).contains(&n) {
        return '\u{FFFD}';
    }
    if (0x80..=0x9F).contains(&n) {
        return WINDOWS_1252[(n - 0x80) as usize];
    }
    char::from_u32(n).unwrap_or('\u{FFFD}')
}

/// Markdown `Text`. A semicolon is required. Leading zeros do not count
/// toward the six hex or seven decimal digits.
fn decode_numeric_ref(bytes: &[u8], i: usize) -> Option<(String, usize)> {
    if bytes.get(i) != Some(&b'&') || bytes.get(i + 1) != Some(&b'#') {
        return None;
    }
    let rest = &bytes[i + 1..];
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
}
