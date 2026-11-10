//! Fail-closed allowlist of the DOM and global calls the map script may use.
//!
//! A member call, a member assignment, or a `new` whose name is not on the
//! list is a sink. A computed key used as a call, or a computed key that
//! contains a string and is assigned, is a sink. `setAttribute` names and
//! string timers are checked after escapes and `'a'+'b'` folding, so a split
//! or hex-escaped name cannot hide.

use std::collections::BTreeSet;

/// Sorted. Member names the page calls (`.name(` / `['name'](`).
const MEMBER_CALL: &[&str] = &[
    "$",
    "add",
    "addClass",
    "addEventListener",
    "animate",
    "append",
    "appendChild",
    "call",
    "charAt",
    "classList",
    "clear",
    "click",
    "contains",
    "cos",
    "createElement",
    "createTextNode",
    "customViz",
    "data",
    "delete",
    "dijkstra",
    "edges",
    "elements",
    "endsWith",
    "entries",
    "error",
    "exec",
    "filter",
    "find",
    "fit",
    "flatMap",
    "forEach",
    "from",
    "fromEntries",
    "get",
    "getAttribute",
    "getBoundingClientRect",
    "getElementById",
    "has",
    "hasClass",
    "hasOwnProperty",
    "id",
    "includes",
    "indexOf",
    "isArray",
    "isEdge",
    "isFinite",
    "isNode",
    "isParent",
    "join",
    "keys",
    "layout",
    "log",
    "map",
    "match",
    "max",
    "min",
    "move",
    "nodes",
    "on",
    "one",
    "parse",
    "pathTo",
    "pop",
    "position",
    "push",
    "querySelector",
    "querySelectorAll",
    "remove",
    "removeClass",
    "replace",
    "replaceChildren",
    "replaceState",
    "resize",
    "run",
    "select",
    "selector",
    "set",
    "setAttribute",
    "sin",
    "slice",
    "some",
    "sort",
    "split",
    "startsWith",
    "stop",
    "style",
    "test",
    "then",
    "toLocaleString",
    "toLowerCase",
    "toUpperCase",
    "toggle",
    "trim",
    "unselect",
    "update",
    "warn",
    "writeText",
];

/// Sorted. Members the page assigns (`node.name =`).
const MEMBER_ASSIGN: &[&str] = &[
    "_timer",
    "background",
    "blastEnabled",
    "checked",
    "className",
    "color",
    "currentTour",
    "disabled",
    "display",
    "focusModeActive",
    "fontFamily",
    "fontSize",
    "fontStyle",
    "fontWeight",
    "hash",
    "id",
    "left",
    "letterSpacing",
    "lineHeight",
    "margin",
    "marginBottom",
    "marginTop",
    "maxHeight",
    "opacity",
    "overflowY",
    "padding",
    "selectedNodeId",
    "stepIndex",
    "text",
    "textAlign",
    "textContent",
    "textTransform",
    "title",
    "top",
    "value",
];

/// Sorted. Bare calls (`name(`) the page makes, plus `setTimeout`.
const BARE_CALL: &[&str] = &[
    "Number",
    "String",
    "alert",
    "appendKids",
    "appendMarkup",
    "applyBlastHighlight",
    "applyFilters",
    "applyFocusMode",
    "applyStory",
    "attachCompounds",
    "buildEdgeGradient",
    "cb",
    "centerOnNode",
    "clearHighlights",
    "clearTimeout",
    "computeBlastRadius",
    "cytoscape",
    "decodeHash",
    "decodeURIComponent",
    "detachCompounds",
    "detailsBlock",
    "dotSpan",
    "edgeLine",
    "el",
    "emptyPanel",
    "encodeState",
    "encodeURIComponent",
    "expandPanelOnMobile",
    "filterToDomains",
    "fontForBand",
    "isDomainFilterActive",
    "isNaN",
    "kindMark",
    "locText",
    "parseFloat",
    "parseInt",
    "pickInitialLayout",
    "renderEdgePanel",
    "renderNodePanel",
    "renderTourStep",
    "replaceContent",
    "requestAnimationFrame",
    "restoreFromHash",
    "restoreOnLoad",
    "rewriteStaticTraceLinks",
    "riskPill",
    "runDormancyStratification",
    "runInitialLayout",
    "runLayout",
    "safeColor",
    "setChipsEnabled",
    "setPickMode",
    "setSelectedNode",
    "setTimeout",
    "showToast",
    "sizeForBand",
    "startTour",
    "statusLabelFor",
    "statusPill",
    "textOf",
    "toggleBlast",
    "toggleFocus",
    "tourExit",
    "tourNext",
    "tourPrev",
    "updateFilterHint",
    "updateHash",
    "updatePathBtn",
    "waitForContainerSize",
];

