use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

use crate::key_hint::has_ctrl_or_alt;
use crate::render::renderable::Renderable;

use super::CancellationEvent;
use super::bottom_pane_view::BottomPaneView;
use super::bottom_pane_view::ViewCompletion;
use super::popup_consts::standard_popup_hint_line;

const MAX_API_KEY_BYTES: usize = 4096;

pub(crate) type ApiKeySubmitted = Box<dyn Fn(String) + Send + Sync>;

/// Single-line secret input used by provider login flows.
pub(crate) struct ApiKeyInputView {
    value: String,
    error: Option<&'static str>,
    on_submit: ApiKeySubmitted,
    completion: Option<ViewCompletion>,
}

impl ApiKeyInputView {
    pub(crate) fn new(on_submit: ApiKeySubmitted) -> Self {
        Self {
            value: String::new(),
            error: None,
            on_submit,
            completion: None,
        }
    }

    fn append(&mut self, value: &str) {
        if self.value.len().saturating_add(value.len()) > MAX_API_KEY_BYTES {
            self.error = Some("API key is too long");
            return;
        }
        self.value.push_str(value);
        self.error = None;
    }

    fn submit(&mut self) {
        let api_key = self.value.trim().to_string();
        if api_key.is_empty() {
            self.error = Some("API key cannot be empty");
            return;
        }
        (self.on_submit)(api_key);
        self.completion = Some(ViewCompletion::Accepted);
    }

    fn masked_value(&self, width: u16) -> String {
        let max_symbols = usize::from(width.saturating_sub(/*rhs*/ 4));
        "•".repeat(self.value.chars().count().min(max_symbols))
    }
}

impl BottomPaneView for ApiKeyInputView {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind != KeyEventKind::Press {
            return;
        }
        match key_event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } => {
                self.completion = Some(ViewCompletion::Cancelled);
            }
            KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            } => self.submit(),
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                self.value.pop();
                self.error = None;
            }
            KeyEvent {
                code: KeyCode::Char(character),
                modifiers,
                ..
            } if !has_ctrl_or_alt(modifiers) => self.append(&character.to_string()),
            _ => {}
        }
    }

    fn on_ctrl_c(&mut self) -> CancellationEvent {
        self.completion = Some(ViewCompletion::Cancelled);
        CancellationEvent::Handled
    }

    fn is_complete(&self) -> bool {
        self.completion.is_some()
    }

    fn completion(&self) -> Option<ViewCompletion> {
        self.completion
    }

    fn view_id(&self) -> Option<&'static str> {
        Some("deepseek-login")
    }

    fn handle_paste(&mut self, pasted: String) -> bool {
        let pasted = pasted.trim();
        if pasted.is_empty() {
            return false;
        }
        self.append(pasted);
        true
    }
}

impl Renderable for ApiKeyInputView {
    fn desired_height(&self, _width: u16) -> u16 {
        5
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        if area.is_empty() {
            return;
        }

        let detail = self.error.map_or_else(
            || "The key is validated before it is saved.".dim(),
            ratatui::prelude::Stylize::red,
        );
        let masked = self.masked_value(area.width);
        let input = if masked.is_empty() {
            "Paste or type your API key".dim()
        } else {
            masked.into()
        };
        let lines = vec![
            Line::from(vec!["▌ ".cyan(), "DeepSeek login".bold()]),
            Line::from(vec!["  ".into(), detail]),
            Line::from(vec!["> ".cyan(), input]),
            Line::default(),
            standard_popup_hint_line(),
        ];
        Paragraph::new(lines).render(area, buf);
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        if area.width <= 2 || area.height <= 2 {
            return None;
        }
        let mask_width = self.masked_value(area.width).chars().count() as u16;
        Some((
            area.x.saturating_add(2 + mask_width),
            area.y.saturating_add(2),
        ))
    }
}

#[cfg(test)]
#[path = "api_key_input_view_tests.rs"]
mod tests;
