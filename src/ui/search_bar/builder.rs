use crate::{loader::utils::ExecVariable, ui::search_bar::TextInput};
use gpui::{Context, SharedString};

pub struct TextInputBuilder {
    scope: Option<&'static str>,
    placeholder: SharedString,
    content: SharedString,
    variable: Option<ExecVariable>,
    ghost_text: Option<String>,
}

#[allow(dead_code)]
impl TextInputBuilder {
    pub fn new() -> Self {
        Self {
            scope: None,
            placeholder: SharedString::default(),
            content: SharedString::default(),
            variable: None,
            ghost_text: None,
        }
    }
    pub fn scope(mut self, scope: &'static str) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn content(mut self, content: impl Into<SharedString>) -> Self {
        self.content = content.into();
        self
    }

    pub fn variable(mut self, variable: ExecVariable) -> Self {
        self.variable = Some(variable);
        self
    }

    pub fn ghost_text(mut self, text: impl Into<String>) -> Self {
        self.ghost_text = Some(text.into());
        self
    }

    pub fn build(self, cx: &mut Context<TextInput>) -> TextInput {
        TextInput {
            scope: self.scope,
            focus_handle: cx.focus_handle(),
            content: self.content,
            placeholder: self.placeholder,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            variable: self.variable,
            ghost_text: self.ghost_text,
            _sub: None,
        }
    }
}