/// Sorted. `new Name`.
const NEW_OK: &[&str] = &["Map", "Set", "URLSearchParams"];

/// Sorted. Static `setAttribute` names the page writes.
const ATTR_OK: &[&str] = &[
    "aria-expanded",
    "aria-pressed",
    "data-domain",
    "href",
    "rel",
    "target",
];

const KEYWORDS: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "export",
    "extends",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "let",
    "new",
    "of",
    "return",
    "static",
    "super",
    "switch",
    "throw",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

pub fn html_parsing_sink(src: &str) -> Option<String> {
    let problems = html_parsing_problems(src);
    problems.into_iter().next()
}

pub fn html_parsing_problems(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for body in js_sources(src) {
        scan_js(&body, &mut out);
    }
    out
}

fn js_sources(src: &str) -> Vec<String> {
    if !src.contains("<script") {
        return vec![src.to_string()];
    }
    let mut out = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let Some(rel) = src[i..].find("<script") else {
            break;
        };
        let start = i + rel;
        let Some(gt) = src[start..].find('>') else {
            break;
        };
        let open = &src[start..start + gt + 1];
        let body_at = start + gt + 1;
        let Some(end_rel) = src[body_at..].find("</script>") else {
            break;
        };
        let body = &src[body_at..body_at + end_rel];
        let skip = open.to_ascii_lowercase().contains("application/json");
        if !skip && !body.trim().is_empty() {
            out.push(body.to_string());
        }
        i = body_at + end_rel + "</script>".len();
    }
    out
}

fn allowed(list: &[&str], name: &str) -> bool {
    list.binary_search(&name).is_ok()
}

fn keyword(name: &str) -> bool {
    KEYWORDS.binary_search(&name).is_ok()
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    /// A template that interpolates. Not a static key.
    Interp,
    Num,
    Dot,
    LBrack,
    RBrack,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Plus,
    Assign,
    Other,
}

fn scan_js(src: &str, out: &mut BTreeSet<String>) {
    let tokens = lex(src);
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Tok::Dot => {
                if let Some(Tok::Ident(name)) = tokens.get(i + 1) {
                    let name = name.clone();
                    let after = i + 2;
                    if matches!(tokens.get(after), Some(Tok::LParen)) {
                        note_member_call(&name, &tokens, after, out);
                    } else if matches!(tokens.get(after), Some(Tok::Assign))
                        && !allowed(MEMBER_ASSIGN, &name)
                    {
                        out.insert(format!("member assign {name}"));
                    }
                    i = after;
                    continue;
                }
            }
            Tok::LBrack if prev_is_value(&tokens, i) => {
                if let Some((key, end)) = bracket_key(&tokens, i) {
                    let after = end + 1;
                    let call = matches!(tokens.get(after), Some(Tok::LParen));
                    let assign = matches!(tokens.get(after), Some(Tok::Assign));
                    match key {
                        Key::Static(name) if call => note_member_call(&name, &tokens, after, out),
                        Key::Static(name) if assign => {
                            if !allowed(MEMBER_ASSIGN, &name) {
                                out.insert(format!("member assign {name}"));
                            }
                        }
                        Key::Num if call => {
                            out.insert("computed call".into());
                        }
                        Key::Computed { has_string: true } if assign || call => {
                            out.insert(if call {
                                "computed call".into()
                            } else {
                                "computed assign".into()
                            });
                        }
                        Key::Computed { has_string: false } if call => {
                            out.insert("computed call".into());
                        }
                        _ => {}
                    }
                    i = after;
                    continue;
                }
            }
            Tok::Ident(name) if name == "new" => {
                if let Some(Tok::Ident(ctor)) = tokens.get(i + 1) {
                    if !keyword(ctor) && !allowed(NEW_OK, ctor) {
                        out.insert(format!("new {ctor}"));
                    }
                }
            }
            Tok::Ident(name) if matches!(tokens.get(i + 1), Some(Tok::LParen)) => {
                let defined = i > 0 && matches!(&tokens[i - 1], Tok::Ident(p) if keyword(p));
                if !defined && !keyword(name) {
                    note_bare_call(name, &tokens, i + 1, out);
                }
            }
            Tok::LParen if i > 0 && matches!(tokens.get(i - 1), Some(Tok::RParen)) => {
                note_grouped_call(&tokens, i - 1, i, out);
            }
            _ => {}
        }
        i += 1;
    }
}

