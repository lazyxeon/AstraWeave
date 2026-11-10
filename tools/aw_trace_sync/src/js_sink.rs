//! Deny-by-default scan of the map page.
//!
//! A call, an assignment, a reference, or a markup form is a sink unless it is
//! positively one of the allowlisted shapes below. That includes script bodies
//! (any tag case, exact `type`, end tags with space before `>`), event-handler
//! attributes, `javascript:` URLs, and `srcdoc`. A computed call is allowlisted
//! only for a static allowlisted name. A computed assignment is allowlisted
//! only for a static allowlisted name, or for a member or call key that
//! contains no string. Unterminated input, a depth overrun, and a scan that
//! cannot finish are sinks. `setAttribute` names and string timers are checked
//! after escapes and `'a'+'b'` folding.

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

const HANDLER_GLOBAL: &[&str] = &[
    "onabort",
    "onanimationend",
    "onblur",
    "oncanplay",
    "onchange",
    "onclick",
    "oncontextmenu",
    "ondblclick",
    "ondrag",
    "ondrop",
    "onerror",
    "onfocus",
    "onfocusin",
    "onfocusout",
    "onhashchange",
    "oninput",
    "oninvalid",
    "onkeydown",
    "onkeypress",
    "onkeyup",
    "onload",
    "onmessage",
    "onmousedown",
    "onmouseenter",
    "onmouseleave",
    "onmousemove",
    "onmouseout",
    "onmouseover",
    "onmouseup",
    "onoffline",
    "ononline",
    "onpagehide",
    "onpageshow",
    "onpointerdown",
    "onpointerup",
    "onpopstate",
    "onreset",
    "onresize",
    "onscroll",
    "onsearch",
    "onselect",
    "onstorage",
    "onsubmit",
    "ontoggle",
    "ontouchend",
    "ontouchmove",
    "ontouchstart",
    "ontransitionend",
    "onunload",
    "onwheel",
];

const SINK_MEMBER: &[&str] = &[
    "innerHTML",
    "insertAdjacentHTML",
    "outerHTML",
    "srcdoc",
    "write",
    "writeln",
];

const URL_ATTR: &[&str] = &[
    "action",
    "background",
    "cite",
    "data",
    "formaction",
    "href",
    "poster",
    "src",
];

const ELEMENT_TAG: &[&str] = &["button", "li", "option", "span"];

const MAX_TEMPLATE_DEPTH: u32 = 8;

pub fn html_parsing_sink(src: &str) -> Option<String> {
    html_parsing_problems(src).into_iter().next()
}

pub fn html_parsing_problems(src: &str) -> BTreeSet<String> {
    let owned = src.to_string();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scan_measured(&owned))) {
        Ok((problems, _)) => problems,
        Err(_) => {
            let mut out = BTreeSet::new();
            out.insert("scan failed closed".to_string());
            out
        }
    }
}

fn scan_measured(src: &str) -> (BTreeSet<String>, u64) {
    let mut problems = BTreeSet::new();
    let mut steps = 0u64;
    let limit = (src.len() as u64).saturating_mul(8).saturating_add(64);
    let chunks = if is_markup(src) {
        html_scripts(src, &mut problems, &mut steps, limit)
    } else {
        vec![src.to_string()]
    };
    for chunk in &chunks {
        if steps > limit {
            break;
        }
        scan_js(chunk, &mut problems, &mut steps, limit);
        legacy_names(chunk, &mut problems);
    }
    if steps > limit {
        problems.insert("scan failed closed".to_string());
    }
    (problems, steps)
}

fn allowed(list: &[&str], name: &str) -> bool {
    list.binary_search(&name).is_ok()
}

fn keyword(name: &str) -> bool {
    KEYWORDS.binary_search(&name).is_ok()
}

fn sink_member(name: &str) -> bool {
    allowed(SINK_MEMBER, name)
}

fn handler_global(name: &str) -> bool {
    allowed(HANDLER_GLOBAL, name)
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Tpl(String),
    Interp,
    Num,
    Dot,
    Opt,
    LBrack,
    RBrack,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Plus,
    Assign,
    Arrow,
    Other,
    Error,
}

fn scan_js(src: &str, out: &mut BTreeSet<String>, steps: &mut u64, limit: u64) {
    let mut lx = Lx {
        src,
        i: 0,
        steps: *steps,
        limit,
        failed: false,
    };
    let tokens = lx.lex(false, 0);
    *steps = lx.steps;
    if lx.failed || tokens.iter().any(|tok| matches!(tok, Tok::Error)) {
        out.insert("scan failed closed".to_string());
    }
    let mate = match_brackets(&tokens);
    let stringish = stringish_names(&tokens);
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Tok::Error => {
                out.insert("scan failed closed".to_string());
            }
            Tok::Dot | Tok::Opt => {
                note_dot(&tokens, i, &stringish, out);
                if matches!(tokens.get(i + 1), Some(Tok::Ident(_))) {
                    i += 2;
                    continue;
                }
            }
            Tok::LBrack if prev_is_value(&tokens, i) => {
                if let Some(end) = mate[i] {
                    note_computed(&tokens, i, end, &stringish, out);
                    i = end + 1;
                    continue;
                }
                out.insert("scan failed closed".to_string());
            }
            Tok::Ident(name) if name == "new" => {
                if let Some(Tok::Ident(ctor)) = tokens.get(i + 1) {
                    if !keyword(ctor) && !allowed(NEW_OK, ctor) {
                        out.insert(format!("new {ctor}"));
                    }
                }
            }
            Tok::Ident(name) if name == "import" => {
                if matches!(tokens.get(i + 1), Some(Tok::LParen)) {
                    out.insert("bare import".to_string());
                }
            }
            Tok::Ident(name)
                if matches!(tokens.get(i + 1), Some(Tok::Opt))
                    && matches!(tokens.get(i + 2), Some(Tok::LParen)) =>
            {
                note_bare_call(name, &tokens, i + 2, &stringish, out);
            }
            Tok::Ident(name) if matches!(tokens.get(i + 1), Some(Tok::Tpl(_) | Tok::Interp)) => {
                note_tagged(name, out);
            }
            Tok::Ident(name) if matches!(tokens.get(i + 1), Some(Tok::LParen)) => {
                let defined = i > 0 && matches!(&tokens[i - 1], Tok::Ident(p) if keyword(p));
                if !defined && !keyword(name) {
                    note_bare_call(name, &tokens, i + 1, &stringish, out);
                }
            }
            Tok::Ident(name) if name == "eval" || name == "Function" => {
                out.insert(format!("bare {name}"));
            }
            Tok::Ident(name)
                if matches!(tokens.get(i + 1), Some(Tok::Assign))
                    && (name == "location" || handler_global(name))
                    && !prev_is_keyword(&tokens, i) =>
            {
                out.insert(format!("bare assign {name}"));
            }
            Tok::LParen if i > 0 && matches!(tokens.get(i - 1), Some(Tok::RParen)) => {
                note_grouped_call(&tokens, &mate, i - 1, i, &stringish, out);
            }
            _ => {}
        }
        i += 1;
    }
}

