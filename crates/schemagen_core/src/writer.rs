/// Small code writer that handles indentation, so emitters don't have to deal with that.
/// Indentation is configurable via the `with_indent` option, defaults to 2 spaces.
#[derive(Debug, Default)]
pub struct Writer {
    buffer: String,
    level: usize,
    indent: String,
}

impl Writer {
    pub fn new() -> Self {
        Writer::with_indent("  ")
    }

    /// Set indent to use per level, can be spaces or a literal tab character `\t`
    pub fn with_indent(indent: &str) -> Self {
        Writer {
            buffer: String::new(),
            level: 0,
            indent: indent.to_string(),
        }
    }

    /// Write one line at the current indentation, terminated by a newline.
    pub fn line(&mut self, text: &str) {
        for _ in 0..self.level {
            self.buffer.push_str(&self.indent);
        }
        self.buffer.push_str(text);
        self.buffer.push('\n');
    }

    /// Write `open`, run `body` one level deeper, then write `close` back at this level.
    ///
    /// # Example
    ///
    /// ```
    /// use schemagen_core::writer::Writer;
    ///
    /// let mut writer = Writer::new();
    /// writer.block("interface User {", "}", |writer| {
    ///     writer.line("name: string;");
    /// });
    /// assert_eq!(writer.finish(), "interface User {\n  name: string;\n}\n");
    /// ```
    pub fn block(&mut self, open: &str, close: &str, body: impl FnOnce(&mut Self)) {
        self.line(open);
        self.level += 1;
        body(self);
        self.level -= 1;
        self.line(close);
    }

    pub fn finish(self) -> String {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_indents_its_body_and_restores_the_level() {
        let mut writer = Writer::new();
        writer.block("interface X {", "}", |writer| {
            writer.line("id: number;");
        });
        writer.line("const after = 1;");
        assert_eq!(
            writer.finish(),
            "interface X {\n  id: number;\n}\nconst after = 1;\n"
        );
    }

    #[test]
    fn a_custom_indent_unit_is_used() {
        let mut writer = Writer::with_indent("\t");
        writer.block("{", "}", |writer| writer.line("x"));
        assert_eq!(writer.finish(), "{\n\tx\n}\n");
    }
}
