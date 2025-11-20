//! Line scanner for TOON input

use crate::types::{BlankLineInfo, Depth, ParsedLine, ToonError, ToonResult};

/// Result of scanning TOON input into parsed lines
pub struct ScanResult {
    pub lines: Vec<ParsedLine>,
    pub blank_lines: Vec<BlankLineInfo>,
}

/// A cursor for iterating through parsed lines
pub struct LineCursor {
    lines: Vec<ParsedLine>,
    index: usize,
    blank_lines: Vec<BlankLineInfo>,
}

impl LineCursor {
    pub fn new(lines: Vec<ParsedLine>, blank_lines: Vec<BlankLineInfo>) -> Self {
        Self {
            lines,
            index: 0,
            blank_lines,
        }
    }

    pub fn get_blank_lines(&self) -> &[BlankLineInfo] {
        &self.blank_lines
    }

    pub fn peek(&self) -> Option<&ParsedLine> {
        self.lines.get(self.index)
    }

    pub fn next(&mut self) -> Option<&ParsedLine> {
        let line = self.lines.get(self.index);
        if line.is_some() {
            self.index += 1;
        }
        line
    }

    pub fn current(&self) -> Option<&ParsedLine> {
        if self.index > 0 {
            self.lines.get(self.index - 1)
        } else {
            None
        }
    }

    pub fn advance(&mut self) {
        self.index += 1;
    }

    pub fn at_end(&self) -> bool {
        self.index >= self.lines.len()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn peek_at_depth(&self, target_depth: Depth) -> Option<&ParsedLine> {
        let line = self.peek()?;
        if line.depth == target_depth {
            Some(line)
        } else {
            None
        }
    }
}

/// Parse a TOON string into parsed lines
pub fn to_parsed_lines(source: &str, indent_size: usize, strict: bool) -> ToonResult<ScanResult> {
    if source.trim().is_empty() {
        return Ok(ScanResult {
            lines: Vec::new(),
            blank_lines: Vec::new(),
        });
    }

    let lines: Vec<&str> = source.split('\n').collect();
    let mut parsed = Vec::new();
    let mut blank_lines = Vec::new();

    for (i, raw) in lines.iter().enumerate() {
        let line_number = i + 1;
        let mut indent = 0;
        let bytes = raw.as_bytes();

        while indent < bytes.len() && bytes[indent] == b' ' {
            indent += 1;
        }

        let content = &raw[indent..];

        // Track blank lines
        if content.trim().is_empty() {
            let depth = compute_depth_from_indent(indent, indent_size);
            blank_lines.push(BlankLineInfo {
                line_number,
                indent,
                depth,
            });
            continue;
        }

        let depth = compute_depth_from_indent(indent, indent_size);

        // Strict mode validation
        if strict {
            // Find the full leading whitespace region
            let mut whitespace_end_index = 0;
            while whitespace_end_index < bytes.len()
                && (bytes[whitespace_end_index] == b' ' || bytes[whitespace_end_index] == b'\t')
            {
                whitespace_end_index += 1;
            }

            // Check for tabs in leading whitespace
            if raw[..whitespace_end_index].contains('\t') {
                return Err(ToonError::syntax(
                    line_number,
                    "Tabs are not allowed in indentation in strict mode",
                ));
            }

            // Check for exact multiples of indent_size
            if indent > 0 && indent % indent_size != 0 {
                return Err(ToonError::syntax(
                    line_number,
                    format!(
                        "Indentation must be exact multiple of {}, but found {} spaces",
                        indent_size, indent
                    ),
                ));
            }
        }

        parsed.push(ParsedLine {
            raw: raw.to_string(),
            indent,
            content: content.to_string(),
            depth,
            line_number,
        });
    }

    Ok(ScanResult {
        lines: parsed,
        blank_lines,
    })
}

fn compute_depth_from_indent(indent_spaces: usize, indent_size: usize) -> Depth {
    indent_spaces / indent_size
}
