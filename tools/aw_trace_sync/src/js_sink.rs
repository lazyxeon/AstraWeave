//! Deny-by-default scan of the map page.
//!
//! A call, an assignment, a reference, or a markup form is a sink unless it is
//! positively one of the allowlisted shapes below. That includes script bodies
//! (any tag case, the first `type`, end tags with space before `>`), event-handler
//! attributes, script-scheme URLs, and `srcdoc`. A computed call or assignment
//! is allowlisted only for a static allowlisted name. Optional computed access
//! is a computed access. A member followed by a template is a call. Script
//! sources must match the pinned URL and integrity pair. Remote-loading
//! elements and a style import are outside the allowlist. Character references
//! in markup are decoded, then tab, line feed, and carriage return are removed,
//! before a scheme check. The same whitespace strip applies to a decoded script
//! string. Unterminated input, an unclosed tag, a depth overrun, and a scan
//! that cannot finish are sinks. `setAttribute` names and string timers are
//! checked after escapes and `'a'+'b'` folding.

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
    "setText",
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

const ELEMENT_TAG: &[&str] = &[
    "a", "br", "button", "code", "details", "div", "em", "h2", "h3", "li", "option", "span",
    "strong", "summary", "ul",
];

/// Pinned external scripts: exact URL and integrity. Any other `src` is a sink.
const SCRIPT_SRC: &[(&str, &str)] = &[
    (
        "https://unpkg.com/cytoscape@3.30.0/dist/cytoscape.min.js",
        "sha384-kpMsYllYzyaWU69Piok08rPNktpnjqAoDMdB00fjqUkEk3lkuUbSuwJ+oXrjvN6B",
    ),
    (
        "https://unpkg.com/layout-base@2.0.1/layout-base.js",
        "sha384-5E2lB9AIGE6LRCnOOSTnZRlYZFZ01iMeN2fw97Z1r4Z/kXALxKw2AC+ZzQqoeDsG",
    ),
    (
        "https://unpkg.com/cose-base@2.2.0/cose-base.js",
        "sha384-RswRBkrMsPUYpJLbZ1CVA08zbNzAkykE2oGJTujBwfjWNdfxv2WVjLJNqv1LhAOp",
    ),
    (
        "https://unpkg.com/cytoscape-fcose@2.2.0/cytoscape-fcose.js",
        "sha384-uk5Wbjq1+KqUdHO30w7N7GrEGdzBhaJeW9o/ANF6v9+yx3M/cBmoX51C000JNCUf",
    ),
    (
        "https://unpkg.com/dagre@0.8.5/dist/dagre.min.js",
        "sha384-2IH3T69EIKYC4c+RXZifZRvaH5SRUdacJW7j6HtE5rQbvLhKKdawxq6vpIzJ7j9M",
    ),
    (
        "https://unpkg.com/cytoscape-dagre@2.5.0/cytoscape-dagre.js",
        "sha384-u69h9ebXeSjlg6q/rb1zKTRAGu/h8deCl0409xpS/QJctMKnc4M9Fzkm01VOQdeF",
    ),
];

const MAX_TEMPLATE_DEPTH: u32 = 8;

#[cfg(test)]
pub fn html_parsing_sink(src: &str) -> Option<String> {
    html_parsing_problems(src).into_iter().next()
}

pub fn html_parsing_problems(src: &str) -> BTreeSet<String> {
    let owned = src.to_string();
    recover(std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || scan_measured(&owned),
    )))
}

fn recover(
    result: Result<(BTreeSet<String>, u64), Box<dyn std::any::Any + Send>>,
) -> BTreeSet<String> {
    match result {
        Ok((problems, _)) => problems,
        Err(_) => {
            let mut out = BTreeSet::new();
            out.insert("scan failed closed".to_string());
            out
        }
    }
}

fn past_limit(steps: u64, limit: u64) -> bool {
    steps > limit
}

fn past_cap(spins: usize, cap: usize) -> bool {
    spins > cap
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
        if past_limit(steps, limit) {
            break;
        }
        scan_js(chunk, &mut problems, &mut steps, limit);
    }
    if past_limit(steps, limit) {
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
    let mut spins = 0usize;
    let cap = tokens.len().saturating_mul(4).saturating_add(8);
    while i < tokens.len() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            out.insert("scan failed closed".to_string());
            break;
        }
        match &tokens[i] {
            Tok::Dot | Tok::Opt => {
                note_dot(&tokens, i, &stringish, out, steps, limit);
                if matches!(tokens.get(i + 1), Some(Tok::Ident(_))) {
                    i = i.saturating_add(2);
                    continue;
                }
            }
            Tok::LBrack if prev_is_value(&tokens, i) => {
                if let Some(end) = mate[i] {
                    note_computed(&tokens, i, end, &stringish, out, steps, limit);
                    i = end + 1;
                    continue;
                }
                out.insert("scan failed closed".to_string());
            }
            Tok::Ident(name) if name == "with" => {
                out.insert("with".to_string());
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
                let defined = i > 0
                    && matches!(&tokens[i - 1], Tok::Ident(p) if matches!(p.as_str(), "function" | "class" | "new"));
                if !defined && !keyword(name) {
                    note_bare_call(name, &tokens, i + 1, &stringish, out);
                }
            }
            Tok::Ident(name)
                if matches!(
                    name.as_str(),
                    "setTimeout" | "setInterval" | "eval" | "Function"
                ) =>
            {
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
                let mut walk = Walk { steps, limit };
                note_grouped_call(&tokens, &mate, i - 1, i, &stringish, out, &mut walk);
            }
            _ => {}
        }
        i = i.saturating_add(1);
    }
}

fn note_dot(
    tokens: &[Tok],
    dot: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
    steps: &mut u64,
    limit: u64,
) {
    let Some(Tok::Ident(name)) = tokens.get(dot + 1) else {
        return;
    };
    let receiver_is =
        |recv: &str| dot > 0 && matches!(&tokens[dot - 1], Tok::Ident(got) if got == recv);
    if receiver_is("Reflect") {
        out.insert("member call Reflect".to_string());
        return;
    }
    if receiver_is("location") && matches!(name.as_str(), "replace" | "assign") {
        out.insert(format!("member call {name}"));
        return;
    }
    if receiver_is("head") && name == "append" {
        out.insert("member call append".to_string());
        return;
    }
    if sink_member(name) {
        out.insert(format!("member {name}"));
    }
    let after = dot + 2;
    if matches!(tokens.get(after), Some(Tok::Tpl(_) | Tok::Interp)) {
        note_member_tagged(name, out);
        return;
    }
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
        if receiver_is("location") && name == "href" {
            out.insert("member call href".to_string());
            return;
        }
        note_member_call(name, tokens, paren, stringish, out, steps, limit);
    } else if assign_follows(tokens, after) {
        if receiver_is("location") && name == "href" {
            out.insert("member assign href".to_string());
        } else if name == "textContent" {
            if !inside_function(tokens, dot, "setText", steps, limit) {
                out.insert("member assign textContent".to_string());
            }
        } else if !allowed(MEMBER_ASSIGN, name) {
            out.insert(format!("member assign {name}"));
        }
    }
}

