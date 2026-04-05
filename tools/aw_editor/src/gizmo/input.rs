//! Numeric input UI widget.

/// Numeric input widget for precise transform values.
#[derive(Debug, Clone, Default)]
pub struct NumericInput {
    buffer: String,
}

impl NumericInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_char(&mut self, c: char) {
        self.buffer.push(c);
    }

    pub fn pop_char(&mut self) {
        self.buffer.pop();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn parse(&self) -> Option<f32> {
        self.buffer.parse().ok()
    }

    pub fn text(&self) -> &str {
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_input_parse() {
        let mut input = NumericInput::new();
        input.push_char('5');
        input.push_char('.');
        input.push_char('2');

        assert_eq!(input.parse(), Some(5.2));
    }

    #[test]
    fn test_numeric_input_pop() {
        let mut input = NumericInput::new();
        input.push_char('5');
        input.push_char('2');
        input.pop_char();

        assert_eq!(input.text(), "5");
    }
}