fn note_dot(tokens: &[Tok], dot: usize, stringish: &BTreeSet<String>, out: &mut BTreeSet<String>) {
    let Some(Tok::Ident(name)) = tokens.get(dot + 1) else {
        return;
    };
    if dot > 0 && matches!(&tokens[dot - 1], Tok::Ident(recv) if recv == "Reflect") {
        out.insert("member call Reflect".to_string());
        return;
    }
    if sink_member(name) {
        out.insert(format!("member {name}"));
    }
    let after = dot + 2;
    if matches!(tokens.get(after), Some(Tok::LParen))
        || (matches!(tokens.get(after), Some(Tok::Opt))
            && matches!(tokens.get(after + 1), Some(Tok::LParen)))
    {
        let paren = if matches!(tokens.get(after), Some(Tok::LParen)) {
            after
        } else {
            after + 1
        };
        if matches!(name.as_str(), "call" | "apply" | "bind") && !borrow_ok(tokens, dot) {
            out.insert(format!("member call {name}"));
            return;
        }
        note_member_call(name, tokens, paren, stringish, out);
    } else if assign_follows(tokens, after) && !allowed(MEMBER_ASSIGN, name) {
        out.insert(format!("member assign {name}"));
    }
}

fn borrow_ok(tokens: &[Tok], dot: usize) -> bool {
    let Some(end) = dot.checked_sub(1) else {
        return false;
    };
    let Tok::Ident(name) = &tokens[end] else {
        return false;
    };
    end > 0
        && matches!(tokens[end - 1], Tok::Dot | Tok::Opt)
        && allowed(MEMBER_CALL, name)
        && !sink_member(name)
}

fn assign_follows(tokens: &[Tok], i: usize) -> bool {
    matches!(tokens.get(i), Some(Tok::Assign))
}

fn note_computed(
    tokens: &[Tok],
    open: usize,
    end: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    let key = classify_key(&tokens[open + 1..end]);
    let after = end + 1;
    let call = matches!(tokens.get(after), Some(Tok::LParen))
        || (matches!(tokens.get(after), Some(Tok::Opt))
            && matches!(tokens.get(after + 1), Some(Tok::LParen)));
    let assign = assign_follows(tokens, after);
    let paren = if matches!(tokens.get(after), Some(Tok::LParen)) {
        after
    } else {
        after + 1
    };
    if let Key::Static(name) = &key {
        if sink_member(name) {
            out.insert(format!("member {name}"));
        }
    }
    if call {
        match key {
            Key::Static(name) => note_member_call(&name, tokens, paren, stringish, out),
            _ => {
                out.insert("computed call".to_string());
            }
        }
        return;
    }
    if assign {
        match key {
            Key::Static(name) => {
                if !allowed(MEMBER_ASSIGN, &name) {
                    out.insert(format!("member assign {name}"));
                }
            }
            Key::Member | Key::Call => {}
            _ => {
                out.insert("computed assign".to_string());
            }
        }
    }
}

fn note_member_call(
    name: &str,
    tokens: &[Tok],
    paren: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    if name == "setAttribute" || name == "setAttributeNS" {
        note_set_attribute(tokens, paren, out);
        return;
    }
    if name == "createElement" {
        note_create_element(tokens, paren, stringish, out);
        return;
    }
    if !allowed(MEMBER_CALL, name) {
        out.insert(format!("member call {name}"));
        return;
    }
    if name == "setTimeout" || name == "setInterval" {
        note_timer(tokens, paren, stringish, out);
    }
}

fn note_bare_call(
    name: &str,
    tokens: &[Tok],
    paren: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    if name == "eval" || name == "Function" {
        out.insert(format!("bare {name}"));
        return;
    }
    if !allowed(BARE_CALL, name) {
        out.insert(format!("bare {name}"));
        return;
    }
    if name == "setTimeout" || name == "setInterval" {
        note_timer(tokens, paren, stringish, out);
    }
}

fn note_tagged(name: &str, out: &mut BTreeSet<String>) {
    if name == "setTimeout" || name == "setInterval" {
        out.insert("string timer".to_string());
        return;
    }
    if !allowed(BARE_CALL, name) || name == "eval" || name == "Function" {
        out.insert(format!("bare {name}"));
    }
}

fn note_timer(
    tokens: &[Tok],
    paren: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    match tokens.get(paren + 1) {
        Some(Tok::Str(_) | Tok::Tpl(_) | Tok::Interp) => {
            out.insert("string timer".to_string());
        }
        Some(Tok::Ident(name)) if name == "function" => {}
        Some(Tok::Ident(name)) => {
            let next = tokens.get(paren + 2);
            if matches!(next, Some(Tok::LParen | Tok::Tpl(_) | Tok::Interp))
                || stringish.contains(name)
            {
                out.insert("string timer".to_string());
            }
        }
        Some(Tok::LParen) => {
            if !arrow_or_function(tokens, paren + 1) {
                out.insert("string timer".to_string());
            }
        }
        _ => {}
    }
}

fn arrow_or_function(tokens: &[Tok], open: usize) -> bool {
    let mut depth = 0i32;
    for (j, tok) in tokens.iter().enumerate().skip(open) {
        match tok {
            Tok::LParen => depth += 1,
            Tok::RParen => {
                depth -= 1;
                if depth == 0 {
                    return matches!(tokens.get(j + 1), Some(Tok::Arrow))
                        || matches!(tokens.get(open + 1), Some(Tok::Ident(n)) if n == "function");
                }
            }
            _ => {}
        }
    }
    false
}

fn note_set_attribute(tokens: &[Tok], paren: usize, out: &mut BTreeSet<String>) {
    match first_arg(tokens, paren) {
        Arg::Static(attr) => {
            let lower = attr.to_ascii_lowercase();
            if !allowed(ATTR_OK, &attr) || lower.starts_with("on") {
                out.insert(format!("attribute {attr}"));
                return;
            }
            if allowed(URL_ATTR, &lower) && value_has_script_scheme(tokens, paren) {
                out.insert(format!("attribute {attr}"));
            }
        }
        Arg::Other | Arg::Missing => {
            out.insert("computed attribute".to_string());
        }
    }
}

fn note_create_element(
    tokens: &[Tok],
    paren: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    match tokens.get(paren + 1) {
        Some(Tok::Str(tag) | Tok::Tpl(tag)) => {
            if !allowed(ELEMENT_TAG, tag) {
                out.insert(format!("element {tag}"));
            }
        }
        Some(Tok::Ident(name)) => {
            let calls = matches!(
                tokens.get(paren + 2),
                Some(Tok::LParen | Tok::Tpl(_) | Tok::Interp)
            );
            if calls || stringish.contains(name) {
                out.insert("element".to_string());
            }
        }
        Some(Tok::RParen) | None => {
            out.insert("element".to_string());
        }
        Some(_) => {
            out.insert("element".to_string());
        }
    }
}

fn value_has_script_scheme(tokens: &[Tok], paren: usize) -> bool {
    let mut depth = 0i32;
    let mut arg = 0i32;
    let mut parts: Vec<&Tok> = Vec::new();
    for tok in tokens.iter().skip(paren) {
        match tok {
            Tok::LParen => {
                depth += 1;
                if depth > 1 && arg == 1 {
                    parts.push(tok);
                }
            }
            Tok::RParen => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                if arg == 1 {
                    parts.push(tok);
                }
            }
            Tok::Comma if depth == 1 => {
                arg += 1;
                if arg > 1 {
                    break;
                }
            }
            _ if depth == 1 && arg == 1 => parts.push(tok),
            _ => {}
        }
    }
    scriptish(&parts)
}

