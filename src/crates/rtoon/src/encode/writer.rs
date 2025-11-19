//! Line writer for TOON output

use crate::constants::LIST_ITEM_PREFIX;
use crate::types::Depth;

/// A writer that manages indented lines for TOON output
pub struct LineWriter {
    lines: Vec<String>,
    indentation_string: String,
}

impl LineWriter {
    pub fn new(indent_size: usize) -> Self {
        Self {
            lines: Vec::new(),
            indentation_string: " ".repeat(indent_size),
        }
    }

    pub fn push(&mut self, depth: Depth, content: &str) {
        let indent = self.indentation_string.repeat(depth);
        self.lines.push(format!("{}{}", indent, content));
    }

    pub fn push_list_item(&mut self, depth: Depth, content: &str) {
        self.push(depth, &format!("{}{}", LIST_ITEM_PREFIX, content));
    }

    pub fn to_string(&self) -> String {
        self.lines.join("\n")
    }
}
