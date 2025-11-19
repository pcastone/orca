//! Validation utilities for decoding

use crate::constants::{COLON, LIST_ITEM_PREFIX};
use crate::types::{ArrayHeaderInfo, BlankLineInfo, DecodeOptions, Delimiter, Depth, ToonError, ToonResult};

use super::scanner::LineCursor;

/// Assert that the actual count matches the expected count in strict mode
pub fn assert_expected_count(
    actual: usize,
    expected: usize,
    item_type: &str,
    options: &DecodeOptions,
) -> ToonResult<()> {
    if options.strict && actual != expected {
        return Err(ToonError::RangeError(format!(
            "Expected {} {}, but got {}",
            expected, item_type, actual
        )));
    }
    Ok(())
}

/// Validate that there are no extra list items beyond the expected count
pub fn validate_no_extra_list_items(
    cursor: &LineCursor,
    item_depth: Depth,
    expected_count: usize,
) -> ToonResult<()> {
    if let Some(next_line) = cursor.peek() {
        if next_line.depth == item_depth && next_line.content.starts_with(LIST_ITEM_PREFIX) {
            return Err(ToonError::RangeError(format!(
                "Expected {} list array items, but found more",
                expected_count
            )));
        }
    }
    Ok(())
}

/// Validate that there are no extra tabular rows beyond the expected count
pub fn validate_no_extra_tabular_rows(
    cursor: &LineCursor,
    row_depth: Depth,
    header: &ArrayHeaderInfo,
) -> ToonResult<()> {
    if let Some(next_line) = cursor.peek() {
        if next_line.depth == row_depth
            && !next_line.content.starts_with(LIST_ITEM_PREFIX)
            && is_data_row(&next_line.content, header.delimiter)
        {
            return Err(ToonError::RangeError(format!(
                "Expected {} tabular rows, but found more",
                header.length
            )));
        }
    }
    Ok(())
}

/// Validate that there are no blank lines within a specific line range in strict mode
pub fn validate_no_blank_lines_in_range(
    start_line: usize,
    end_line: usize,
    blank_lines: &[BlankLineInfo],
    strict: bool,
    context: &str,
) -> ToonResult<()> {
    if !strict {
        return Ok(());
    }

    // Find blank lines within the range
    let first_blank = blank_lines
        .iter()
        .find(|blank| blank.line_number > start_line && blank.line_number < end_line);

    if let Some(blank) = first_blank {
        return Err(ToonError::syntax(
            blank.line_number,
            format!(
                "Blank lines inside {} are not allowed in strict mode",
                context
            ),
        ));
    }

    Ok(())
}

/// Check if a line is a data row (vs a key-value pair) in a tabular array
fn is_data_row(content: &str, delimiter: Delimiter) -> bool {
    let colon_pos = content.find(COLON);
    let delimiter_pos = content.find(delimiter.as_char());

    // No colon = definitely a data row
    if colon_pos.is_none() {
        return true;
    }

    // Has delimiter and it comes before colon = data row
    if let (Some(d_pos), Some(c_pos)) = (delimiter_pos, colon_pos) {
        if d_pos < c_pos {
            return true;
        }
    }

    // Colon before delimiter or no delimiter = key-value pair
    false
}