/// `(0, eval)(x)` is not a name. `(eval)(x)` and `(document.write)(x)` are.
fn note_grouped_call(tokens: &[Tok], close: usize, call: usize, out: &mut BTreeSet<String>) {
    let Some(open) = matching_open(tokens, close) else {
        out.insert("indirect call".into());
        return;
    };
    if comma_at_root(tokens, open, close) {
        out.insert("indirect call".into());
        return;
    }
    let interior = &tokens[open + 1..close];
    if let [Tok::Ident(name)] = interior {
        note_bare_call(name, tokens, call, out);
        return;
    }
    if let [Tok::Ident(_), Tok::Dot, Tok::Ident(name)] | [Tok::Dot, Tok::Ident(name)] = interior {
        note_member_call(name, tokens, call, out);
    }
}

fn matching_open(tokens: &[Tok], close: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut j = close;
    loop {
        match &tokens[j] {
            Tok::RParen => depth += 1,
            Tok::LParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
        if j == 0 {
            return None;
        }
        j -= 1;
    }
}

fn comma_at_root(tokens: &[Tok], open: usize, close: usize) -> bool {
    let mut paren = 0i32;
    let mut brace = 0i32;
    let mut brack = 0i32;
    for tok in &tokens[open + 1..close] {
        match tok {
            Tok::LParen => paren += 1,
            Tok::RParen => paren -= 1,
            Tok::LBrace => brace += 1,
            Tok::RBrace => brace -= 1,
            Tok::LBrack => brack += 1,
            Tok::RBrack => brack -= 1,
            Tok::Comma if paren == 0 && brace == 0 && brack == 0 => return true,
            _ => {}
        }
    }
    false
}

fn note_member_call(name: &str, tokens: &[Tok], paren: usize, out: &mut BTreeSet<String>) {
    if name == "setAttribute" || name == "setAttributeNS" {
        match first_arg(tokens, paren) {
            Arg::Static(attr) => {
                if !allowed(ATTR_OK, &attr) || attr.to_ascii_lowercase().starts_with("on") {
                    out.insert(format!("attribute {attr}"));
                }
            }
            Arg::Other => {
                out.insert("computed attribute".into());
            }
            Arg::Missing => {
                out.insert("computed attribute".into());
            }
        }
        return;
    }
    if !allowed(MEMBER_CALL, name) {
        out.insert(format!("member call {name}"));
        return;
    }
    if name == "setTimeout" || name == "setInterval" {
        note_timer(tokens, paren, out);
    }
}

fn note_bare_call(name: &str, tokens: &[Tok], paren: usize, out: &mut BTreeSet<String>) {
    if !allowed(BARE_CALL, name) {
        out.insert(format!("bare {name}"));
        return;
    }
    if name == "setTimeout" || name == "setInterval" {
        note_timer(tokens, paren, out);
    }
}

fn note_timer(tokens: &[Tok], paren: usize, out: &mut BTreeSet<String>) {
    match first_arg(tokens, paren) {
        Arg::Static(_) => {
            out.insert("string timer".into());
        }
        Arg::Other => {
            // An identifier or a function is the page's form. A leading
            // string was already classified as Static. Anything else that
            // starts with a string-like interp is Other only when mixed;
            // a pure string is Static. Reject a first token that is Interp.
            if matches!(tokens.get(paren + 1), Some(Tok::Interp)) {
                out.insert("string timer".into());
            }
        }
        Arg::Missing => {}
    }
}

enum Key {
    Static(String),
    Num,
    Computed { has_string: bool },
}

fn bracket_key(tokens: &[Tok], open: usize) -> Option<(Key, usize)> {
    let mut depth = 0;
    let mut parts: Vec<&Tok> = Vec::new();
    let mut end = None;
    for (j, tok) in tokens.iter().enumerate().skip(open) {
        match tok {
            Tok::LBrack => depth += 1,
            Tok::RBrack => {
                depth -= 1;
                if depth == 0 {
                    end = Some(j);
                    break;
                }
            }
            _ if depth == 1 => parts.push(tok),
            _ => {}
        }
    }
    let end = end?;
    let key = classify_key(&parts);
    Some((key, end))
}

fn classify_key(parts: &[&Tok]) -> Key {
    if parts.len() == 1 {
        return match parts[0] {
            Tok::Str(s) => Key::Static(s.clone()),
            Tok::Num => Key::Num,
            Tok::Interp => Key::Computed { has_string: true },
            _ => Key::Computed { has_string: false },
        };
    }
    let mut folded = String::new();
    let mut expect_str = true;
    let mut strings = 0;
    for tok in parts {
        match (expect_str, tok) {
            (true, Tok::Str(s)) => {
                folded.push_str(s);
                strings += 1;
                expect_str = false;
            }
            (false, Tok::Plus) => expect_str = true,
            _ => {
                let has_string = parts.iter().any(|t| matches!(t, Tok::Str(_) | Tok::Interp));
                return Key::Computed { has_string };
            }
        }
    }
    if strings >= 1 && !expect_str {
        Key::Static(folded)
    } else {
        Key::Computed {
            has_string: strings > 0,
        }
    }
}

enum Arg {
    Static(String),
    Other,
    Missing,
}

fn first_arg(tokens: &[Tok], paren: usize) -> Arg {
    let mut depth = 0;
    let mut parts: Vec<&Tok> = Vec::new();
    let mut started = false;
    for tok in tokens.iter().skip(paren) {
        match tok {
            Tok::LParen => {
                if started && depth == 0 {
                    break;
                }
                depth += 1;
                started = true;
                if depth > 1 {
                    return Arg::Other;
                }
            }
            Tok::RParen => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Tok::Comma if depth == 1 => break,
            _ if depth == 1 => parts.push(tok),
            _ => {}
        }
    }
    if parts.is_empty() {
        return Arg::Missing;
    }
    match classify_key(&parts) {
        Key::Static(s) => Arg::Static(s),
        _ => Arg::Other,
    }
}

fn prev_is_value(tokens: &[Tok], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    matches!(
        &tokens[i - 1],
        Tok::Ident(_) | Tok::RParen | Tok::RBrack | Tok::Str(_) | Tok::Num | Tok::Interp
    )
}

fn lex(src: &str) -> Vec<Tok> {
    let mut extra = Vec::new();
    let mut out = lex_into(src, &mut extra);
    for chunk in extra {
        out.extend(lex(&chunk));
    }
    out
}

fn lex_into(src: &str, extra: &mut Vec<String>) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            i += 2;
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
            continue;
        }
        if c == b'/' && regex_here(&out) {
            i = skip_regex(src, i + 1);
            out.push(Tok::Other);
            continue;
        }
        if c == b'\'' || c == b'"' {
            let (value, next) = read_string(src, i);
            out.push(Tok::Str(value));
            i = next;
            continue;
        }
        if c == b'`' {
            let (tok, next) = read_template(src, i, extra);
            out.push(tok);
            i = next;
            continue;
        }
        if c == b'.'
            && b.get(i + 1).is_some_and(u8::is_ascii_digit)
            && !prev_is_value(&out, out.len())
        {
            let next = skip_number(src, i);
            out.push(Tok::Num);
            i = next;
            continue;
        }
        if c == b'?' && b.get(i + 1) == Some(&b'.') {
            out.push(Tok::Dot);
            i += 2;
            continue;
        }
        if is_ident_start(src, i) {
            let (name, next) = read_ident(src, i);
            out.push(Tok::Ident(name));
            i = next;
            continue;
        }
        if c.is_ascii_digit() {
            i = skip_number(src, i);
            out.push(Tok::Num);
            continue;
        }
        let (tok, n) = punct(b, i);
        out.push(tok);
        i += n;
    }
    out
}

