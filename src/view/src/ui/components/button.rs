//! Button component
//!
//! A simple, reusable button with the app's secondary styling.

use gpui::{prelude::*, *};

/// Button size variant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonSize {
    Small,
    Medium,
}

/// Simple button component
pub struct Button {
    label: String,
    size: ButtonSize,
}

impl Button {
    /// Create a new button
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            size: ButtonSize::Medium,
        }
    }

    /// Set button size
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Build the button - returns the div for further chaining
    pub fn build(self) -> Div {
        let (height, padding_x, padding_y) = match self.size {
            ButtonSize::Small => (px(24.), px(12.), px(4.)),
            ButtonSize::Medium => (px(32.), px(16.), px(8.)),
        };

        div()
            .px(padding_x)
            .py(padding_y)
            .h(height)
            .min_w(px(80.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(4.))
            .text_sm()
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(0xcdd6f4))
            .cursor_pointer()
            .bg(rgb(0x45475a))
            .hover(|style| style.bg(rgb(0x585b70)))
            .child(div().child(self.label))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test; // `use gpui::*` brings in gpui's own `test` macro

    #[test]
    fn test_button_creation() {
        let button = Button::new("Test");
        assert_eq!(button.label, "Test");
        assert_eq!(button.size, ButtonSize::Medium);
    }

    #[test]
    fn test_button_builder() {
        let button = Button::new("Test").size(ButtonSize::Small);
        assert_eq!(button.size, ButtonSize::Small);
    }
}
