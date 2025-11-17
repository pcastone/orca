//! Form components for TUI with multi-field support

use ratatui::{
    prelude::*,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Field types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Password,
    Number,
    Select,
}

/// Validator function type
pub type Validator = fn(&str) -> Result<(), String>;

/// Form field
#[derive(Debug, Clone)]
pub struct FormField {
    pub label: String,
    pub field_type: FieldType,
    pub value: String,
    pub cursor: usize,
    pub options: Vec<String>, // For select fields
    pub selected_option: usize,
    pub validator: Option<&'static Validator>,
    pub error: Option<String>,
    pub required: bool,
}

impl FormField {
    /// Create a new text field
    pub fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            field_type: FieldType::Text,
            value: String::new(),
            cursor: 0,
            options: Vec::new(),
            selected_option: 0,
            validator: None,
            error: None,
            required: false,
        }
    }

    /// Create a new password field
    pub fn password(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            field_type: FieldType::Password,
            value: String::new(),
            cursor: 0,
            options: Vec::new(),
            selected_option: 0,
            validator: None,
            error: None,
            required: false,
        }
    }

    /// Create a new number field
    pub fn number(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            field_type: FieldType::Number,
            value: String::new(),
            cursor: 0,
            options: Vec::new(),
            selected_option: 0,
            validator: None,
            error: None,
            required: false,
        }
    }

    /// Create a new select field
    pub fn select(label: impl Into<String>, options: Vec<String>) -> Self {
        Self {
            label: label.into(),
            field_type: FieldType::Select,
            value: options.first().map(|s| s.clone()).unwrap_or_default(),
            cursor: 0,
            options,
            selected_option: 0,
            validator: None,
            error: None,
            required: false,
        }
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Add validator
    pub fn with_validator(mut self, validator: &'static Validator) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Add character to field (for text, password, number)
    pub fn add_char(&mut self, c: char) {
        match self.field_type {
            FieldType::Text | FieldType::Password => {
                self.value.insert(self.cursor, c);
                self.cursor += 1;
                self.error = None;
            }
            FieldType::Number => {
                if c.is_numeric() || c == '-' || c == '.' {
                    self.value.insert(self.cursor, c);
                    self.cursor += 1;
                    self.error = None;
                }
            }
            FieldType::Select => {}
        }
    }

    /// Backspace in field
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.value.remove(self.cursor - 1);
            self.cursor -= 1;
            self.error = None;
        }
    }

    /// Move cursor left
    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn cursor_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Move to next option (for select fields)
    pub fn next_option(&mut self) {
        if !self.options.is_empty() {
            self.selected_option = (self.selected_option + 1) % self.options.len();
            self.value = self.options[self.selected_option].clone();
            self.error = None;
        }
    }

    /// Move to previous option (for select fields)
    pub fn prev_option(&mut self) {
        if !self.options.is_empty() {
            self.selected_option = if self.selected_option > 0 {
                self.selected_option - 1
            } else {
                self.options.len() - 1
            };
            self.value = self.options[self.selected_option].clone();
            self.error = None;
        }
    }

    /// Validate field value
    pub fn validate(&mut self) -> bool {
        // Check required
        if self.required && self.value.is_empty() {
            self.error = Some(format!("{} is required", self.label));
            return false;
        }

        // Run validator if present
        if let Some(validator) = self.validator {
            if !self.value.is_empty() {
                match validator(&self.value) {
                    Ok(_) => {
                        self.error = None;
                        true
                    }
                    Err(err) => {
                        self.error = Some(err);
                        false
                    }
                }
            } else {
                self.error = None;
                true
            }
        } else {
            self.error = None;
            true
        }
    }

    /// Get field value
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Multi-field form
#[derive(Debug, Clone)]
pub struct Form {
    pub title: String,
    pub fields: Vec<FormField>,
    pub focused_field: usize,
    pub submit_label: String,
    pub cancel_label: String,
}