fn regex_here(out: &[Tok]) -> bool {
    let Some(prev) = out.last() else {
        return true;
    };
    match prev {
        Tok::Ident(name) if keyword(name) => true,
        Tok::Ident(_) | Tok::Num | Tok::Str(_) | Tok::RParen | Tok::RBrack | Tok::Interp => false,
        _ => true,
    }
}

fn skip_regex(src: &str, mut i: usize) -> usize {
    let b = src.as_bytes();
    let mut class = false;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'[' => {
                class = true;
                i += 1;
            }
            b']' => {
                class = false;
                i += 1;
            }
            b'/' if !class => {
                i += 1;
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i += 1;
                }
                return i;
            }
            b'\n' => return i,
            _ => i += 1,
        }
    }
    i
}

fn read_string(src: &str, start: usize) -> (String, usize) {
    let b = src.as_bytes();
    let quote = b[start];
    let mut i = start + 1;
    let mut value = String::new();
    while i < b.len() {
        if b[i] == b'\\' {
            let (ch, next) = decode_escape(src, i);
            value.push(ch);
            i = next;
            continue;
        }
        if b[i] == quote {
            return (value, i + 1);
        }
        let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
        value.push(ch);
        i += ch.len_utf8();
    }
    (value, i)
}

fn read_template(src: &str, start: usize, extra: &mut Vec<String>) -> (Tok, usize) {
    let b = src.as_bytes();
    let mut i = start + 1;
    let mut value = String::new();
    let mut interp = false;
    while i < b.len() {
        if b[i] == b'\\' {
            let (ch, next) = decode_escape(src, i);
            value.push(ch);
            i = next;
            continue;
        }
        if b[i] == b'`' {
            i += 1;
            break;
        }
        if b[i] == b'$' && b.get(i + 1) == Some(&b'{') {
            interp = true;
            i += 2;
            let body = i;
            let mut depth = 1;
            while i < b.len() && depth > 0 {
                if b[i] == b'`' {
                    let (_, next) = read_template(src, i, extra);
                    i = next;
                    continue;
                }
                if b[i] == b'\\' {
                    i = (i + 2).min(b.len());
                    continue;
                }
                match b[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    extra.push(src[body..i].to_string());
                    i += 1;
                    break;
                }
                let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
                i += ch.len_utf8();
            }
            continue;
        }
        let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
        value.push(ch);
        i += ch.len_utf8();
    }
    if interp {
        (Tok::Interp, i)
    } else {
        (Tok::Str(value), i)
    }
}

