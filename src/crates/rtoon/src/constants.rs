//! Constants used throughout the TOON library

// List markers
pub const LIST_ITEM_MARKER: char = '-';
pub const LIST_ITEM_PREFIX: &str = "- ";

// Structural characters
pub const COMMA: char = ',';
pub const COLON: char = ':';
pub const SPACE: char = ' ';
pub const PIPE: char = '|';
pub const DOT: char = '.';

// Brackets and braces
pub const OPEN_BRACKET: char = '[';
pub const CLOSE_BRACKET: char = ']';
pub const OPEN_BRACE: char = '{';
pub const CLOSE_BRACE: char = '}';

// Literals
pub const NULL_LITERAL: &str = "null";
pub const TRUE_LITERAL: &str = "true";
pub const FALSE_LITERAL: &str = "false";

// Escape characters
pub const BACKSLASH: char = '\\';
pub const DOUBLE_QUOTE: char = '"';
pub const NEWLINE: char = '\n';
pub const CARRIAGE_RETURN: char = '\r';
pub const TAB: char = '\t';

/// Delimiter types for array values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delimiter {
    Comma,
    Tab,
    Pipe,
}

impl Delimiter {
    pub fn as_char(&self) -> char {
        match self {
            Delimiter::Comma => COMMA,
            Delimiter::Tab => TAB,
            Delimiter::Pipe => PIPE,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Delimiter::Comma => ",",
            Delimiter::Tab => "\t",
            Delimiter::Pipe => "|",
        }
    }
}

impl Default for Delimiter {
    fn default() -> Self {
        Delimiter::Comma
    }
}

pub const DEFAULT_DELIMITER: Delimiter = Delimiter::Comma;