fn note_member_tagged(name: &str, out: &mut BTreeSet<String>) {
    if sink_member(name) || name == "eval" || name == "Function" {
        out.insert(format!("member {name}"));
        return;
    }
    if !allowed(MEMBER_CALL, name) && !allowed(BARE_CALL, name) {
        out.insert(format!("member call {name}"));
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
    steps: &mut u64,
    limit: u64,
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
            Key::Static(name) => {
                note_member_call(&name, tokens, paren, stringish, out, steps, limit);
            }
            _ => {
                out.insert("computed call".to_string());
            }
        }
        return;
    }
    if assign {
        match key {
            Key::Static(name) => {
                if name == "textContent" {
                    if !inside_function(tokens, open, "setText", steps, limit) {
                        out.insert("member assign textContent".to_string());
                    }
                } else if !allowed(MEMBER_ASSIGN, &name) {
                    out.insert(format!("member assign {name}"));
                }
            }
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
    steps: &mut u64,
    limit: u64,
) {
    if name == "setAttribute" || name == "setAttributeNS" {
        note_set_attribute(tokens, paren, out, steps, limit);
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
    if name == "el" {
        note_el(tokens, paren, out);
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
    let arg = call_arg(tokens, paren, 0);
    if timer_arg_allowed(&arg, stringish) {
        return;
    }
    out.insert("string timer".to_string());
}

fn timer_arg_allowed(arg: &[Tok], stringish: &BTreeSet<String>) -> bool {
    if arg.is_empty() {
        return false;
    }
    if matches!(arg.first(), Some(Tok::Ident(name)) if name == "function") {
        return true;
    }
    if is_arrow(arg) {
        return true;
    }
    if arg.len() == 1 {
        if let Tok::Ident(name) = &arg[0] {
            return !stringish.contains(name) && name != "eval" && name != "Function";
        }
    }
    false
}

fn is_arrow(arg: &[Tok]) -> bool {
    let mut depth = 0i32;
    for tok in arg {
        match tok {
            Tok::LParen | Tok::LBrace | Tok::LBrack => depth += 1,
            Tok::RParen | Tok::RBrace | Tok::RBrack => depth -= 1,
            Tok::Arrow if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn call_arg(tokens: &[Tok], paren: usize, want: i32) -> Vec<Tok> {
    let mut depth = 0i32;
    let mut arg = 0i32;
    let mut started = false;
    let mut parts = Vec::new();
    for tok in tokens.iter().skip(paren) {
        match tok {
            Tok::LParen | Tok::LBrace | Tok::LBrack => {
                depth += 1;
                if depth == 1 {
                    started = true;
                    continue;
                }
                if started && arg == want {
                    parts.push(tok.clone());
                }
            }
            Tok::RParen if depth == 1 => break,
            Tok::RParen | Tok::RBrace | Tok::RBrack => {
                depth -= 1;
                if started && arg == want {
                    parts.push(tok.clone());
                }
            }
            Tok::Comma if depth == 1 => arg += 1,
            _ if started && arg == want => parts.push(tok.clone()),
            _ => {}
        }
    }
    parts
}

fn note_set_attribute(
    tokens: &[Tok],
    paren: usize,
    out: &mut BTreeSet<String>,
    steps: &mut u64,
    limit: u64,
) {
    match first_arg(tokens, paren) {
        Arg::Static(attr) => {
            let lower = attr.to_ascii_lowercase();
            if !allowed(ATTR_OK, &attr) || lower.starts_with("on") {
                out.insert(format!("attribute {attr}"));
                return;
            }
            if allowed(URL_ATTR, &lower) && !url_value_allowed(tokens, paren, steps, limit) {
                out.insert(format!("attribute {attr}"));
            }
        }
        Arg::Other | Arg::Missing => {
            out.insert("computed attribute".to_string());
        }
    }
}

fn url_value_allowed(tokens: &[Tok], paren: usize, steps: &mut u64, limit: u64) -> bool {
    if value_has_script_scheme(tokens, paren) {
        return false;
    }
    let parts = call_arg(tokens, paren, 1);
    if parts.is_empty() {
        return false;
    }
    if let Some(Tok::Ident(name)) = parts.first() {
        if safe_url_names(tokens).contains(name) {
            return true;
        }
    }
    if inside_function(tokens, paren, "el", steps, limit) {
        return true;
    }
    let mut folded = String::new();
    let mut expect = true;
    for tok in &parts {
        match (expect, tok) {
            (true, Tok::Str(s) | Tok::Tpl(s)) => {
                folded.push_str(s);
                expect = false;
            }
            (false, Tok::Plus) => expect = true,
            _ => return false,
        }
    }
    !expect && safe_url_literal(&folded)
}

fn safe_url_names(tokens: &[Tok]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut i = 0;
    let mut spins = 0usize;
    let cap = tokens.len().saturating_mul(4).saturating_add(8);
    while i + 3 < tokens.len() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            break;
        }
        if matches!(&tokens[i], Tok::Ident(kw) if matches!(kw.as_str(), "const" | "let" | "var")) {
            if let (Tok::Ident(name), Some(Tok::Assign), Some(Tok::Str(value) | Tok::Tpl(value))) =
                (&tokens[i + 1], tokens.get(i + 2), tokens.get(i + 3))
            {
                if safe_url_literal(value) {
                    names.insert(name.clone());
                }
            }
        }
        i = i.saturating_add(1);
    }
    names
}

fn safe_url_literal(value: &str) -> bool {
    if script_scheme(value) {
        return false;
    }
    let flat = normalize_scheme(value);
    flat.is_empty()
        || flat.starts_with("https:")
        || flat.starts_with("http:")
        || flat.starts_with("mailto:")
        || flat.starts_with("./")
        || flat.starts_with('#')
        || flat.starts_with('/')
        || !flat.contains(':')
}

fn inside_function(tokens: &[Tok], at: usize, name: &str, steps: &mut u64, limit: u64) -> bool {
    let mut depth = 0i32;
    let mut i = at;
    let mut spins = 0usize;
    let cap = tokens.len().saturating_mul(4).saturating_add(8);
    while i > 0 {
        spins = spins.saturating_add(1);
        *steps = steps.saturating_add(1);
        if past_cap(spins, cap) || past_limit(*steps, limit) {
            return false;
        }
        i = i.saturating_sub(1);
        match &tokens[i] {
            Tok::RBrace => depth += 1,
            Tok::LBrace => {
                if depth == 0 {
                    if brace_is_function(tokens, i, name, steps, limit) {
                        return true;
                    }
                } else {
                    depth -= 1;
                }
            }
            _ => {}
        }
    }
    false
}

fn brace_is_function(
    tokens: &[Tok],
    brace: usize,
    name: &str,
    steps: &mut u64,
    limit: u64,
) -> bool {
    if brace == 0 || !matches!(tokens.get(brace - 1), Some(Tok::RParen)) {
        return false;
    }
    let mut i = brace - 1;
    let mut depth = 0i32;
    let mut spins = 0usize;
    let cap = tokens.len().saturating_mul(4).saturating_add(8);
    loop {
        spins = spins.saturating_add(1);
        *steps = steps.saturating_add(1);
        if past_cap(spins, cap) || past_limit(*steps, limit) {
            return false;
        }
        match &tokens[i] {
            Tok::RParen => depth += 1,
            Tok::LParen => {
                depth -= 1;
                if depth == 0 {
                    return i >= 2
                        && matches!(&tokens[i - 1], Tok::Ident(got) if got == name)
                        && matches!(&tokens[i - 2], Tok::Ident(kw) if kw == "function");
                }
            }
            _ => {}
        }
        if i == 0 {
            return false;
        }
        i = i.saturating_sub(1);
    }
}

fn note_el(tokens: &[Tok], paren: usize, out: &mut BTreeSet<String>) {
    let tag = match tokens.get(paren + 1) {
        Some(Tok::Str(tag) | Tok::Tpl(tag)) => tag.clone(),
        _ => {
            out.insert("element".to_string());
            return;
        }
    };
    if !allowed(ELEMENT_TAG, &tag) {
        out.insert(format!("element {tag}"));
    }
    let executable = matches!(
        tag.as_str(),
        "script" | "iframe" | "object" | "embed" | "style" | "link" | "base"
    );
    let parts = call_arg(tokens, paren, 1);
    let mut i = 0;
    while i < parts.len() {
        if let Tok::Ident(key) = &parts[i] {
            if key == "href" || key == "src" || (executable && key == "text") {
                let rest = &parts[i + 1..];
                if rest.iter().any(|tok| match tok {
                    Tok::Str(s) | Tok::Tpl(s) => script_scheme(s),
                    _ => false,
                }) || (executable && key == "text")
                {
                    out.insert(format!("attribute {key}"));
                }
            }
        }
        i = i.saturating_add(1);
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
        Some(Tok::Ident(name)) if is_el_param(tokens, name) && !stringish.contains(name) => {}
        _ => {
            out.insert("element".to_string());
        }
    }
}

fn is_el_param(tokens: &[Tok], name: &str) -> bool {
    tokens.windows(4).any(|win| {
        matches!(&win[0], Tok::Ident(kw) if kw == "function")
            && matches!(&win[1], Tok::Ident(fn_name) if fn_name == "el")
            && matches!(win[2], Tok::LParen)
            && matches!(&win[3], Tok::Ident(param) if param == name)
    })
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
    let flat = normalize_scheme(value);
    flat.starts_with("javascript:") || flat.starts_with("vbscript:") || flat.starts_with("data:")
}

fn normalize_scheme(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch <= ' ' {
            continue;
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn decode_html_chars(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some((ch, next)) = html_char_ref(input, i) {
                out.push(ch);
                i = next;
                continue;
            }
        }
        let ch = input[i..].chars().next().unwrap_or('\u{FFFD}');
        out.push(ch);
        i = i.saturating_add(ch.len_utf8());
    }
    out
}

fn html_char_ref(input: &str, at: usize) -> Option<(char, usize)> {
    let rest = &input[at + 1..];
    let bytes = rest.as_bytes();
    if bytes.first() == Some(&b'#') {
        let hex = bytes.get(1).copied() == Some(b'x') || bytes.get(1).copied() == Some(b'X');
        let digits = if hex { &rest[2..] } else { &rest[1..] };
        let mut len: usize = 0;
        for b in digits.bytes() {
            let ok = if hex {
                b.is_ascii_hexdigit()
            } else {
                b.is_ascii_digit()
            };
            if !ok {
                break;
            }
            len = len.saturating_add(1);
            if len == 7 {
                break;
            }
        }
        if len == 0 {
            return None;
        }
        let raw = &digits[..len];
        let code = u32::from_str_radix(raw, if hex { 16 } else { 10 }).ok()?;
        let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
        let mut next = at + 1 + if hex { 2 } else { 1 } + len;
        if input.as_bytes().get(next) == Some(&b';') {
            next = next.saturating_add(1);
        }
        return Some((ch, next));
    }
    let mut len: usize = 0;
    for b in bytes {
        if !b.is_ascii_alphanumeric() {
            break;
        }
        len = len.saturating_add(1);
        if len == 32 {
            break;
        }
    }
    if len == 0 {
        return None;
    }
    let name = &rest[..len];
    let ch = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "colon" | "Colon" => ':',
        "Tab" | "NewLine" => {
            if name == "Tab" {
                '\t'
            } else {
                '\n'
            }
        }
        _ => return None,
    };
    let mut next = at + 1 + len;
    if input.as_bytes().get(next) == Some(&b';') {
        next = next.saturating_add(1);
    }
    Some((ch, next))
}

struct Walk<'a> {
    steps: &'a mut u64,
    limit: u64,
}

fn note_grouped_call(
    tokens: &[Tok],
    mate: &[Option<usize>],
    close: usize,
    call: usize,
    stringish: &BTreeSet<String>,
    out: &mut BTreeSet<String>,
    walk: &mut Walk<'_>,
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
        note_member_call(name, tokens, call, stringish, out, walk.steps, walk.limit);
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
        Tok::Opt
        | Tok::RParen
        | Tok::RBrack
        | Tok::Str(_)
        | Tok::Tpl(_)
        | Tok::Num
        | Tok::Interp => true,
        _ => false,
    }
}

fn prev_is_keyword(tokens: &[Tok], i: usize) -> bool {
    i > 0 && matches!(&tokens[i - 1], Tok::Ident(name) if keyword(name))
}

fn stringish_names(tokens: &[Tok]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut i = 0;
    let mut spins = 0usize;
    let cap = tokens.len().saturating_mul(4).saturating_add(8);
    while i + 2 < tokens.len() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            break;
        }
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
        i = i.saturating_add(1);
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
            if past_limit(self.steps, self.limit) {
                self.failed = true;
                out.push(Tok::Error);
                return out;
            }
            if !self.src.is_char_boundary(self.i) {
                self.i = self.i.saturating_add(1);
                self.steps = self.steps.saturating_add(1);
                continue;
            }
            let start = self.i;
            let c = b[self.i];
            if c.is_ascii_whitespace() {
                self.i = self.i.saturating_add(1);
                self.charge(start);
                continue;
            }
            if c == b'/' && b.get(self.i + 1) == Some(&b'/') {
                self.i = self.i.saturating_add(2);
                while self.i < b.len() && b[self.i] != b'\n' {
                    if !self.advance() {
                        return out;
                    }
                }
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'/' && b.get(self.i + 1) == Some(&b'*') {
                self.i = self.i.saturating_add(2);
                while self.i + 1 < b.len() && !(b[self.i] == b'*' && b[self.i + 1] == b'/') {
                    if !self.advance() {
                        return out;
                    }
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
                    if !self.advance() {
                        return out;
                    }
                }
                regex_ready = false;
                self.charge(start);
                continue;
            }
            if c == b'/' && regex_here(out.last(), regex_ready, keyword_member(&out)) {
                let next = skip_regex(self.src, self.i + 1);
                if next > b.len()
                    || next == self.i + 1
                    || (next >= b.len() && !closed_regex(self.src, self.i + 1))
                {
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
                self.i = self.i.saturating_add(2);
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
                if !self.advance() {
                    return out;
                }
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
            self.i = self.i.saturating_add(n);
            self.charge(start);
        }
        if stop_brace {
            self.failed = true;
            out.push(Tok::Error);
        }
        out
    }

    fn advance(&mut self) -> bool {
        match forward(self.src, self.i) {
            Some(next) => {
                self.i = next;
                true
            }
            None => {
                self.failed = true;
                self.i = self.src.len();
                false
            }
        }
    }

    fn charge(&mut self, start: usize) {
        let n = (self.i.saturating_sub(start) as u64).max(1);
        self.steps = self.steps.saturating_add(n);
        if past_limit(self.steps, self.limit) {
            self.failed = true;
        }
    }

    fn read_template(&mut self, depth: u32) -> (Tok, Vec<Tok>) {
        let start_tick = self.i;
        if depth >= MAX_TEMPLATE_DEPTH {
            self.failed = true;
            let next = skip_template_linear(self.src, self.i);
            self.i = next.min(self.src.len());
            if next > self.src.len() {
                self.failed = true;
            }
            self.charge(start_tick);
            return (Tok::Error, Vec::new());
        }
        let b = self.src.as_bytes();
        self.i = self.i.saturating_add(1);
        self.charge(start_tick);
        let mut value = String::new();
        let mut interp = false;
        let mut body = Vec::new();
        while self.i < b.len() {
            if past_limit(self.steps, self.limit) {
                self.failed = true;
                return (Tok::Error, body);
            }
            if !self.src.is_char_boundary(self.i) {
                self.i = self.i.saturating_add(1);
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
                self.i = self.i.saturating_add(1);
                self.charge(tick);
                return if interp {
                    (Tok::Interp, body)
                } else {
                    (Tok::Tpl(value), body)
                };
            }
            if b[self.i] == b'$' && b.get(self.i + 1) == Some(&b'{') {
                interp = true;
                self.i = self.i.saturating_add(2);
                self.charge(tick);
                let nested = self.lex(true, depth + 1);
                body.extend(nested);
                if self.i < b.len() && b[self.i] == b'}' {
                    self.i = self.i.saturating_add(1);
                    self.steps = self.steps.saturating_add(1);
                }
                continue;
            }
            let ch = self.src[self.i..].chars().next().unwrap_or('\u{FFFD}');
            value.push(ch);
            self.i = self.i.saturating_add(ch.len_utf8());
            self.charge(tick);
        }
        self.failed = true;
        (Tok::Error, body)
    }
}

fn control_keyword(name: &str) -> bool {
    matches!(name, "if" | "for" | "while" | "switch" | "with" | "catch")
}

fn keyword_member(out: &[Tok]) -> bool {
    let Some(Tok::Ident(name)) = out.last() else {
        return false;
    };
    keyword(name)
        && matches!(
            out.get(out.len().wrapping_sub(2)),
            Some(Tok::Dot | Tok::Opt)
        )
}

fn regex_here(prev: Option<&Tok>, ready: bool, member: bool) -> bool {
    if ready {
        return true;
    }
    match prev {
        None => true,
        Some(Tok::Ident(name)) if keyword(name) => !member,
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
    let cap = src.len().saturating_mul(4).saturating_add(8);
    let mut spins = 0usize;
    while i < b.len() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            return false;
        }
        if !src.is_char_boundary(i) {
            return false;
        }
        match b[i] {
            b'\\' => {
                i = i.saturating_add(1);
                if i < src.len() {
                    i = match forward(src, i) {
                        Some(next) => next,
                        None => return false,
                    };
                }
            }
            b'[' => {
                class = true;
                i = i.saturating_add(1);
            }
            b']' => {
                class = false;
                i = i.saturating_add(1);
            }
            b'/' if !class => return true,
            b'\n' => return false,
            _ => {
                i = match forward(src, i) {
                    Some(next) => next,
                    None => return false,
                };
            }
        }
    }
    false
}