fn decode_escape(src: &str, slash: usize) -> (char, usize) {
    let b = src.as_bytes();
    let i = slash + 1;
    if i >= b.len() {
        return ('\\', i);
    }
    match b[i] {
        b'n' => ('\n', i + 1),
        b'r' => ('\r', i + 1),
        b't' => ('\t', i + 1),
        b'b' => ('\u{0008}', i + 1),
        b'f' => ('\u{000c}', i + 1),
        b'v' => ('\u{000b}', i + 1),
        b'0' => ('\0', i + 1),
        b'\\' => ('\\', i + 1),
        b'\'' => ('\'', i + 1),
        b'"' => ('"', i + 1),
        b'`' => ('`', i + 1),
        b'x' => hex_char(src, i + 1, 2).unwrap_or(('x', i + 1)),
        b'u' if b.get(i + 1) == Some(&b'{') => {
            let start = i + 2;
            let mut j = start;
            while j < b.len() && b[j] != b'}' && j - start < 6 {
                j += 1;
            }
            let digits = &src[start..j];
            let ch = u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or('u');
            (
                ch,
                if j < b.len() && b[j] == b'}' {
                    j + 1
                } else {
                    j
                },
            )
        }
        b'u' => hex_char(src, i + 1, 4).unwrap_or(('u', i + 1)),
        other => (other as char, i + 1),
    }
}

