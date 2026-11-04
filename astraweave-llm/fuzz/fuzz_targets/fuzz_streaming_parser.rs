//! Fuzz target for streaming LLM response parsing.
//!
//! Tests incremental parsing of chunked responses.

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

use std::collections::VecDeque;

#[derive(Debug, Arbitrary)]
struct FuzzStreamInput {
    chunks: Vec<Vec<u8>>,
}

struct StreamingJsonParser {
    buffer: String,
    depth: i32,
    in_string: bool,
    escape_next: bool,
}

impl StreamingJsonParser {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            depth: 0,
            in_string: false,
            escape_next: false,
        }
    }

    fn feed(&mut self, chunk: &str) -> Vec<String> {
        let mut complete_objects = Vec::new();

        for c in chunk.chars() {
            self.buffer.push(c);

            if self.escape_next {
                self.escape_next = false;
                continue;
            }

            match c {
                '\\' if self.in_string => {
                    self.escape_next = true;
                }
                '"' => {
                    self.in_string = !self.in_string;
                }
                '{' if !self.in_string => {
                    // A slice starts at `{`. Prefix text and a stray `}` are not part of it.
                    if self.depth == 0 {
                        self.buffer.clear();
                        self.buffer.push('{');
                    }
                    self.depth += 1;
                }
                '}' if !self.in_string => {
                    // A close brace with no open brace is not an object.
                    if self.depth == 0 {
                        self.buffer.pop();
                        continue;
                    }
                    self.depth -= 1;
                    if self.depth == 0 {
                        complete_objects.push(self.buffer.clone());
                        self.buffer.clear();
                    }
                }
                _ => {}
            }
        }

        complete_objects
    }

    fn is_complete(&self) -> bool {
        self.depth == 0 && !self.buffer.is_empty()
    }
}

/// An emitted object is the balanced slice that starts at `{`.
fn assert_brace_object(obj: &str) {
    assert!(
        obj.starts_with('{') && obj.ends_with('}'),
        "extracted slice must start at '{{': {obj:?}"
    );
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for c in obj.chars() {
        if escape {
            escape = false;
            continue;
        }
        if in_string {
            if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                assert!(depth > 0, "close brace outside an object: {obj:?}");
                depth -= 1;
            }
            _ => {}
        }
    }
    assert_eq!(depth, 0, "{obj:?}");
    assert!(!in_string, "{obj:?}");
}

fuzz_target!(|input: FuzzStreamInput| {
    let mut parser = StreamingJsonParser::new();

    for chunk in &input.chunks {
        if let Ok(s) = std::str::from_utf8(chunk) {
            for obj in parser.feed(s) {
                assert_brace_object(&obj);
            }
        }
    }

    // Feed the same chunks again. A stray `}` must not prefix the next object.
    let mut buffer_sizes: VecDeque<usize> = VecDeque::with_capacity(10);
    for chunk in &input.chunks {
        if let Ok(s) = std::str::from_utf8(chunk) {
            for obj in parser.feed(s) {
                assert_brace_object(&obj);
            }
            buffer_sizes.push_back(parser.buffer.len());
            if buffer_sizes.len() > 10 {
                buffer_sizes.pop_front();
            }
        }
    }
});