fn skip_regex(src: &str, mut i: usize) -> usize {
    let b = src.as_bytes();
    let mut class = false;
    let cap = src.len().saturating_mul(4).saturating_add(8);
    let mut spins = 0usize;
    let fail = src.len().saturating_add(1);
    while i < b.len() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            return fail;
        }
        if !src.is_char_boundary(i) {
            return i;
        }
        match b[i] {
            b'\\' => {
                i = i.saturating_add(1);
                if i < src.len() {
                    i = match forward(src, i) {
                        Some(next) => next,
                        None => return fail,
                    };
                }
            }
            b'[' => {
                class = true;
                i = i.saturating_add(1);
            }
            b']' => {
                class = false;
                i = i.saturating_add(1);
            }
            b'/' if !class => {
                i = i.saturating_add(1);
                while i < b.len() && b[i].is_ascii_alphabetic() {
                    i = i.saturating_add(1);
                }
                return i;
            }
            b'\n' => return i,
            _ => {
                i = match forward(src, i) {
                    Some(next) => next,
                    None => return fail,
                };
            }
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
            i = i.saturating_add(1);
            continue;
        }
        if b[i] == b'\\' {
            let (ch, next) = decode_escape(src, i);
            if next <= i {
                return (value, i, false);
            }
            value.push(ch);
            i = next;
            continue;
        }
        if b[i] == quote {
            return (value, i + 1, true);
        }
        let ch = src[i..].chars().next().unwrap_or('\u{FFFD}');
        value.push(ch);
        i = i.saturating_add(ch.len_utf8());
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
                j = j.saturating_add(1);
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
            i = i.saturating_add(ch.len_utf8());
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
        i = i.saturating_add(2);
        while i < b.len() && (b[i].is_ascii_hexdigit() || b[i] == b'_') {
            i = i.saturating_add(1);
        }
        return i;
    }
    while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'_' || b[i] == b'.') {
        i = i.saturating_add(1);
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i = i.saturating_add(1);
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i = i.saturating_add(1);
        }
        while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'_') {
            i = i.saturating_add(1);
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

fn forward(src: &str, i: usize) -> Option<usize> {
    let next = bump(src, i);
    if next <= i {
        None
    } else {
        Some(next)
    }
}

fn skip_template_linear(src: &str, start: usize) -> usize {
    let b = src.as_bytes();
    let mut i = start + 1;
    let mut mode: Vec<u8> = vec![0];
    let mut braces: Vec<i32> = vec![0];
    let cap = src.len().saturating_mul(4).saturating_add(8);
    let mut spins = 0usize;
    let fail = src.len().saturating_add(1);
    while i < b.len() && !mode.is_empty() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            return fail;
        }
        if !src.is_char_boundary(i) {
            i = i.saturating_add(1);
            continue;
        }
        let kind = *mode.last().unwrap();
        if b[i] == b'\\' && kind != 2 {
            i = i.saturating_add(1);
            if i < src.len() {
                i = match forward(src, i) {
                    Some(next) => next,
                    None => return fail,
                };
            }
            continue;
        }
        if kind == 0 {
            if b[i] == b'`' {
                mode.pop();
                braces.pop();
                i = i.saturating_add(1);
                if mode.is_empty() {
                    return i;
                }
                continue;
            }
            if b[i] == b'$' && b.get(i + 1) == Some(&b'{') {
                mode.push(1);
                braces.push(1);
                i = i.saturating_add(2);
                continue;
            }
            i = match forward(src, i) {
                Some(next) => next,
                None => return fail,
            };
            continue;
        }
        if b[i] == b'\'' || b[i] == b'"' {
            let next = skip_quoted(src, i);
            if next <= i || next > src.len() {
                return fail;
            }
            i = next;
            continue;
        }
        if b[i] == b'`' {
            mode.push(0);
            braces.push(0);
            i = i.saturating_add(1);
            continue;
        }
        if b[i] == b'{' {
            *braces.last_mut().unwrap() += 1;
            i = i.saturating_add(1);
            continue;
        }
        if b[i] == b'}' {
            *braces.last_mut().unwrap() -= 1;
            i = i.saturating_add(1);
            if *braces.last().unwrap() == 0 {
                mode.pop();
                braces.pop();
            }
            continue;
        }
        i = match forward(src, i) {
            Some(next) => next,
            None => return fail,
        };
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
    let cap = src.len().saturating_mul(4).saturating_add(8);
    let mut spins = 0usize;
    let fail = src.len().saturating_add(1);
    while i < b.len() {
        spins = spins.saturating_add(1);
        if past_cap(spins, cap) {
            return fail;
        }
        if !src.is_char_boundary(i) {
            i = i.saturating_add(1);
            continue;
        }
        if b[i] == b'\\' {
            i = i.saturating_add(1);
            if i < src.len() {
                i = match forward(src, i) {
                    Some(next) => next,
                    None => return fail,
                };
            }
            continue;
        }
        if b[i] == quote {
            return i + 1;
        }
        i = match forward(src, i) {
            Some(next) => next,
            None => return fail,
        };
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
        i = i.saturating_add(1);
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
            match parse_tag(src, i) {
                TagTry::Tag(_) | TagTry::Unclosed { .. } => return true,
                TagTry::NotTag => {}
            }
        }
        i = i.saturating_add(1);
    }
    false
}