fn hex_char(src: &str, start: usize, n: usize) -> Option<(char, usize)> {
    let b = src.as_bytes();
    if start + n > b.len() {
        return None;
    }
    let digits = &src[start..start + n];
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let code = u32::from_str_radix(digits, 16).ok()?;
    Some((char::from_u32(code).unwrap_or('\u{FFFD}'), start + n))
}

fn is_ident_start(src: &str, i: usize) -> bool {
    let b = src.as_bytes();
    if b[i].is_ascii_alphabetic() || b[i] == b'_' || b[i] == b'$' {
        return true;
    }
    if b[i] == b'\\' && b.get(i + 1) == Some(&b'u') {
        return true;
    }
    src[i..].chars().next().is_some_and(|c| c.is_alphabetic())
}

fn read_ident(src: &str, start: usize) -> (String, usize) {
    let b = src.as_bytes();
    let mut i = start;
    let mut name = String::new();
    while i < b.len() {
        if b[i] == b'\\' && b.get(i + 1) == Some(&b'u') {
            let (ch, next) = decode_escape(src, i);
            if ch == '\\' || ch == 'u' {
                break;
            }
            name.push(ch);
            i = next;
            continue;
        }
        let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' || ch.is_alphabetic() {
            name.push(ch);
            i += ch.len_utf8();
            continue;
        }
        break;
    }
    (name, i)
}

fn skip_number(src: &str, start: usize) -> usize {
    let b = src.as_bytes();
    let mut i = start;
    if b.get(i) == Some(&b'0')
        && matches!(b.get(i + 1), Some(b'x' | b'X' | b'b' | b'B' | b'o' | b'O'))
    {
        i += 2;
        while i < b.len() && (b[i].is_ascii_hexdigit() || b[i] == b'_') {
            i += 1;
        }
        return i;
    }
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'_' || b[i] == b'.') {
        i += 1;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'_') {
            i += 1;
        }
    }
    i
}

fn punct(b: &[u8], i: usize) -> (Tok, usize) {
    let two = |a, c| b.get(i) == Some(&a) && b.get(i + 1) == Some(&c);
    if two(b'+', b'+')
        || two(b'-', b'-')
        || two(b'+', b'=')
        || two(b'-', b'=')
        || two(b'*', b'=')
        || two(b'/', b'=')
        || two(b'%', b'=')
        || two(b'&', b'=')
        || two(b'|', b'=')
        || two(b'^', b'=')
        || (b.get(i) == Some(&b'?') && b.get(i + 1) == Some(&b'?') && b.get(i + 2) == Some(&b'='))
    {
        let n = if b.get(i) == Some(&b'?') { 3 } else { 2 };
        return (Tok::Assign, n);
    }
    if two(b'=', b'=')
        || two(b'!', b'=')
        || two(b'<', b'=')
        || two(b'>', b'=')
        || two(b'&', b'&')
        || two(b'|', b'|')
        || two(b'?', b'?')
        || two(b'=', b'>')
        || two(b'<', b'<')
        || two(b'>', b'>')
    {
        let triple = (b.get(i + 2) == Some(&b'=')
            && (b.get(i) == Some(&b'=') || b.get(i) == Some(&b'!'))
            && b.get(i + 1) == Some(&b'='))
            || (b.get(i) == Some(&b'>')
                && b.get(i + 1) == Some(&b'>')
                && b.get(i + 2) == Some(&b'>'));
        let n = if triple { 3 } else { 2 };
        return (Tok::Other, n);
    }
    let tok = match b[i] {
        b'.' => Tok::Dot,
        b'[' => Tok::LBrack,
        b']' => Tok::RBrack,
        b'(' => Tok::LParen,
        b')' => Tok::RParen,
        b'{' => Tok::LBrace,
        b'}' => Tok::RBrace,
        b',' => Tok::Comma,
        b'+' => Tok::Plus,
        b'=' => Tok::Assign,
        _ => Tok::Other,
    };
    (tok, 1)
}