impl Form {
    /// Create a new form
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            fields: Vec::new(),
            focused_field: 0,
            submit_label: "Submit".to_string(),
            cancel_label: "Cancel".to_string(),
        }
    }

    /// Add field to form
    pub fn add_field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    /// Set submit button label
    pub fn submit_label(mut self, label: impl Into<String>) -> Self {
        self.submit_label = label.into();
        self
    }

    /// Set cancel button label
    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Get focused field
    pub fn focused_field(&self) -> Option<&FormField> {
        self.fields.get(self.focused_field)
    }

    /// Get mutable focused field
    pub fn focused_field_mut(&mut self) -> Option<&mut FormField> {
        self.fields.get_mut(self.focused_field)
    }

    /// Move to next field
    pub fn next_field(&mut self) {
        if !self.fields.is_empty() {
            self.focused_field = (self.focused_field + 1) % self.fields.len();
        }
    }

    /// Move to previous field
    pub fn prev_field(&mut self) {
        if !self.fields.is_empty() {
            self.focused_field = if self.focused_field > 0 {
                self.focused_field - 1
            } else {
                self.fields.len() - 1
            };
        }
    }

    /// Validate all fields
    pub fn validate(&mut self) -> bool {
        let mut all_valid = true;
        for field in &mut self.fields {
            if !field.validate() {
                all_valid = false;
            }
        }
        all_valid
    }

    /// Get form data as key-value pairs
    pub fn get_data(&self) -> Vec<(String, String)> {
        self.fields
            .iter()
            .map(|f| (f.label.clone(), f.value.clone()))
            .collect()
    }

    /// Clear all fields
    pub fn clear(&mut self) {
        for field in &mut self.fields {
            field.value.clear();
            field.cursor = 0;
            field.error = None;
        }
    }
}

/// Render a form
pub fn render_form(f: &mut Frame, form: &Form, area: Rect) {
    // Calculate layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Length(3))
                .take(form.fields.len())
                .chain(std::iter::once(Constraint::Min(2)))
                .collect::<Vec<_>>(),
        )
        .split(area);

    // Render each field
    for (idx, field) in form.fields.iter().enumerate() {
        if let Some(area) = chunks.get(idx) {
            render_form_field(f, field, *area, idx == form.focused_field);
        }
    }

    // Render buttons at bottom
    if let Some(button_area) = chunks.last() {
        render_form_buttons(f, form, *button_area);
    }
}

/// Render single form field
fn render_form_field(f: &mut Frame, field: &FormField, area: Rect, focused: bool) {
    let block_style = if focused {
        Style::default().fg(Color::Cyan).bold()
    } else if field.error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::White)
    };

    let block = Block::default()
        .title(field.label.as_str())
        .borders(Borders::ALL)
        .style(block_style);

    // Create a rect for content
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: 1,
    };

    // Render field value with cursor or password
    let display_text = match field.field_type {
        FieldType::Password => {
            let mut text = String::new();
            for (i, _) in field.value.chars().enumerate() {
                if i == field.cursor {
                    text.push('│');
                }
                text.push('•');
            }
            if field.cursor == field.value.len() {
                text.push('│');
            }
            text
        }
        FieldType::Select => {
            format!("{} (press ↑↓ to change)", field.value)
        }
        _ => {
            let mut text = String::new();
            for (i, c) in field.value.chars().enumerate() {
                if i == field.cursor {
                    text.push('│');
                }
                text.push(c);
            }
            if field.cursor == field.value.len() {
                text.push('│');
            }
            text
        }
    };

    let content_style = if field.error.is_some() {
        Style::default().fg(Color::Red)
    } else if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    };

    let content = Paragraph::new(display_text).style(content_style);

    // Render block and content
    f.render_widget(block, area);
    f.render_widget(content, inner);

    // Render error if present
    if let Some(error) = &field.error {
        let error_area = Rect {
            x: area.x + 1,
            y: area.y + 2,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let error_text = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        f.render_widget(error_text, error_area);
    }
}

/// Render form buttons
fn render_form_buttons(f: &mut Frame, form: &Form, area: Rect) {
    let button_text = format!("  [{}]  [{}]  ", form.submit_label, form.cancel_label);
    let buttons = Paragraph::new(button_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center);
    f.render_widget(buttons, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_field_text() {
        let mut field = FormField::text("Name");
        field.add_char('J');
        field.add_char('o');
        assert_eq!(field.value(), "Jo");
        assert_eq!(field.cursor, 2);
    }

    #[test]
    fn test_form_field_number() {
        let mut field = FormField::number("Age");
        field.add_char('2');
        field.add_char('5');
        assert_eq!(field.value(), "25");

        // Non-numeric characters should be ignored
        field.add_char('a');
        assert_eq!(field.value(), "25");
    }

    #[test]
    fn test_form_field_validation() {
        let mut field = FormField::text("Email").required();
        assert!(!field.validate()); // Empty required field

        field.value = "test@example.com".to_string();
        assert!(field.validate()); // Now valid
    }

    #[test]
    fn test_form_multiple_fields() {
        let form = Form::new("Contact")
            .add_field(FormField::text("Name").required())
            .add_field(FormField::text("Email").required())
            .add_field(FormField::text("Message"));

        assert_eq!(form.fields.len(), 3);
    }
}
