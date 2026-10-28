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
                    self.depth += 1;
                }
                '}' if !self.in_string => {
                    // A close brace with no open brace is not an object.
                    if self.depth == 0 {
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

fuzz_target!(|input: FuzzStreamInput| {
    let mut parser = StreamingJsonParser::new();
    let mut all_objects = Vec::new();

    // Feed chunks incrementally
    for chunk in &input.chunks {
        if let Ok(s) = std::str::from_utf8(chunk) {
            let objects = parser.feed(s);
            all_objects.extend(objects);
        }
    }

    // Verify parser state is consistent
    assert!(
        parser.depth >= 0 || parser.in_string,
        "Parser depth went negative unexpectedly"
    );

    // Try to parse extracted objects
    for obj in &all_objects {
        let _ = serde_json::from_str::<serde_json::Value>(obj);
    }

    // Test progressive buffer growth
    let mut buffer_sizes: VecDeque<usize> = VecDeque::with_capacity(10);
    for chunk in &input.chunks {
        if let Ok(s) = std::str::from_utf8(chunk) {
            let _ = parser.feed(s);
            buffer_sizes.push_back(parser.buffer.len());
            if buffer_sizes.len() > 10 {
                buffer_sizes.pop_front();
            }
        }
    }
});