fn handler_value_is_code(value: &str) -> bool {
    let decoded = decode_html_chars(value).trim().to_string();
    if decoded.is_empty() {
        return true;
    }
    if decoded.starts_with("function") || decoded.contains("=>") {
        return false;
    }
    true
}

fn remote_element(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "base" | "embed" | "iframe" | "object"
    )
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
    Unclosed { at: usize },
}

fn parse_tag(src: &str, start: usize) -> TagTry {
    let b = src.as_bytes();
    if b.get(start) != Some(&b'<') {
        return TagTry::NotTag;
    }
    let mut i = start + 1;
    if i >= b.len() {
        return TagTry::Unclosed { at: i };
    }
    let closing = if b[i] == b'/' {
        i = i.saturating_add(1);
        true
    } else {
        false
    };
    if i >= b.len() {
        return TagTry::Unclosed { at: i };
    }
    if !b[i].is_ascii_alphabetic() {
        return TagTry::NotTag;
    }
    let name_at = i;
    i = i.saturating_add(1);
    while i < b.len() && is_tag_char(b[i]) {
        i = i.saturating_add(1);
    }
    let name = src[name_at..i].to_string();
    let mut attrs = Vec::new();
    loop {
        if i >= b.len() {
            return TagTry::Unclosed { at: i };
        }
        if b[i] == b'>' {
            return TagTry::Tag(Parsed {
                name,
                attrs,
                end: i + 1,
                closing,
            });
        }
        if b[i] == b'<' {
            return TagTry::Unclosed { at: i };
        }
        if b[i] == b'/' || b[i].is_ascii_whitespace() {
            i = i.saturating_add(1);
            continue;
        }
        if !b[i].is_ascii_alphabetic() {
            i = i.saturating_add(1);
            continue;
        }
        let attr_at = i;
        i = i.saturating_add(1);
        while i < b.len() && is_attr_char(b[i]) {
            i = i.saturating_add(1);
        }
        let attr = src[attr_at..i].to_string();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i = i.saturating_add(1);
        }
        if i >= b.len() {
            return TagTry::Unclosed { at: i };
        }
        if b[i] != b'=' {
            attrs.push((attr, String::new()));
            continue;
        }
        i = i.saturating_add(1);
        while i < b.len() && b[i].is_ascii_whitespace() {
            i = i.saturating_add(1);
        }
        if i >= b.len() {
            return TagTry::Unclosed { at: i };
        }
        let value = if b[i] == b'"' || b[i] == b'\'' {
            let quote = b[i];
            i = i.saturating_add(1);
            let vs = i;
            while i < b.len() && b[i] != quote {
                i = i.saturating_add(1);
            }
            if i >= b.len() {
                return TagTry::Unclosed { at: i };
            }
            let value = src[vs..i].to_string();
            i = i.saturating_add(1);
            value
        } else {
            let vs = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' && b[i] != b'<' {
                i = i.saturating_add(1);
            }
            if i < b.len() && b[i] == b'<' {
                return TagTry::Unclosed { at: i };
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
        if past_limit(*steps, limit) {
            out.insert("scan failed closed".to_string());
            break;
        }
        if b[i] != b'<' {
            i = i.saturating_add(1);
            *steps = steps.saturating_add(1);
            continue;
        }
        if src[i..].starts_with("<!--") {
            if let Some(next) = html_comment_end(src, i) {
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
                i = i.saturating_add(1);
                *steps = steps.saturating_add(1);
            }
            TagTry::Unclosed { at } => {
                *steps = steps.saturating_add((at.saturating_sub(i) as u64).max(1));
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
                if remote_element(&tag.name) {
                    out.insert(format!("element {}", tag.name));
                }
                note_script_src(&tag, out);
                note_meta_refresh(&tag, out);
                note_link_href(&tag, out);
                for (name, value) in &tag.attrs {
                    note_markup_attr(name, value, out, &mut chunks);
                }
                if tag.name.eq_ignore_ascii_case("script") {
                    let skip = script_type_skips(&tag.attrs);
                    if let Some(end_gt) = find_end_tag(src, tag.end, "script") {
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
                } else if tag.name.eq_ignore_ascii_case("style") {
                    if let Some(end_gt) = find_end_tag(src, tag.end, "style") {
                        let body = body_before_end(src, tag.end, end_gt);
                        if body.to_ascii_lowercase().contains("@import") {
                            out.insert("style import".to_string());
                        }
                        *steps = steps.saturating_add(body.len() as u64);
                        i = end_gt + 1;
                    } else {
                        out.insert("scan failed closed".to_string());
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

fn html_comment_end(src: &str, i: usize) -> Option<usize> {
    let rest = src.get(i..)?;
    if !rest.starts_with("<!--") {
        return None;
    }
    let after = &rest[4..];
    if after.starts_with('>') {
        return Some(i + 5);
    }
    if after.starts_with("->") {
        return Some(i + 6);
    }
    let dash = after.find("-->").map(|rel| i + 4 + rel + 3);
    let bang = after.find("--!>").map(|rel| i + 4 + rel + 4);
    match (dash, bang) {
        (Some(dash), Some(bang)) => Some(dash.min(bang)),
        (Some(end), None) | (None, Some(end)) => Some(end),
        (None, None) => None,
    }
}

fn script_type_skips(attrs: &[(String, String)]) -> bool {
    let Some((_, value)) = attrs
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("type"))
    else {
        return false;
    };
    let value = value.trim();
    value.eq_ignore_ascii_case("application/json")
        || value.eq_ignore_ascii_case("application/ld+json")
}

fn note_script_src(tag: &Parsed, out: &mut BTreeSet<String>) {
    if !tag.name.eq_ignore_ascii_case("script") {
        return;
    }
    let Some(src) = attr_value(&tag.attrs, "src") else {
        return;
    };
    let integrity = attr_value(&tag.attrs, "integrity").unwrap_or("");
    let pinned = SCRIPT_SRC
        .iter()
        .any(|(url, hash)| src == *url && integrity == *hash);
    if !pinned {
        out.insert("script src".to_string());
    }
}

fn note_meta_refresh(tag: &Parsed, out: &mut BTreeSet<String>) {
    if !tag.name.eq_ignore_ascii_case("meta") {
        return;
    }
    let equiv = attr_value(&tag.attrs, "http-equiv").unwrap_or("");
    if equiv.trim().eq_ignore_ascii_case("refresh") {
        out.insert("meta refresh".to_string());
    }
}

fn note_link_href(tag: &Parsed, out: &mut BTreeSet<String>) {
    if !tag.name.eq_ignore_ascii_case("link") {
        return;
    }
    let rel = attr_value(&tag.attrs, "rel")
        .unwrap_or("")
        .to_ascii_lowercase();
    let href = attr_value(&tag.attrs, "href").unwrap_or("");
    if rel
        .split_whitespace()
        .any(|part| part == "stylesheet" || part == "import")
        || !href.is_empty()
    {
        out.insert("element link".to_string());
    }
}

fn attr_value<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(attr, _)| attr.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

fn find_end_tag(src: &str, from: usize, name: &str) -> Option<usize> {
    let b = src.as_bytes();
    let mut i = from;
    while i + name.len() + 2 < b.len() {
        if b[i] == b'<' && b.get(i + 1) == Some(&b'/') && starts_with_name(&src[i + 2..], name) {
            let mut j = i + 2 + name.len();
            while j < b.len() && b[j].is_ascii_whitespace() {
                j = j.saturating_add(1);
            }
            if j < b.len() && b[j] == b'>' {
                return Some(j);
            }
        }
        i = i.saturating_add(1);
    }
    None
}

fn note_markup_attr(name: &str, value: &str, out: &mut BTreeSet<String>, chunks: &mut Vec<String>) {
    let decoded = decode_html_chars(value);
    if name.eq_ignore_ascii_case("srcdoc") {
        out.insert("attribute srcdoc".to_string());
        if !decoded.trim().is_empty() {
            chunks.push(decoded);
        }
        return;
    }
    if is_handler_attr(name) {
        if handler_value_is_code(&decoded) {
            out.insert(format!("attribute {name}"));
        }
        if !decoded.trim().is_empty() {
            chunks.push(decoded);
        }
        return;
    }
    if script_scheme(&decoded) {
        out.insert(format!("attribute {name}"));
    }
}

#[cfg(test)]
fn contains_ident(src: &str, name: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + name.len() <= bytes.len() {
        if !src.is_char_boundary(i) {
            i = i.saturating_add(1);
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
        i = i.saturating_add(1);
    }
    false
}

#[cfg(test)]
fn is_js_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

#[cfg(test)]
fn string_timer_text(src: &str) -> bool {
    for name in ["setTimeout", "setInterval"] {
        let bytes = src.as_bytes();
        let mut i = 0;
        while i + name.len() <= bytes.len() {
            if !src.is_char_boundary(i) {
                i = i.saturating_add(1);
                continue;
            }
            if src[i..].starts_with(name) {
                let prev_ok = i == 0 || !is_js_ident(bytes[i - 1]);
                if prev_ok && call_opens_with_string(&src[i + name.len()..]) {
                    return true;
                }
            }
            i = i.saturating_add(1);
        }
    }
    false
}

#[cfg(test)]
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

#[cfg(test)]
fn call_with_paren(src: &str, name: &str) -> bool {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + name.len() <= bytes.len() {
        if !src.is_char_boundary(i) {
            i = i.saturating_add(1);
            continue;
        }
        if src[i..].starts_with(name) {
            let prev_ok = i == 0 || !is_js_ident(bytes[i - 1]);
            let mut j = i + name.len();
            if prev_ok {
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j = j.saturating_add(1);
                }
                if j < bytes.len() && bytes[j] == b'(' {
                    return true;
                }
            }
        }
        i = i.saturating_add(1);
    }
    false
}

#[cfg(test)]
fn fold_string_concats(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if !src.is_char_boundary(i) {
            i = i.saturating_add(1);
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
        i = i.saturating_add(ch.len_utf8());
    }
    out
}

#[cfg(test)]
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

#[cfg(test)]
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
        i = i.saturating_add(ch.len_utf8());
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

    fn pinned_script(index: usize) -> String {
        let (url, hash) = SCRIPT_SRC[index];
        format!("<script src=\"{url}\" integrity=\"{hash}\" crossorigin=\"anonymous\"></script>")
    }

    fn cat(parts: &[&str]) -> String {
        parts.concat()
    }

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
        sink("<script src=\"https://example.invalid/x.js\"></script>");
        clean(&pinned_script(0));
    }

    #[test]
    fn the_map_script_is_scanned() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/architecture/workspace_map.html");
        let html = std::fs::read_to_string(&path).unwrap();
        let problems = html_parsing_problems(&html);
        assert!(problems.is_empty(), "{problems:?}");
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
        clean("if (a < b) node.title = a");
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
        clean("function setText(node, value) { node.textContent = value; }");
        sink("x.textContent = s");
        clean("node.appendChild(document.createTextNode(String(kid)))");
        clean("function safeColor(value) { return ''; }");
        clean("setTimeout(() => {}, 10)");
        clean("setTimeout(function () {}, 1)");
        clean("setTimeout(tryRestore, 100)");
        clean("document.createElement('li')");
        clean("document.createElement('span')");
        clean("document.createElement('option')");
        clean("document.createElement('button')");
        clean("function el(tag) { return document.createElement(tag); }");
        clean("const REPO_BLOB = 'https://github.com/lazyxeon/AstraWeave/blob/main/'; a.setAttribute('href', REPO_BLOB + m[1])");
        clean("const REPO_BLOB = 'https://github.com/lazyxeon/AstraWeave/blob/main/'; a.setAttribute('href', REPO_BLOB + '_meta/')");
        clean("visiblePerDomain.set(d.domain, (visiblePerDomain.get(d.domain) || 0) + 1)");
        clean("out.set('q', v)");
        clean("node.title = text");
        clean("a.setAttribute('target', x)");
        clean("node.addEventListener('click', 'ready')");
        clean("node.on('click', 'ready')");
        clean("setTimeout(fn, 'delay')");
        clean("var href = 'javascript&colon;ready'");
        clean("Object.prototype.hasOwnProperty.call(obj, key)");
        clean("(function(){ return 1 })()");
        clean("var evaluation = 1");
        clean("var srcdocs = 1");
        clean("orientation = 1");
        clean("const [k, v] = pair.split('=')");
        clean("if (ok) /eval/; node.title = x");
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

    #[test]
    fn member_template_is_a_sink() {
        sink(&cat(&["globalThis.eval`", "alert", "(1)`"]));
    }

    #[test]
    fn optional_computed_call_is_a_sink() {
        sink(&cat(&["document?.['", "write", "'](x)"]));
        sink(&cat(&["node?.['", "writeln", "'](s)"]));
        sink(&cat(&["node?.['", "wr", "'+'ite", "'](s)"]));
        sink(&cat(&[
            "window?.['",
            "document",
            "']?.['",
            "write",
            "'](s)",
        ]));
    }

    #[test]
    fn helper_and_constructor_arguments_are_a_sink() {
        sink(&cat(&["el('", "script", "',{text:'", "alert", "(1)'})"]));
        sink("x.textContent = s");
        sink("document.createElement(tag)");
        sink("document.head.append(node)");
        sink(&cat(&["el('a',{attrs:{href:'", "javascript:", "x'}})"]));
        sink("node[a.b] = x");
        sink("node[f()] = x");
        sink("return open(x)");
        sink("location.replace(x)");
        sink("with (scope) { href = x }");
    }

    #[test]
    fn timer_concatenation_is_a_sink() {
        sink(&cat(&["setTimeout(x+'", "alert", "(1)')"]));
        sink(&cat(&["setTimeout(x+'", "code", "')"]));
        sink("setTimeout(['code'])");
        sink("list.forEach(setTimeout)");
    }

    #[test]
    fn scheme_whitespace_is_a_sink() {
        sink(&cat(&[
            "a.setAttribute('href','java\\t",
            "script:",
            "alert(1)')",
        ]));
        sink("a.setAttribute('href', 'java\\tscript:x')");
        sink("a.setAttribute('href', 'java\\nscript:x')");
        sink("a.setAttribute('href', 'java\\rscript:x')");
        sink("a.setAttribute('href', '\\u0001javascript:x')");
    }

    #[test]
    fn unpinned_script_source_is_a_sink() {
        sink("<script src=\"https://example.invalid/a.js\"></script>");
        let (url, _) = SCRIPT_SRC[0];
        sink(&format!("<script src=\"{url}\"></script>"));
        clean(&pinned_script(0));
    }

    #[test]
    fn character_references_are_a_sink() {
        sink(&cat(&["<a href=\"java", "script&colon;x\">z</a>"]));
        sink(&cat(&["<a href=\"&#106;", "avascript:x\">z</a>"]));
        sink(&cat(&["<a href=\"java&#9;", "script:x\">z</a>"]));
        sink("<math><maction href=\"javascript:x\">z</maction></math>");
        sink("<svg><script>eval(1)</script></svg>");
    }

    #[test]
    fn remote_markup_is_a_sink() {
        sink("<iframe src=\"https://example.invalid/x\"></iframe>");
        sink("<object data=\"https://example.invalid/x\"></object>");
        sink("<embed src=\"https://example.invalid/x\"></embed>");
        sink("<base href=\"https://example.invalid/\">");
        sink("<meta http-equiv=\"refresh\" content=\"0;url=https://example.invalid/\">");
        sink("<link rel=\"stylesheet\" href=\"https://example.invalid/a.css\">");
        sink("<style>@import url(\"https://example.invalid/a.css\");</style>");
        sink("<script type=\"text/javascript\" type=\"application/json\">eval(1)</script>");
        clean("<img onerror=\"function(){}\">");
    }

    #[test]
    fn comment_closers_do_not_hide_a_later_sink() {
        sink(&cat(&["<!-->", "<script>eval(1)</script>"]));
        sink(&cat(&["<!--->", "<script>eval(1)</script>"]));
        sink(&cat(&["<!-- --!>", "<script>eval(1)</script>"]));
        for sample in [
            "<!--> <script>foo(1)</script> -->",
            "<!---> <script>foo(1)</script> -->",
            "<!-- --!> <script>foo(1)</script> -->",
        ] {
            let problems = html_parsing_problems(sample);
            assert!(problems.contains("bare foo"), "{sample} -> {problems:?}");
            assert!(
                !problems.iter().any(|item| item.contains("failed closed")),
                "{sample} -> {problems:?}"
            );
        }
    }

    #[test]
    fn a_keyword_member_divisor_stays_a_call() {
        for sample in [
            "obj.return / document.write(1) / 1",
            "obj?.delete / document.write(1) / 1",
        ] {
            let problems = html_parsing_problems(sample);
            assert!(
                problems.contains("member call write"),
                "{sample} -> {problems:?}"
            );
            assert!(
                !problems.iter().any(|item| item.contains("failed closed")),
                "{sample} -> {problems:?}"
            );
        }
    }

    #[test]
    fn many_text_assignments_fail_closed_in_linear_time() {
        let sample = "x.textContent=1;".repeat(8_000);
        let started = std::time::Instant::now();
        let (problems, steps) = scan_measured(&sample);
        assert!(
            started.elapsed().as_millis() < 1000,
            "{:?}",
            started.elapsed()
        );
        assert!(
            problems.iter().any(|item| item.contains("failed closed")),
            "{problems:?}"
        );
        let limit = (sample.len() as u64).saturating_mul(8).saturating_add(64);
        assert!(steps <= limit.saturating_mul(2), "{steps} > {limit}");
    }

    #[test]
    fn a_data_scheme_attribute_is_a_sink() {
        let problems = html_parsing_problems("<a href=\"data:text/html,x\"></a>");
        assert!(problems.contains("attribute href"), "{problems:?}");
    }

    #[test]
    fn markup_names_are_case_insensitive() {
        let script = html_parsing_problems("<SCRIPT>foo(1)</SCRIPT>");
        assert!(script.contains("bare foo"), "{script:?}");
        let handler = html_parsing_problems("<img ONERROR=\"foo(1)\">");
        assert!(handler.contains("attribute ONERROR"), "{handler:?}");
        let srcdoc = html_parsing_problems("<div SRCDOC=\"x\"></div>");
        assert!(srcdoc.contains("attribute srcdoc"), "{srcdoc:?}");
        let href = html_parsing_problems("<a HREF=\"javascript:x\"></a>");
        assert!(href.contains("attribute HREF"), "{href:?}");
    }

    #[test]
    fn a_panic_during_scan_fails_closed() {
        let got = recover(std::panic::catch_unwind(|| -> (BTreeSet<String>, u64) {
            panic!("scan")
        }));
        assert!(got.contains("scan failed closed"), "{got:?}");
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn unclosed_tags_fail_closed_in_linear_time() {
        let sample = "<a ".repeat(40_000);
        let started = std::time::Instant::now();
        let (problems, steps) = scan_measured(&sample);
        assert!(
            started.elapsed().as_millis() < 1000,
            "{:?}",
            started.elapsed()
        );
        assert!(
            problems.iter().any(|item| item.contains("failed closed")),
            "{problems:?}"
        );
        assert!(steps <= (sample.len() as u64).saturating_mul(8).saturating_add(64));
    }

    #[test]
    fn allowlist_guards_have_the_strict_result() {
        assert!(allowed(&["a", "b"], "a"));
        assert!(!allowed(&["a", "b"], "c"));
        assert!(sink_member("innerHTML"));
        assert!(!sink_member("textContent"));
        assert!(handler_global("onclick"));
        assert!(!handler_global("orientation"));
        assert_eq!(bump("ab", 0), 1);
        assert_eq!(bump("éx", 0), 'é'.len_utf8());
        let quoted = "'a\\'b'";
        assert_eq!(skip_quoted(quoted, 0), quoted.len());
        let (ch, next) = decode_escape("\\tX", 0);
        assert_eq!((ch, next), ('\t', 2));
        let (ch, next) = decode_escape("\\u006a", 0);
        assert_eq!(ch, 'j');
        assert!(next > 2);
        assert!(script_scheme("java\tscript:x"));
        assert!(script_scheme("\u{0001}javascript:x"));
        assert!(!script_scheme("https://example.com"));
        assert!(!script_scheme("javascript&colon;x"));
        assert_eq!(normalize_scheme("java\tscript:"), "javascript:");
    }
}

#[cfg(test)]
mod mutant_tests {
    use super::*;

    fn lex_of(src: &str, limit: u64) -> (Vec<Tok>, bool, u64) {
        let mut lx = Lx {
            src,
            i: 0,
            steps: 0,
            limit,
            failed: false,
        };
        let toks = lx.lex(false, 0);
        (toks, lx.failed, lx.steps)
    }

    fn tok_s(tok: &Tok) -> String {
        match tok {
            Tok::Ident(s) => format!("id:{s}"),
            Tok::Str(s) => format!("str:{s}"),
            Tok::Tpl(s) => format!("tpl:{s}"),
            Tok::Interp => "interp".into(),
            Tok::Num => "num".into(),
            Tok::Dot => ".".into(),
            Tok::Opt => "?.".into(),
            Tok::LBrack => "[".into(),
            Tok::RBrack => "]".into(),
            Tok::LParen => "(".into(),
            Tok::RParen => ")".into(),
            Tok::LBrace => "{".into(),
            Tok::RBrace => "}".into(),
            Tok::Comma => ",".into(),
            Tok::Plus => "+".into(),
            Tok::Assign => "=".into(),
            Tok::Arrow => "=>".into(),
            Tok::Other => "op".into(),
            Tok::Error => "err".into(),
        }
    }

    fn lex_s(src: &str) -> String {
        let (toks, failed, steps) = lex_of(src, u64::MAX / 8);
        let body = toks.iter().map(tok_s).collect::<Vec<_>>().join(" ");
        format!("{body}|f={failed}|s={steps}")
    }

    fn key_s(key: Key) -> String {
        match key {
            Key::Static(s) => format!("static:{s}"),
            Key::Bare => "bare".into(),
            Key::Num => "num".into(),
            Key::Member => "member".into(),
            Key::Call => "call".into(),
            Key::Other => "other".into(),
        }
    }

    fn arg_s(arg: Arg) -> String {
        match arg {
            Arg::Static(s) => format!("static:{s}"),
            Arg::Other => "other".into(),
            Arg::Missing => "missing".into(),
        }
    }

    fn tag_s(src: &str) -> String {
        match parse_tag(src, 0) {
            TagTry::NotTag => "not".into(),
            TagTry::Unclosed { at } => format!("unclosed:{at}"),
            TagTry::Tag(tag) => {
                let attrs = tag
                    .attrs
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "tag:{}:end{}:close{}:{attrs}",
                    tag.name, tag.end, tag.closing
                )
            }
        }
    }

    fn probs(src: &str) -> String {
        let (set, steps) = scan_measured(src);
        let limit = (src.len() as u64).saturating_mul(8).saturating_add(64);
        let body = set.into_iter().collect::<Vec<_>>().join(" | ");
        format!("{body}|steps={steps}|limit={limit}")
    }

    #[test]
    fn bounds_and_helpers_are_exact() {
        assert!(!past_limit(0, 0));
        assert!(!past_limit(1, 1));
        assert!(past_limit(2, 1));
        assert!(!past_cap(0, 0));
        assert!(!past_cap(3, 3));
        assert!(past_cap(4, 3));

        let mut out = BTreeSet::new();
        let mut steps = 0u64;
        scan_js("a", &mut out, &mut steps, 0);
        assert!(out.contains("scan failed closed"), "{out:?}");
        assert!(steps > 0);

        let mut out = BTreeSet::new();
        let mut steps = 5u64;
        scan_js("eval(1)", &mut out, &mut steps, 5);
        assert!(out.contains("bare eval"), "{out:?} steps={steps}");
        assert!(out.contains("scan failed closed"), "{out:?} steps={steps}");

        assert!(allowed(MEMBER_CALL, "push"));
        assert!(!allowed(MEMBER_CALL, "write"));
        assert!(keyword("return"));
        assert!(!keyword("eval"));
        assert!(sink_member("writeln"));
        assert!(!sink_member("push"));
        assert!(handler_global("onerror"));
        assert!(!handler_global("on"));

        let escapes = [
            ("\\n", '\n', 2usize),
            ("\\r", '\r', 2),
            ("\\t", '\t', 2),
            ("\\b", '\u{0008}', 2),
            ("\\f", '\u{000c}', 2),
            ("\\v", '\u{000b}', 2),
            ("\\0", '\0', 2),
            ("\\\\", '\\', 2),
            ("\\'", '\'', 2),
            ("\\\"", '"', 2),
            ("\\`", '`', 2),
            ("\\$", '$', 2),
            ("\\q", 'q', 2),
            ("\\x41", 'A', 4),
            ("\\x4", 'x', 2),
            ("\\x4G", 'x', 2),
            ("\\u0041", 'A', 6),
            ("\\u041", 'u', 2),
            ("\\u{41}", 'A', 6),
            ("\\u{41", 'A', 5),
            ("\\u{}", 'u', 4),
            ("\\u{110000}", 'u', 10),
            ("\\", '\\', 1),
        ];
        for (src, ch, end) in escapes {
            assert_eq!(decode_escape(src, 0), (ch, end), "{src}");
        }
        assert_eq!(hex_char("41", 0, 2), Some(('A', 2)));
        assert_eq!(hex_char("4", 0, 2), None);
        assert_eq!(hex_char("4G", 0, 2), None);
        assert_eq!(hex_char("00", 0, 0), None);

        assert_eq!(bump("ab", 0), 1);
        assert_eq!(bump("ab", 2), 2);
        assert_eq!(bump("é", 0), 2);
        assert_eq!(bump("é", 1), 2);
        assert_eq!(forward("ab", 0), Some(1));
        assert_eq!(forward("ab", 2), None);
        assert_eq!(forward("é", 1), Some(2));

        assert_eq!(read_string("'ab'", 0), ("ab".into(), 4, true));
        assert_eq!(read_string("'a\\n'", 0), ("a\n".into(), 5, true));
        assert_eq!(read_string("'a", 0), ("a".into(), 2, false));
        assert_eq!(read_string("\"a\\\"b\"", 0), ("a\"b".into(), 6, true));
        assert_eq!(skip_quoted("'a\\'b'", 0), 6);
        assert_eq!(skip_quoted("'a", 0), 2);
        assert_eq!(skip_quoted("", 0), 0);

        assert_eq!(skip_number("123", 0), 3);
        assert_eq!(skip_number("1.2", 0), 3);
        assert_eq!(skip_number("1e10", 0), 4);
        assert_eq!(skip_number("1e+2", 0), 4);
        assert_eq!(skip_number("1E-2", 0), 4);
        assert_eq!(skip_number("0xFF", 0), 4);
        assert_eq!(skip_number("0b10_1", 0), 6);
        assert_eq!(skip_number("0o7", 0), 3);
        assert_eq!(skip_number("12a", 0), 2);
        assert_eq!(skip_number(".5", 0), 2);
        assert_eq!(skip_number("", 0), 0);

        assert!(is_ident_start("a", 0));
        assert!(is_ident_start("_", 0));
        assert!(is_ident_start("$", 0));
        assert!(is_ident_start("é", 0));
        assert!(!is_ident_start("1", 0));
        assert!(is_ident_start("\\u0041", 0));
        assert_eq!(read_ident("ab1$", 0), ("ab1$".into(), 4));
        assert_eq!(read_ident("éx", 0), ("éx".into(), 3));
        assert_eq!(read_ident("\\u0041Z", 0).0, "AZ");

        assert!(closed_regex("ab/", 0));
        assert!(!closed_regex("ab", 0));
        assert!(!closed_regex("ab\n", 0));
        assert!(closed_regex("[/]/", 0));
        assert!(closed_regex("\\//", 0));
        assert_eq!(skip_regex("ab/g", 0), 4);
        assert_eq!(skip_regex("ab", 0), 2);
        assert_eq!(skip_regex("[/]/i", 0), 5);
        assert_eq!(skip_template_linear("`a`", 0), 3);
        assert_eq!(skip_template_linear("`a${b}`", 0), 7);
        assert!(skip_template_linear("`a", 0) <= 3);

        assert_eq!(normalize_scheme(" Java\tScript: "), "javascript:");
        assert!(script_scheme("javascript:x"));
        assert!(script_scheme("vbscript:x"));
        assert!(script_scheme("data:x"));
        assert!(!script_scheme("https://x"));
        assert!(!script_scheme("javascript&colon;x"));
        assert_eq!(decode_html_chars("a&amp;b"), "a&b");
        assert_eq!(decode_html_chars("&#106;s"), "js");
        assert_eq!(decode_html_chars("&#x6A;"), "j");
        assert_eq!(html_char_ref("&lt;", 0), Some(('<', 4)));
        assert_eq!(html_char_ref("&gt", 0), Some(('>', 3)));
        assert_eq!(html_char_ref("&quot;", 0), Some(('"', 6)));
        assert_eq!(html_char_ref("&apos;", 0), Some(('\'', 6)));
        assert_eq!(html_char_ref("&colon;", 0), Some((':', 7)));
        assert_eq!(html_char_ref("&Colon;", 0), Some((':', 7)));
        assert_eq!(html_char_ref("&Tab;", 0), Some(('\t', 5)));
        assert_eq!(html_char_ref("&NewLine;", 0), Some(('\n', 9)));
        assert_eq!(html_char_ref("&nope;", 0), None);
        assert_eq!(html_char_ref("&#;", 0), None);
        assert_eq!(html_char_ref("&;", 0), None);

        assert_eq!(html_comment_end("<!-->", 0), Some(5));
        assert_eq!(html_comment_end("<!--->", 0), Some(6));
        assert_eq!(html_comment_end("<!-- -->", 0), Some(8));
        assert_eq!(html_comment_end("<!-- --!>", 0), Some(9));
        assert_eq!(html_comment_end("<!--", 0), None);
        assert_eq!(html_comment_end("abc", 0), None);

        assert!(script_type_skips(&[(
            "type".into(),
            "application/json".into()
        )]));
        assert!(script_type_skips(&[(
            "TYPE".into(),
            " application/ld+json ".into()
        )]));
        assert!(!script_type_skips(&[
            ("type".into(), "text/javascript".into()),
            ("type".into(), "application/json".into()),
        ]));
        assert!(!script_type_skips(&[(
            "id".into(),
            "application/json".into()
        )]));
        assert_eq!(
            attr_value(&[("Src".into(), "a.js".into())], "src"),
            Some("a.js")
        );
        assert_eq!(attr_value(&[], "src"), None);

        assert!(is_handler_attr("onclick"));
        assert!(is_handler_attr("onClick"));
        assert!(!is_handler_attr("on"));
        assert!(!is_handler_attr("on1"));
        assert!(!is_handler_attr("on-click"));
        assert!(!is_handler_attr("orientation"));
        assert!(handler_value_is_code(""));
        assert!(handler_value_is_code("  "));
        assert!(!handler_value_is_code(" function(){}"));
        assert!(!handler_value_is_code("a=>b"));
        assert!(handler_value_is_code("alert(1)"));
        assert!(remote_element("IFRAME"));
        assert!(remote_element("base"));
        assert!(!remote_element("script"));
        assert!(starts_with_name("script>", "script"));
        assert!(starts_with_name("SCRIPT", "script"));
        assert!(!starts_with_name("scripting", "script"));
        assert!(!starts_with_name("script.x", "script"));
        assert!(!starts_with_name("sc", "script"));
        assert!(is_tag_char(b'a') && is_tag_char(b'-') && is_tag_char(b':'));
        assert!(!is_tag_char(b'.') && !is_tag_char(b' '));
        assert!(is_attr_char(b'.') && !is_attr_char(b' '));
        assert!(is_markup("<script>"));
        assert!(is_markup("<SCRIPT>"));
        assert!(is_markup("<img onerror=x>"));
        assert!(!is_markup("a < b"));
        assert!(script_open_at("<script>", 0));
        assert!(!script_open_at("<div>", 0));

        assert_eq!(tag_s("abc"), "not");
        assert_eq!(tag_s("<"), "unclosed:1");
        assert_eq!(tag_s("</"), "unclosed:2");
        assert_eq!(tag_s("<1>"), "not");
        assert_eq!(tag_s("<a>"), "tag:a:end3:closefalse:");
        assert_eq!(tag_s("</a>"), "tag:a:end4:closetrue:");
        assert_eq!(tag_s("<a href=x>"), "tag:a:end10:closefalse:href=x");
        assert_eq!(tag_s("<a href=\"x\">"), "tag:a:end12:closefalse:href=x");
        assert_eq!(tag_s("<a href='x'>"), "tag:a:end12:closefalse:href=x");
        assert_eq!(tag_s("<br/>"), "tag:br:end5:closefalse:");
        assert!(tag_s("<a href=x").starts_with("unclosed:"));
        assert!(tag_s("<a href=\"x").starts_with("unclosed:"));
        assert_eq!(
            find_end_tag("<script>x</script>", 8, "script"),
            Some("<script>x</script>".len() - 1)
        );
        assert_eq!(find_end_tag("<script>x</script >", 8, "script"), Some(18));
        assert_eq!(find_end_tag("nope", 0, "script"), None);

        assert!(safe_url_literal("https://example.com"));
        assert!(safe_url_literal("http://example.com"));
        assert!(safe_url_literal("mailto:a@b.c"));
        assert!(safe_url_literal("./a"));
        assert!(safe_url_literal("#a"));
        assert!(safe_url_literal("/a"));
        assert!(safe_url_literal(""));
        assert!(safe_url_literal("plain"));
        assert!(!safe_url_literal("javascript:x"));
        assert!(!safe_url_literal("data:x"));
        assert!(control_keyword("if") && control_keyword("for") && control_keyword("catch"));
        assert!(!control_keyword("function"));
        assert!(regex_here(None, false, false));
        assert!(regex_here(Some(&Tok::Ident("if".into())), false, false));
        assert!(!regex_here(Some(&Tok::Ident("if".into())), false, true));
        assert!(!regex_here(Some(&Tok::Ident("foo".into())), false, false));
        assert!(regex_here(Some(&Tok::Ident("foo".into())), true, true));
        assert!(!regex_here(Some(&Tok::Num), false, false));
        assert!(regex_here(Some(&Tok::Assign), false, false));
        assert!(keyword_member(&[Tok::Dot, Tok::Ident("return".into())]));
        assert!(keyword_member(&[Tok::Opt, Tok::Ident("delete".into())]));
        assert!(!keyword_member(&[Tok::Ident("return".into())]));
        assert!(!keyword_member(&[Tok::Dot, Tok::Ident("write".into())]));
        let ret = lex_of("return a", u64::MAX / 8).0;
        assert!(prev_is_keyword(&ret, 1));
        assert!(!prev_is_keyword(&ret, 0));
        assert!(!prev_is_value(&ret, 1));
        assert!(prev_is_value(&ret, 2));
        assert!(assign_follows(&[Tok::Assign], 0));
        assert!(!assign_follows(&[Tok::Ident("a".into())], 0));
        assert!(init_is_stringish(&[Tok::Str("a".into())]));
        assert!(init_is_stringish(&[Tok::Ident("foo".into()), Tok::LParen]));
        assert!(!init_is_stringish(&[
            Tok::Ident("function".into()),
            Tok::LParen
        ]));
        assert!(!init_is_stringish(&[Tok::Num]));
    }

    fn inside(src: &str) -> Vec<Tok> {
        let (toks, _, _) = lex_of(src, u64::MAX / 8);
        let open = toks
            .iter()
            .position(|tok| matches!(tok, Tok::LBrack))
            .unwrap();
        let close = toks
            .iter()
            .rposition(|tok| matches!(tok, Tok::RBrack))
            .unwrap();
        toks[open + 1..close].to_vec()
    }

    #[test]
    fn behavior_oracle_covers_each_branch() {
        let mut lines = Vec::new();
        let lex_src = [
            "",
            "a",
            "a.b",
            "a?.b",
            "a+=1",
            "a&&=b",
            "a||=b",
            "a**=b",
            "a<<=b",
            "a>>=b",
            "a>>>=b",
            "a??=b",
            "a==b",
            "a===b",
            "a!=b",
            "a=>b",
            "a++",
            "a--",
            "()",
            "[]",
            "{}",
            "a+b",
            "1.5",
            ".5",
            "0x10",
            "0b11",
            "'ab'",
            "\"a\\nb\"",
            "`a${b+1}c`",
            "// c\na",
            "/* c */a",
            "/*",
            "<!--\na",
            "/ab/g",
            "a/b",
            "if(a)/b/",
            "foo?.(1)",
            "foo`x`",
            "eval(1)",
            "'unterminated",
            "`unterminated",
            "\\u0041",
            "é",
            "   a",
            "a , b",
            "a=b",
            "a;b",
            "a!b",
            "a<b",
            "a&&b",
            "a||b",
            "a??b",
            "a<<b",
            "a>>b",
            "a>>>b",
            "a**b",
            "(a)",
            "f()",
            "`${`a`}`",
            "1e+2",
            "'a\\t\\u0041'",
            "/* c",
            "/[/]/",
            "a/=b",
        ];
        for src in lex_src {
            lines.push(format!("LEX {:?}", lex_s(src)));
        }
        let samples = [
            "eval(1)",
            "foo(1)",
            "foo?.(1)",
            "foo`x`",
            "setTimeout`x`",
            "setTimeout(foo)",
            "setTimeout('a')",
            "setTimeout(function(){})",
            "setTimeout(()=>1)",
            "new Map()",
            "new Blob()",
            "import(x)",
            "with(x){}",
            "location=1",
            "let location=1",
            "onclick=1",
            "let onclick=1",
            "document.write(1)",
            "node.title=1",
            "node.foo=1",
            "node.innerHTML=1",
            "Reflect.set(a,1,2)",
            "location.replace(1)",
            "head.append(1)",
            "arr.push(1)",
            "eval.call(null,1)",
            "obj.hasOwnProperty.call(obj,k)",
            "node['innerHTML']=1",
            "node[k]=1",
            "node[0]()",
            "node['push'](1)",
            "(0,eval)(1)",
            "(foo)(1)",
            "el('script')",
            "el('li')",
            "document.createElement('script')",
            "document.createElement('li')",
            "a.setAttribute('href','javascript:x')",
            "a.setAttribute('target','x')",
            "a.setAttribute(name,x)",
            "return open(x)",
            "void setTimeout(s)",
            "location.href=x",
            "x.textContent=s",
            "function setText(n,v){n.textContent=v}",
            "<script>eval(1)</script>",
            "<SCRIPT>foo(1)</SCRIPT>",
            "<script type=\"application/json\">eval(1)</script>",
            "<script type=\"text/javascript\" type=\"application/json\">foo(1)</script>",
            "<img onerror=\"alert(1)\">",
            "<img onerror=\"function(){}\">",
            "<a href=\"javascript:x\">z</a>",
            "<iframe src=\"https://example.invalid/x\"></iframe>",
            "<base href=\"https://example.invalid/\">",
            "<script src=\"https://example.invalid/a.js\"></script>",
            "<style>@import \"x\";</style>",
            "<meta http-equiv=\"refresh\" content=\"0;url=https://x\">",
            "<link rel=\"stylesheet\" href=\"https://x\">",
            "<!--><script>foo(1)</script>",
            "<a ",
            "<svg><script>foo(1)</script></svg>",
            "<math><maction href=\"javascript:x\">z</maction></math>",
            "<object data=\"https://example.invalid/x\"></object>",
            "<embed src=\"https://example.invalid/x\"></embed>",
            "<link rel=\"import\" href=\"https://x\">",
            "<!-- --!><script>foo(1)</script>",
            "<!---> <script>foo(1)</script>",
            "return eval(x)",
            "document?.['write'](s)",
            "globalThis.eval`x`",
            "node.innerHTML ||= s",
            "(function(){return 1})()",
            "const U='https://example.com'; a.setAttribute('href', U)",
            "function el(tag){return document.createElement(tag)}",
            "el('a',{href:'javascript:x'})",
            "setTimeout(x+'code')",
            "list.forEach(setTimeout)",
            "node[a.b]=x",
            "node[f()]=x",
        ];
        for src in samples {
            lines.push(format!("PROB {:?}", probs(src)));
        }
        let keys = [
            "['ab']",
            "['a'+'b']",
            "[foo]",
            "[1]",
            "[foo.bar]",
            "[foo()]",
            "[foo+1]",
            "[`ab`]",
            "[a?.b]",
        ];
        for src in keys {
            lines.push(format!("KEY {src} {}", key_s(classify_key(&inside(src)))));
        }
        let calls = [
            "f(a,b+c,d)",
            "f()",
            "f(function(){})",
            "setTimeout(()=>1, 2)",
        ];
        for src in calls {
            let toks = lex_of(src, u64::MAX / 8).0;
            let paren = toks
                .iter()
                .position(|tok| matches!(tok, Tok::LParen))
                .unwrap();
            lines.push(format!(
                "ARG0 {src} {}",
                lex_s_from(&call_arg(&toks, paren, 0))
            ));
            lines.push(format!(
                "ARG1 {src} {}",
                lex_s_from(&call_arg(&toks, paren, 1))
            ));
            lines.push(format!("FIRST {src} {}", arg_s(first_arg(&toks, paren))));
        }
        let bracket_src = [
            "(a[b])",
            "(a",
            "([)]",
            "{(}",
            "f(a,b)",
            "f((a,b))",
            "(function(){})()",
        ];
        for src in bracket_src {
            let toks = lex_of(src, u64::MAX / 8).0;
            let mate = match_brackets(&toks);
            let pairs = mate
                .iter()
                .enumerate()
                .filter_map(|(i, m)| m.map(|j| format!("{i}>{j}")))
                .collect::<Vec<_>>()
                .join(",");
            lines.push(format!("MATE {src} {pairs}"));
            if let (Some(open), Some(close)) = (
                toks.iter().position(|t| matches!(t, Tok::LParen)),
                toks.iter().rposition(|t| matches!(t, Tok::RParen)),
            ) {
                lines.push(format!("COMMA {src} {}", comma_at_root(&toks, open, close)));
            }
        }
        let named = "const u = 'https://example.com'; let bad = 'javascript:x'; var n = 1; const s = String(a);";
        let named_toks = lex_of(named, u64::MAX / 8).0;
        lines.push(format!(
            "SAFE {:?}",
            safe_url_names(&named_toks).into_iter().collect::<Vec<_>>()
        ));
        lines.push(format!(
            "STR {:?}",
            stringish_names(&named_toks).into_iter().collect::<Vec<_>>()
        ));
        let fn_src = "function setText(n,v){ n.textContent = v } function el(tag){ return tag }";
        let fn_toks = lex_of(fn_src, u64::MAX / 8).0;
        let at = fn_toks
            .iter()
            .position(|t| matches!(t, Tok::Ident(s) if s == "textContent"))
            .unwrap();
        lines.push(format!(
            "IN setText {} el {}",
            inside_function(&fn_toks, at, "setText", &mut 0, u64::MAX),
            inside_function(&fn_toks, at, "el", &mut 0, u64::MAX)
        ));
        lines.push(format!(
            "ELPARAM {}",
            is_el_param(&fn_toks, "tag") && !is_el_param(&fn_toks, "n")
        ));
        let borrow_src = "obj.hasOwnProperty.call(obj,k)";
        let borrow = lex_of(borrow_src, u64::MAX / 8).0;
        let dot = borrow.iter().rposition(|t| matches!(t, Tok::Dot)).unwrap();
        lines.push(format!("BORROW {}", borrow_ok(&borrow, dot)));
        let dot_eval = lex_of("eval.call(null,1)", u64::MAX / 8).0;
        let dot_e = dot_eval.iter().position(|t| matches!(t, Tok::Dot)).unwrap();
        lines.push(format!("BORROW_EVAL {}", borrow_ok(&dot_eval, dot_e)));
        for (label, part) in [
            ("arrow", "() => 1"),
            ("fn", "function(){}"),
            ("id", "foo"),
            ("eval", "eval"),
            ("str", "'a'"),
            ("nested", "(a)=>1"),
        ] {
            let toks = lex_of(&format!("setTimeout({part})"), u64::MAX / 8).0;
            let paren = toks.iter().position(|t| matches!(t, Tok::LParen)).unwrap();
            let arg = call_arg(&toks, paren, 0);
            lines.push(format!(
                "TIMER {label} arrow={} allowed={}",
                is_arrow(&arg),
                timer_arg_allowed(&arg, &BTreeSet::new())
            ));
        }
        let scheme_src = "a.setAttribute('href','java'+'script:x')";
        let scheme = lex_of(scheme_src, u64::MAX / 8).0;
        let paren = scheme
            .iter()
            .rposition(|t| matches!(t, Tok::LParen))
            .unwrap();
        lines.push(format!(
            "SCHEME {} scriptish {}",
            value_has_script_scheme(&scheme, paren),
            scriptish(&call_arg(&scheme, paren, 1).iter().collect::<Vec<_>>())
        ));
        for src in ["a", "if a", "(a)", "'a'", "1", "a?.", "return a"] {
            let toks = lex_of(src, u64::MAX / 8).0;
            lines.push(format!(
                "PREV {src:?} value={} kw={}",
                prev_is_value(&toks, toks.len()),
                prev_is_keyword(&toks, toks.len())
            ));
        }
        for parts in [
            vec![Tok::Ident("a".into()), Tok::Dot, Tok::Ident("b".into())],
            vec![Tok::Ident("a".into()), Tok::LParen, Tok::RParen],
            vec![Tok::Ident("a".into()), Tok::Plus, Tok::Num],
            vec![Tok::Num],
        ] {
            lines.push(format!(
                "MEM {} CALL {}",
                is_member_key(&parts),
                is_call_key(&parts)
            ));
        }
        for src in [
            "<a href=x>",
            "<a href=\"x\" id=1>",
            "<div class>",
            "<?",
            "<a href=x<b>",
        ] {
            lines.push(format!("TAG {src:?} {}", tag_s(src)));
        }
        let text = lines.join("\n");
        assert_eq!(text, include_str!("oracle_js.txt"));
    }

    fn lex_s_from(toks: &[Tok]) -> String {
        toks.iter().map(tok_s).collect::<Vec<_>>().join(" ")
    }
}