fn scriptish(parts: &[&Tok]) -> bool {
    let mut folded = String::new();
    let mut expect = true;
    let mut pure = true;
    for tok in parts {
        let text = match tok {
            Tok::Str(s) | Tok::Tpl(s) => s,
            Tok::Plus if !expect => {
                expect = true;
                continue;
            }
            _ => {
                pure = false;
                continue;
            }
        };
        if script_scheme(text) {
            return true;
        }
        if pure && expect {
            folded.push_str(text);
            expect = false;
        } else {
            pure = false;
        }
    }
    pure && script_scheme(&folded)
}

fn script_scheme(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("javascript:") || lower.starts_with("vbscript:") || lower.starts_with("data:")
}

fn note_grouped_call(
    tokens: &[Tok],
    mate: &[Option<usize>],
    close: usize,
    call: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    let Some(open) = mate[close] else {
        out.insert("indirect call".to_string());
        return;
    };
    if comma_at_root(tokens, open, close) {
        out.insert("indirect call".to_string());
        return;
    }
    let interior = &tokens[open + 1..close];
    if let [Tok::Ident(name)] = interior {
        note_bare_call(name, tokens, call, stringish, out);
        return;
    }
    if matches!(interior.first(), Some(Tok::Ident(name)) if name == "function") {
        return;
    }
    if let [Tok::Ident(_), Tok::Dot, Tok::Ident(name)] | [Tok::Dot, Tok::Ident(name)] = interior {
        note_member_call(name, tokens, call, stringish, out);
        return;
    }
    out.insert("indirect call".to_string());
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

enum Key {
    Static(String),
    Bare,
    Num,
    Member,
    Call,
    Other,
}

fn classify_key(parts: &[Tok]) -> Key {
    if parts.len() == 1 {
        return match &parts[0] {
            Tok::Str(s) | Tok::Tpl(s) => Key::Static(s.clone()),
            Tok::Num => Key::Num,
            Tok::Ident(_) => Key::Bare,
            Tok::Interp => Key::Other,
            _ => Key::Other,
        };
    }
    let mut folded = String::new();
    let mut expect_str = true;
    let mut strings = 0usize;
    let mut pure = true;
    for tok in parts {
        match (expect_str, tok) {
            (true, Tok::Str(s) | Tok::Tpl(s)) => {
                folded.push_str(s);
                strings += 1;
                expect_str = false;
            }
            (false, Tok::Plus) => expect_str = true,
            _ => {
                pure = false;
                break;
            }
        }
    }
    if pure && strings >= 1 && !expect_str {
        return Key::Static(folded);
    }
    let has_string = parts
        .iter()
        .any(|tok| matches!(tok, Tok::Str(_) | Tok::Tpl(_) | Tok::Interp));
    if !has_string && is_member_key(parts) {
        return Key::Member;
    }
    if !has_string && is_call_key(parts) {
        return Key::Call;
    }
    Key::Other
}

fn is_member_key(parts: &[Tok]) -> bool {
    if parts.len() < 3 || parts.len() % 2 == 0 {
        return false;
    }
    parts.iter().enumerate().all(|(i, tok)| {
        if i % 2 == 0 {
            matches!(tok, Tok::Ident(_))
        } else {
            matches!(tok, Tok::Dot | Tok::Opt)
        }
    })
}

fn is_call_key(parts: &[Tok]) -> bool {
    if parts.len() < 3 || !matches!(parts.first(), Some(Tok::Ident(_))) {
        return false;
    }
    if !matches!(parts.get(1), Some(Tok::LParen)) || !matches!(parts.last(), Some(Tok::RParen)) {
        return false;
    }
    let mut depth = 0i32;
    for (i, tok) in parts.iter().enumerate().skip(1) {
        match tok {
            Tok::LParen => depth += 1,
            Tok::RParen => {
                depth -= 1;
                if depth == 0 {
                    return i + 1 == parts.len();
                }
            }
            _ => {}
        }
    }
    false
}

enum Arg {
    Static(String),
    Other,
    Missing,
}

fn first_arg(tokens: &[Tok], paren: usize) -> Arg {
    let mut depth = 0i32;
    let mut parts: Vec<&Tok> = Vec::new();
    for tok in tokens.iter().skip(paren) {
        match tok {
            Tok::LParen => {
                depth += 1;
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
    match classify_key(&parts.into_iter().cloned().collect::<Vec<_>>()) {
        Key::Static(s) => Arg::Static(s),
        _ => Arg::Other,
    }
}

fn prev_is_value(tokens: &[Tok], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    match &tokens[i - 1] {
        Tok::Ident(name) if !keyword(name) => true,
        Tok::RParen | Tok::RBrack | Tok::Str(_) | Tok::Tpl(_) | Tok::Num | Tok::Interp => true,
        _ => false,
    }
}

fn prev_is_keyword(tokens: &[Tok], i: usize) -> bool {
    i > 0 && matches!(&tokens[i - 1], Tok::Ident(name) if keyword(name))
}

fn stringish_names(tokens: &[Tok]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut i = 0;
    while i + 2 < tokens.len() {
        if let Tok::Ident(kw) = &tokens[i] {
            if matches!(kw.as_str(), "const" | "let" | "var") {
                if let Tok::Ident(name) = &tokens[i + 1] {
                    if matches!(tokens.get(i + 2), Some(Tok::Assign))
                        && init_is_stringish(&tokens[i + 3..])
                    {
                        names.insert(name.clone());
                    }
                }
            }
        }
        i += 1;
    }
    names
}

fn init_is_stringish(tokens: &[Tok]) -> bool {
    match tokens.first() {
        Some(Tok::Str(_) | Tok::Tpl(_) | Tok::Interp) => true,
        Some(Tok::Ident(name)) if name != "function" => {
            matches!(tokens.get(1), Some(Tok::LParen | Tok::Tpl(_) | Tok::Interp))
        }
        _ => false,
    }
}

fn match_brackets(tokens: &[Tok]) -> Vec<Option<usize>> {
    let mut mate = vec![None; tokens.len()];
    let mut stack: Vec<(u8, usize)> = Vec::new();
    for (i, tok) in tokens.iter().enumerate() {
        let open = match tok {
            Tok::LParen => Some(0u8),
            Tok::LBrack => Some(1),
            Tok::LBrace => Some(2),
            Tok::RParen => Some(10),
            Tok::RBrack => Some(11),
            Tok::RBrace => Some(12),
            _ => None,
        };
        let Some(kind) = open else {
            continue;
        };
        if kind < 10 {
            stack.push((kind, i));
            continue;
        }
        let Some((pk, pi)) = stack.pop() else {
            continue;
        };
        if pk + 10 == kind {
            mate[pi] = Some(i);
            mate[i] = Some(pi);
        }
    }
    mate
}

struct Lx<'a> {
    src: &'a str,
    i: usize,
    steps: u64,
    limit: u64,
    failed: bool,
}

impl<'a> Lx<'a> {
    fn lex(&mut self, stop_brace: bool, depth: u32) -> Vec<Tok> {
        let b = self.src.as_bytes();
        let mut out = Vec::new();
        let mut ctrl: Vec<bool> = Vec::new();
        let mut regex_ready = false;
        let mut brace = 0i32;
        while self.i < b.len() {
            if self.steps > self.limit {
                self.failed = true;
                out.push(Tok::Error);
                return out;
            }
            if !self.src.is_char_boundary(self.i) {
                self.i += 1;
                self.steps = self.steps.saturating_add(1);
                continue;
            }
            let start = self.i;
            let c = b[self.i];
            if c.is_ascii_whitespace() {
                self.i += 1;
                self.charge(start);
                continue;
            }
            if c == b'/' && b.get(self.i + 1) == Some(&b'/') {
                self.i += 2;
                while self.i < b.len() && b[self.i] != b'\n' {
                    self.i = bump(self.src, self.i);
                }
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'/' && b.get(self.i + 1) == Some(&b'*') {
                self.i += 2;
                while self.i + 1 < b.len() && !(b[self.i] == b'*' && b[self.i + 1] == b'/') {
                    self.i = bump(self.src, self.i);
                }
                self.i = (self.i + 2).min(b.len());
                if self.i >= b.len() && !self.src[start..].contains("*/") {
                    self.failed = true;
                    out.push(Tok::Error);
                }
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if self.src[self.i..].starts_with("<!--") {
                while self.i < b.len() && b[self.i] != b'\n' {
                    self.i = bump(self.src, self.i);
                }
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'/' && regex_here(out.last(), regex_ready) {
                let next = skip_regex(self.src, self.i + 1);
                if next == self.i + 1 || (next >= b.len() && !closed_regex(self.src, self.i + 1)) {
                    self.failed = true;
                    out.push(Tok::Error);
                    self.i = b.len();
                    return out;
                }
                self.i = next;
                out.push(Tok::Other);
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'\'' || c == b'"' {
                let (value, next, closed) = read_string(self.src, self.i);
                if !closed {
                    self.failed = true;
                    out.push(Tok::Error);
                }
                out.push(Tok::Str(value));
                self.i = next;
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'`' {
                let (tok, body) = self.read_template(depth);
                out.push(tok);
                out.extend(body);
                regex_ready = false;
                continue;
            }
            if c == b'.'
                && b.get(self.i + 1).is_some_and(u8::is_ascii_digit)
                && !prev_is_value(&out, out.len())
            {
                self.i = skip_number(self.src, self.i);
                out.push(Tok::Num);
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'?' && b.get(self.i + 1) == Some(&b'.') {
                out.push(Tok::Opt);
                self.i += 2;
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if is_ident_start(self.src, self.i) {
                let (name, next) = read_ident(self.src, self.i);
                out.push(Tok::Ident(name));
                self.i = next;
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c.is_ascii_digit() {
                self.i = skip_number(self.src, self.i);
                out.push(Tok::Num);
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if stop_brace && c == b'}' && brace == 0 {
                self.charge(start);
                return out;
            }
            let (tok, n) = punct(b, self.i);
            if n == 0 || self.i + n > b.len() {
                self.failed = true;
                out.push(Tok::Error);
                self.i = bump(self.src, self.i);
                return out;
            }
            match &tok {
                Tok::LParen => {
                    let control =
                        matches!(out.last(), Some(Tok::Ident(name)) if control_keyword(name));
                    ctrl.push(control);
                    regex_ready = false;
                }
                Tok::RParen => {
                    regex_ready = ctrl.pop().unwrap_or(false);
                }
                Tok::LBrace => {
                    brace += 1;
                    regex_ready = false;
                }
                Tok::RBrace => {
                    brace -= 1;
                    regex_ready = false;
                }
                _ => regex_ready = false,
            }
            out.push(tok);
            self.i += n;
            self.charge(start);
        }
        if stop_brace {
            self.failed = true;
            out.push(Tok::Error);
        }
        out
    }

    fn charge(&mut self, start: usize) {
        let n = (self.i.saturating_sub(start) as u64).max(1);
        self.steps = self.steps.saturating_add(n);
        if self.steps > self.limit {
            self.failed = true;
        }
    }

    fn read_template(&mut self, depth: u32) -> (Tok, Vec<Tok>) {
        let start_tick = self.i;
        if depth >= MAX_TEMPLATE_DEPTH {
            self.failed = true;
            self.i = skip_template_linear(self.src, self.i);
            self.charge(start_tick);
            return (Tok::Error, Vec::new());
        }
        let b = self.src.as_bytes();
        self.i += 1;
        self.charge(start_tick);
        let mut value = String::new();
        let mut interp = false;
        let mut body = Vec::new();
        while self.i < b.len() {
            if self.steps > self.limit {
                self.failed = true;
                return (Tok::Error, body);
            }
            if !self.src.is_char_boundary(self.i) {
                self.i += 1;
                self.steps = self.steps.saturating_add(1);
                continue;
            }
            let tick = self.i;
            if b[self.i] == b'\\' {
                let (ch, next) = decode_escape(self.src, self.i);
                value.push(ch);
                self.i = next;
                self.charge(tick);
                continue;
            }
            if b[self.i] == b'`' {
                self.i += 1;
                self.charge(tick);
                return if interp {
                    (Tok::Interp, body)
                } else {
                    (Tok::Tpl(value), body)
                };
            }
            if b[self.i] == b'$' && b.get(self.i + 1) == Some(&b'{') {
                interp = true;
                self.i += 2;
                self.charge(tick);
                let nested = self.lex(true, depth + 1);
                body.extend(nested);
                if self.i < b.len() && b[self.i] == b'}' {
                    self.i += 1;
                    self.steps = self.steps.saturating_add(1);
                }
                continue;
            }
            let ch = self.src[self.i..].chars().next().unwrap_or('\u{FFFD}');
            value.push(ch);
            self.i += ch.len_utf8();
            self.charge(tick);
        }
        self.failed = true;
        (Tok::Error, body)
    }
}

fn control_keyword(name: &str) -> bool {
    matches!(name, "if" | "for" | "while" | "switch" | "with" | "catch")
}

fn regex_here(prev: Option<&Tok>, ready: bool) -> bool {
    if ready {
        return true;
    }
    match prev {
        None => true,
        Some(Tok::Ident(name)) if keyword(name) => true,
        Some(
            Tok::Ident(_)
            | Tok::Num
            | Tok::Str(_)
            | Tok::Tpl(_)
            | Tok::RParen
            | Tok::RBrack
            | Tok::Interp,
        ) => false,
        _ => true,
    }
}

fn closed_regex(src: &str, mut i: usize) -> bool {
    let b = src.as_bytes();
    let mut class = false;
    while i < b.len() {
        if !src.is_char_boundary(i) {
            return false;
        }
        match b[i] {
            b'\\' => {
                i += 1;
                if i < src.len() {
                    i = bump(src, i);
                }
            }
            b'[' => {
                class = true;
                i += 1;
            }
            b']' => {
                class = false;
                i += 1;
            }
            b'/' if !class => return true,
            b'\n' => return false,
            _ => i = bump(src, i),
        }
    }
    false
}

fn skip_regex(src: &str, mut i: usize) -> usize {
    let b = src.as_bytes();
    let mut class = false;
    while i < b.len() {
        if !src.is_char_boundary(i) {
            return i;
        }
        match b[i] {
            b'\\' => {
                i += 1;
                if i < src.len() {
                    i = bump(src, i);
                }
            }
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
            _ => i = bump(src, i),
        }
    }
    i
}

fn read_string(src: &str, start: usize) -> (String, usize, bool) {
    let b = src.as_bytes();
    let quote = b[start];
    let mut i = start + 1;
    let mut value = String::new();
    while i < b.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if b[i] == b'\\' {
            let (ch, next) = decode_escape(src, i);
            value.push(ch);
            i = next;
            continue;
        }
        if b[i] == quote {
            return (value, i + 1, true);
        }
        let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
        value.push(ch);
        i += ch.len_utf8();
    }
    (value, i, false)
}

fn decode_escape(src: &str, slash: usize) -> (char, usize) {
    let b = src.as_bytes();
    let i = slash + 1;
    if i >= b.len() || !src.is_char_boundary(i) {
        return ('\\', i.min(src.len()));
    }
    match b[i] {
        b'n' => ('\n', i + 1),
        b'r' => ('\r', i + 1),
        b't' => ('\t', i + 1),
        b'b' => ('\u{0008}', i + 1),
        b'f' => ('\u{000c}', i + 1),
        b'v' => ('\u{000b}', i + 1),
        b'0' => ('\0', i + 1),
        b'\\' | b'\'' | b'"' | b'`' | b'$' => (b[i] as char, i + 1),
        b'x' => hex_char(src, i + 1, 2).unwrap_or(('x', i + 1)),
        b'u' if b.get(i + 1) == Some(&b'{') => {
            let start = i + 2;
            let mut j = start;
            while j < b.len() && b[j].is_ascii_hexdigit() && j - start < 6 {
                j += 1;
            }
            let digits = &src[start..j];
            let ch = u32::from_str_radix(digits, 16)
                .ok()
                .and_then(char::from_u32)
                .unwrap_or('u');
            let next = if j < b.len() && b[j] == b'}' {
                j + 1
            } else {
                j
            };
            (ch, next)
        }
        b'u' => hex_char(src, i + 1, 4).unwrap_or(('u', i + 1)),
        _ => {
            let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
            (ch, i + ch.len_utf8())
        }
    }
}

fn hex_char(src: &str, start: usize, n: usize) -> Option<(char, usize)> {
    let b = src.as_bytes();
    if start + n > b.len() {
        return None;
    }
    for byte in &b[start..start + n] {
        if !byte.is_ascii_hexdigit() {
            return None;
        }
    }
    let digits = &src[start..start + n];
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
        if !src.is_char_boundary(i) {
            break;
        }
        if b[i] == b'\\' && b.get(i + 1) == Some(&b'u') {
            let (ch, next) = decode_escape(src, i);
            if ch == '\\' || ch == 'u' || next == i {
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
    if name.is_empty() {
        let ch = src[start..].chars().next().unwrap_or('\u{FFFD}');
        return (ch.to_string(), start + ch.len_utf8());
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
    let rest = b.len() - i;
    let starts = |lit: &[u8]| rest >= lit.len() && &b[i..i + lit.len()] == lit;
    if starts(b">>>=") {
        return (Tok::Assign, 4);
    }
    for lit in [b">>=", b"<<=", b"**=", b"&&=", b"||=", b"??="] {
        if starts(lit) {
            return (Tok::Assign, 3);
        }
    }
    for lit in [b"===", b"!==", b">>>"] {
        if starts(lit) {
            return (Tok::Other, 3);
        }
    }
    if starts(b"=>") {
        return (Tok::Arrow, 2);
    }
    for lit in [b"+=", b"-=", b"*=", b"/=", b"%=", b"&=", b"|=", b"^="] {
        if starts(lit) {
            return (Tok::Assign, 2);
        }
    }
    for lit in [
        b"==", b"!=", b"<=", b">=", b"&&", b"||", b"??", b"<<", b">>", b"**", b"++", b"--",
    ] {
        if starts(lit) {
            let assign = lit == b"++" || lit == b"--";
            return if assign {
                (Tok::Assign, 2)
            } else {
                (Tok::Other, 2)
            };
        }
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

fn bump(src: &str, i: usize) -> usize {
    if i >= src.len() || !src.is_char_boundary(i) {
        return (i + 1).min(src.len());
    }
    let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
    i + ch.len_utf8()
}

fn skip_template_linear(src: &str, start: usize) -> usize {
    let b = src.as_bytes();
    let mut i = start + 1;
    let mut mode: Vec<u8> = vec![0];
    let mut braces: Vec<i32> = vec![0];
    while i < b.len() && !mode.is_empty() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let kind = *mode.last().unwrap();
        if b[i] == b'\\' && kind != 2 {
            i += 1;
            if i < src.len() {
                i = bump(src, i);
            }
            continue;
        }
        if kind == 0 {
            if b[i] == b'`' {
                mode.pop();
                braces.pop();
                i += 1;
                if mode.is_empty() {
                    return i;
                }
                continue;
            }
            if b[i] == b'$' && b.get(i + 1) == Some(&b'{') {
                mode.push(1);
                braces.push(1);
                i += 2;
                continue;
            }
            i = bump(src, i);
            continue;
        }
        if b[i] == b'\'' || b[i] == b'"' {
            i = skip_quoted(src, i);
            continue;
        }
        if b[i] == b'`' {
            mode.push(0);
            braces.push(0);
            i += 1;
            continue;
        }
        if b[i] == b'{' {
            *braces.last_mut().unwrap() += 1;
            i += 1;
            continue;
        }
        if b[i] == b'}' {
            *braces.last_mut().unwrap() -= 1;
            i += 1;
            if *braces.last().unwrap() == 0 {
                mode.pop();
                braces.pop();
            }
            continue;
        }
        i = bump(src, i);
    }
    i
}

fn skip_quoted(src: &str, start: usize) -> usize {
    let b = src.as_bytes();
    if start >= b.len() {
        return start;
    }
    let quote = b[start];
    let mut i = start + 1;
    while i < b.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if b[i] == b'\\' {
            i += 1;
            if i < src.len() {
                i = bump(src, i);
            }
            continue;
        }
        if b[i] == quote {
            return i + 1;
        }
        i = bump(src, i);
    }
    i
}

fn is_markup(src: &str) -> bool {
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'<' && script_open_at(src, i) {
            return true;
        }
        i += 1;
    }
    attr_markup(src)
}

fn script_open_at(src: &str, i: usize) -> bool {
    let rest = &src[i + 1..];
    starts_with_name(rest, "script")
}

fn starts_with_name(src: &str, name: &str) -> bool {
    let b = src.as_bytes();
    if src.len() < name.len() || !src.is_char_boundary(name.len()) {
        return false;
    }
    if !src[..name.len()].eq_ignore_ascii_case(name) {
        return false;
    }
    match b.get(name.len()).copied() {
        None => true,
        Some(c) => !is_tag_char(c) && c != b'.',
    }
}

fn is_tag_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-' || c == b':' || c == b'_'
}

fn is_attr_char(c: u8) -> bool {
    is_tag_char(c) || c == b'.'
}

fn attr_markup(src: &str) -> bool {
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'<' {
            if let TagTry::Tag(tag) = parse_tag(src, i) {
                if tag
                    .attrs
                    .iter()
                    .any(|(name, value)| markup_attr(name, value))
                {
                    return true;
                }
                i = tag.end;
                continue;
            }
        }
        i += 1;
    }
    false
}

fn markup_attr(name: &str, value: &str) -> bool {
    if name.eq_ignore_ascii_case("srcdoc") {
        return true;
    }
    if is_handler_attr(name) {
        return true;
    }
    allowed(URL_ATTR, &name.to_ascii_lowercase()) && script_scheme(value)
}

fn is_handler_attr(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() > 2
        && name[..2].eq_ignore_ascii_case("on")
        && b[2].is_ascii_alphabetic()
        && b.iter().all(|c| c.is_ascii_alphanumeric())
}

struct Parsed {
    name: String,
    attrs: Vec<(String, String)>,
    end: usize,
    closing: bool,
}

enum TagTry {
    Tag(Parsed),
    NotTag,
    Unclosed,
}

fn parse_tag(src: &str, start: usize) -> TagTry {
    let b = src.as_bytes();
    if b.get(start) != Some(&b'<') {
        return TagTry::NotTag;
    }
    let mut i = start + 1;
    if i >= b.len() {
        return TagTry::Unclosed;
    }
    let closing = if b[i] == b'/' {
        i += 1;
        true
    } else {
        false
    };
    if i >= b.len() {
        return TagTry::Unclosed;
    }
    if !b[i].is_ascii_alphabetic() {
        return TagTry::NotTag;
    }
    let name_at = i;
    i += 1;
    while i < b.len() && is_tag_char(b[i]) {
        i += 1;
    }
    let name = src[name_at..i].to_string();
    let mut attrs = Vec::new();
    loop {
        if i >= b.len() {
            return TagTry::Unclosed;
        }
        if b[i] == b'>' {
            return TagTry::Tag(Parsed {
                name,
                attrs,
                end: i + 1,
                closing,
            });
        }
        if b[i] == b'/' || b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if !b[i].is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let attr_at = i;
        i += 1;
        while i < b.len() && is_attr_char(b[i]) {
            i += 1;
        }
        let attr = src[attr_at..i].to_string();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return TagTry::Unclosed;
        }
        if b[i] != b'=' {
            attrs.push((attr, String::new()));
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            return TagTry::Unclosed;
        }
        let value = if b[i] == b'"' || b[i] == b'\'' {
            let quote = b[i];
            i += 1;
            let vs = i;
            while i < b.len() && b[i] != quote {
                i += 1;
            }
            if i >= b.len() {
                return TagTry::Unclosed;
            }
            let value = src[vs..i].to_string();
            i += 1;
            value
        } else {
            let vs = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                i += 1;
            }
            src[vs..i].to_string()
        };
        attrs.push((attr, value));
    }
}

fn html_scripts(src: &str, out: &mut BTreeSet<String>, steps: &mut u64, limit: u64) -> Vec<String> {
    let b = src.as_bytes();
    let mut chunks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if *steps > limit {
            out.insert("scan failed closed".to_string());
            break;
        }
        if b[i] != b'<' {
            i += 1;
            *steps = steps.saturating_add(1);
            continue;
        }
        if src[i..].starts_with("<!--") {
            if let Some(rel) = src[i + 4..].find("-->") {
                let next = i + 4 + rel + 3;
                *steps = steps.saturating_add((next - i) as u64);
                i = next;
            } else {
                out.insert("scan failed closed".to_string());
                break;
            }
            continue;
        }
        match parse_tag(src, i) {
            TagTry::NotTag => {
                i += 1;
                *steps = steps.saturating_add(1);
            }
            TagTry::Unclosed => {
                out.insert("scan failed closed".to_string());
                if script_open_at(src, i) {
                    chunks.push(src[i..].to_string());
                }
                break;
            }
            TagTry::Tag(tag) => {
                *steps = steps.saturating_add((tag.end - i) as u64);
                if tag.closing {
                    i = tag.end;
                    continue;
                }
                for (name, value) in &tag.attrs {
                    note_markup_attr(name, value, out, &mut chunks);
                }
                if tag.name.eq_ignore_ascii_case("script") {
                    let skip = script_type_skips(&tag.attrs);
                    if let Some(end_gt) = find_script_end(src, tag.end) {
                        let body = body_before_end(src, tag.end, end_gt);
                        if !skip && !body.trim().is_empty() {
                            chunks.push(body.clone());
                        }
                        *steps = steps.saturating_add(body.len() as u64);
                        i = end_gt + 1;
                    } else {
                        out.insert("scan failed closed".to_string());
                        if !skip {
                            chunks.push(src[tag.end..].to_string());
                        }
                        break;
                    }
                } else {
                    i = tag.end;
                }
            }
        }
    }
    chunks
}

fn body_before_end(src: &str, body_at: usize, end_gt: usize) -> String {
    let end_lt = src[..end_gt].rfind('<').unwrap_or(body_at);
    src[body_at..end_lt].to_string()
}

fn script_type_skips(attrs: &[(String, String)]) -> bool {
    attrs.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("type")
            && (value.trim().eq_ignore_ascii_case("application/json")
                || value.trim().eq_ignore_ascii_case("application/ld+json"))
    })
}

fn find_script_end(src: &str, from: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = from;
    while i + 8 < b.len() {
        if b[i] == b'<' && b.get(i + 1) == Some(&b'/') && starts_with_name(&src[i + 2..], "script")
        {
            let mut j = i + 2 + "script".len();
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < b.len() && b[j] == b'>' {
                return Some(j);
            }
        }
        i += 1;
    }
    None
}

fn note_markup_attr(name: &str, value: &str, out: &mut BTreeSet<String>, chunks: &mut Vec<String>) {
    if name.eq_ignore_ascii_case("srcdoc") {
        out.insert("attribute srcdoc".to_string());
        if !value.trim().is_empty() {
            chunks.push(value.to_string());
        }
        return;
    }
    if is_handler_attr(name) {
        out.insert(format!("attribute {name}"));
        if !value.trim().is_empty() {
            chunks.push(value.to_string());
        }
        return;
    }
    let lower = name.to_ascii_lowercase();
    if allowed(URL_ATTR, &lower) && script_scheme(value) {
        out.insert(format!("attribute {name}"));
    }
    if name.eq_ignore_ascii_case("src") && script_scheme(value) {
        out.insert("attribute src".to_string());
    }
}

fn legacy_names(src: &str, out: &mut BTreeSet<String>) {
    let folded = fold_string_concats(src);
    for name in ["innerHTML", "outerHTML", "insertAdjacentHTML", "srcdoc"] {
        if contains_ident(&folded, name) {
            out.insert(format!("member {name}"));
        }
    }
    if contains_ident(&folded, "createContextualFragment") {
        out.insert("member call createContextualFragment".to_string());
    }
    if contains_ident(&folded, "DOMParser") && folded.contains("text/html") {
        out.insert("member call parseFromString".to_string());
    }
    if string_timer_text(&folded) {
        out.insert("string timer".to_string());
    }
    if contains_phrase(&folded, "document.write") || contains_phrase(&folded, "document.writeln") {
        out.insert("member write".to_string());
    }
    if call_with_paren(&folded, "eval") {
        out.insert("bare eval".to_string());
    }
    if call_with_paren(&folded, "Function") {
        out.insert("bare Function".to_string());
    }
}

fn contains_ident(src: &str, name: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + name.len() <= bytes.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if src[i..].starts_with(name) {
            let prev_ok = i == 0 || !is_js_ident(bytes[i - 1]);
            let after = i + name.len();
            let next_ok =
                after >= bytes.len() || !src.is_char_boundary(after) || !is_js_ident(bytes[after]);
            if prev_ok && next_ok && src.is_char_boundary(after) {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn contains_phrase(src: &str, phrase: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + phrase.len() <= bytes.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if src[i..].starts_with(phrase) {
            let prev_ok = i == 0 || !is_js_ident(bytes[i - 1]);
            let after = i + phrase.len();
            let next_ok = after >= bytes.len() || !is_js_ident(bytes[after]);
            if prev_ok && next_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_js_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

fn string_timer_text(src: &str) -> bool {
    for name in ["setTimeout", "setInterval"] {
        let bytes = src.as_bytes();
        let mut i = 0;
        while i + name.len() <= bytes.len() {
            if !src.is_char_boundary(i) {
                i += 1;
                continue;
            }
            if src[i..].starts_with(name) {
                let prev_ok = i == 0 || !is_js_ident(bytes[i - 1]);
                if prev_ok && call_opens_with_string(&src[i + name.len()..]) {
                    return true;
                }
            }
            i += 1;
        }
    }
    false
}

fn call_opens_with_string(src: &str) -> bool {
    let rest = src.trim_start();
    let Some(rest) = rest.strip_prefix('(') else {
        return false;
    };
    matches!(
        rest.trim_start().as_bytes().first(),
        Some(b'\'' | b'"' | b'`')
    )
}

fn call_with_paren(src: &str, name: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + name.len() <= bytes.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if src[i..].starts_with(name) {
            let prev_ok = i == 0 || !is_js_ident(bytes[i - 1]);
            let mut j = i + name.len();
            if prev_ok {
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'(' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

fn fold_string_concats(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if !src.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if matches!(bytes[i], b'\'' | b'"' | b'`') {
            if let Some((joined, end)) = concat_literals(src, i) {
                out.push('\'');
                out.push_str(&joined);
                out.push('\'');
                i = end;
                continue;
            }
            if let Some(end) = read_literal(src, i).map(|(_, end)| end) {
                out.push_str(&src[i..end]);
                i = end;
                continue;
            }
        }
        let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn concat_literals(src: &str, start: usize) -> Option<(String, usize)> {
    let (mut piece, mut i) = read_literal(src, start)?;
    let mut joined = piece.clone();
    let mut parts = 1;
    loop {
        let rest = src[i..].trim_start();
        let skipped = src[i..].len() - rest.len();
        if !rest.starts_with('+') {
            break;
        }
        let after_plus = rest[1..].trim_start();
        let plus_span = skipped + 1 + (rest[1..].len() - after_plus.len());
        let next_at = i + plus_span;
        if next_at >= src.len() || !matches!(src.as_bytes()[next_at], b'\'' | b'"' | b'`') {
            break;
        }
        (piece, i) = read_literal(src, next_at)?;
        joined.push_str(&piece);
        parts += 1;
    }
    if parts < 2 {
        return None;
    }
    Some((joined, i))
}

fn read_literal(src: &str, start: usize) -> Option<(String, usize)> {
    let bytes = src.as_bytes();
    let quote = bytes.get(start).copied()?;
    if !matches!(quote, b'\'' | b'"' | b'`') {
        return None;
    }
    let mut i = start + 1;
    let mut value = String::new();
    while i < bytes.len() {
        if !src.is_char_boundary(i) {
            return None;
        }
        if quote == b'`' && bytes[i] == b'$' && bytes.get(i + 1) == Some(&b'{') {
            return None;
        }
        if bytes[i] == b'\\' {
            if i + 1 >= bytes.len() {
                return None;
            }
            let ch = src[i + 1..].chars().next()?;
            value.push(ch);
            i += 1 + ch.len_utf8();
            continue;
        }
        if bytes[i] == quote {
            return Some((value, i + 1));
        }
        let ch = src[i..].chars().next()?;
        value.push(ch);
        i += ch.len_utf8();
    }
    None
}

#[cfg(test)]
fn d45_rejects(src: &str) -> bool {
    let folded = fold_string_concats(src);
    for name in ["innerHTML", "outerHTML", "insertAdjacentHTML"] {
        if folded.contains(name) {
            return true;
        }
    }
    if contains_ident(&folded, "createContextualFragment")
        || contains_ident(&folded, "srcdoc")
        || (contains_ident(&folded, "DOMParser") && folded.contains("text/html"))
        || string_timer_text(&folded)
        || folded.contains("document.write")
        || call_with_paren(&folded, "eval")
        || call_with_paren(&folded, "Function")
    {
        return true;
    }
    false
}

#[cfg(test)]
mod class_tests {
    use super::*;

    fn sink(sample: &str) {
        let (_problems, steps) = scan_measured(sample);
        assert!(
            steps <= (sample.len() as u64).saturating_mul(8).saturating_add(64),
            "steps {steps} for {sample}"
        );
        assert!(
            html_parsing_sink(sample).is_some(),
            "{sample} was not a sink"
        );
    }

    fn clean(sample: &str) {
        let (problems, steps) = scan_measured(sample);
        assert!(
            steps <= (sample.len() as u64).saturating_mul(8).saturating_add(64),
            "steps {steps}"
        );
        assert!(problems.is_empty(), "{sample} was a sink: {problems:?}");
    }

    #[test]
    fn optional_call_is_a_sink() {
        sink("eval?.(x)");
        sink("document?.write(x)");
        sink("document.write?.(x)");
        sink("eval?.(s.title)");
    }

    #[test]
    fn compound_and_logical_assignment_is_a_sink() {
        for op in ["&&=", "||=", "**=", "<<=", ">>=", ">>>=", "??="] {
            sink(&format!("node.innerHTML {op} s"));
            sink(&format!("node.outerHTML {op} s"));
        }
        sink("opt.innerHTML ||= s.title");
        sink("opt.outerHTML &&= s.title");
    }

    #[test]
    fn indirect_invocation_is_a_sink() {
        sink("eval.call(null, s)");
        sink("document.write.call(document, s)");
        sink("x.insertAdjacentHTML.call(x, 'beforeend', s)");
        sink("setTimeout.call(window, 'code')");
        sink("Reflect.set(x, 'inner' + 'HTML', s)");
        sink("Reflect.apply(fn, null, [])");
        sink("(window.document.write)(s)");
        sink("(0 || eval)(s)");
        sink("(1 ? eval : 0)(s)");
        sink("(0, eval)(x)");
        sink("[s].forEach(eval)");
        sink("[s].forEach(document.write, document)");
        sink("const { write } = document; write.call(document, s)");
    }

    #[test]
    fn tagged_template_is_a_sink() {
        sink("eval`x`");
        sink("(Function`alert(1)`)()");
        sink("setTimeout`alert(1)`");
    }

    #[test]
    fn a_sink_only_inside_a_script_block_fails() {
        sink("<script>eval(1)</script>");
        sink("<SCRIPT>eval(location.hash.slice(1))</SCRIPT>");
        sink("<script>\nlet q = 1;\n</script >\n<script>eval(1)</script >");
        sink("<script type=\"text/javascript\">eval(1)</script>");
        sink("<script data-kind=\"application/json\">eval(1)</script>");
        clean("<script type=\"application/json\">eval(1)</script>");
        clean("<script type=\"application/ld+json\">{\"a\":1}</script>");
        clean("<script src=\"https://unpkg.com/example.js\"></script>");
    }

    #[test]
    fn the_map_script_is_scanned() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/architecture/workspace_map.html");
        let html = std::fs::read_to_string(&path).unwrap();
        assert!(html_parsing_problems(&html).is_empty());
        let end = html.rfind("</script>").expect("script");
        let mut injected = html.clone();
        injected.insert_str(end, "eval(1);");
        assert!(html_parsing_sink(&injected).is_some());
    }

    #[test]
    fn markup_handlers_urls_and_srcdoc_fail() {
        sink("<img src=\"x\" alt=\"\" onerror=\"eval(1)\">");
        sink("<iframe srcdoc=\"&lt;script&gt;parent.x=1&lt;/script&gt;\"></iframe>");
        sink("<a href=\"javascript:eval(1)\">x</a>");
        sink("<a href='javascript:alert(1)'>x</a>");
        clean("var href = 'https://example.com'");
        clean("if (a < b) node.textContent = a");
    }

    #[test]
    fn untested_call_and_assign_shapes_fail_closed() {
        sink("obj[0]()");
        sink("window['ev'+name]()");
        sink("window[k]()");
        sink("(eval)(x)");
        sink("(document.write)(x)");
        sink("foo)(x)");
        sink("el.setAttribute(name, x)");
        sink("setTimeout(`${c}`)");
        sink("`x ${document.write(1)}`");
        sink("const k = 'innerHTML'; opt[k] = s");
        sink("opt[k] = s");
        sink("const c = String(s); setTimeout(c)");
        sink("import('data:text/javascript,' + s)");
        sink("location = 'javascript:' + s");
        sink("[opt.innerHTML] = [s]");
        sink("for (opt.innerHTML of [s]);");
        sink("document.createElement('script')");
        sink("document.createElement('iframe')");
        sink("opt.setAttribute('href', 'javascript:' + s)");
        sink("node['inner'+'HTML'] = x");
    }

    #[test]
    fn allowlisted_page_shapes_stay_clean() {
        clean("node.textContent = String(props.text)");
        clean("node.appendChild(document.createTextNode(String(kid)))");
        clean("function safeColor(value) { return ''; }");
        clean("setTimeout(() => {}, 10)");
        clean("setTimeout(function () {}, 1)");
        clean("setTimeout(tryRestore, 100)");
        clean("document.createElement('li')");
        clean("document.createElement('span')");
        clean("document.createElement('option')");
        clean("document.createElement('button')");
        clean("document.createElement(tag)");
        clean("document.createElement(name)");
        clean("a.setAttribute('href', String(attrs.href))");
        clean("a.setAttribute('href', REPO_BLOB + m[1])");
        clean("a.setAttribute('href', REPO_BLOB + '_meta/')");
        clean("visiblePerDomain[d.domain] = (visiblePerDomain[d.domain] || 0) + 1");
        clean("out[decodeURIComponent(k)] = v");
        clean("Object.prototype.hasOwnProperty.call(obj, key)");
        clean("(function(){ return 1 })()");
        clean("var evaluation = 1");
        clean("var srcdocs = 1");
        clean("orientation = 1");
        clean("const [k, v] = pair.split('=')");
        clean("if (ok) /eval/; node.textContent = x");
    }

    #[test]
    fn comments_regex_and_template_strings_do_not_hide_a_sink() {
        sink("<!--\neval(1)");
        sink("if (ok) /eval/; eval(1)");
        sink("`pre ${ '}' + eval(1) } post`");
    }

    #[test]
    fn hostile_input_does_not_panic_and_stays_bounded() {
        let samples = [
            "'\\é'",
            "'\\x€'",
            "`${\\é}`",
            "<script>",
            "[[[",
            "/*",
            "'",
            "\"",
            "/",
            "<!--",
            "<script>eval(1)",
            "\u{e9}",
        ];
        for sample in samples {
            let (problems, steps) = scan_measured(sample);
            assert!(steps <= (sample.len() as u64).saturating_mul(8).saturating_add(64));
            let _ = problems;
            let _ = html_parsing_problems(sample);
        }
        let mut state = 0x1234_5678_u64;
        for n in 0..200 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let len = (state % 80) as usize;
            let mut buf = String::new();
            for _ in 0..len {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                buf.push(char::from((state % 96) as u8 + 32));
            }
            let (_problems, steps) = scan_measured(&buf);
            assert!(
                steps <= (buf.len() as u64).saturating_mul(8).saturating_add(64),
                "n={n} steps={steps} len={}",
                buf.len()
            );
            let _ = html_parsing_problems(&buf);
        }
    }

    #[test]
    fn nested_templates_stay_inside_the_step_budget() {
        let mut sample = String::from("eval(1)");
        for _ in 0..32 {
            sample = format!("`${{{sample}}}`");
        }
        let started = std::time::Instant::now();
        let (problems, steps) = scan_measured(&sample);
        assert!(
            started.elapsed().as_millis() < 500,
            "{:?}",
            started.elapsed()
        );
        assert!(steps <= (sample.len() as u64).saturating_mul(8).saturating_add(64));
        assert!(!problems.is_empty());
    }

    #[test]
    fn unclosed_brackets_are_not_quadratic() {
        let sample = "[".repeat(30_000);
        let started = std::time::Instant::now();
        let (_problems, steps) = scan_measured(&sample);
        assert!(
            started.elapsed().as_millis() < 1000,
            "{:?}",
            started.elapsed()
        );
        assert!(steps <= (sample.len() as u64).saturating_mul(8).saturating_add(64));
    }

    #[test]
    fn forms_the_previous_scanner_rejected_stay_rejected() {
        let rejected = [
            "node.innerHTML = x",
            "node.innerHTML += x",
            "node.outerHTML = x",
            "node.insertAdjacentHTML('beforeend', x)",
            "node['inner'+'HTML'] = x",
            "range.createContextualFragment(x)",
            "new DOMParser().parseFromString(x, 'text/html')",
            "iframe.srcdoc = x",
            "setTimeout('alert(1)')",
            "window.setInterval(`alert(1)`)",
            "document.write(x)",
            "document.writeln(x)",
            "eval(x)",
            "new Function('return 1')",
            "Function (x)",
        ];
        for sample in rejected {
            assert!(d45_rejects(sample), "oracle missed {sample}");
            sink(sample);
        }
        assert!(!d45_rejects("evaluation(x)"));
        assert!(!d45_rejects("srcdocs(x)"));
        assert!(!d45_rejects("setTimeout(tryRestore, 100)"));
    }
}
