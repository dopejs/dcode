//! Mouse routing for the DCode full-screen TUI.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;

use super::*;

const MOUSE_WHEEL_ROWS: usize = 3;

impl App {
    pub(super) async fn handle_mouse_event(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        mouse_event: MouseEvent,
    ) -> Result<()> {
        let key_code = match mouse_event.kind {
            MouseEventKind::ScrollUp => KeyCode::Up,
            MouseEventKind::ScrollDown => KeyCode::Down,
            MouseEventKind::ScrollLeft
            | MouseEventKind::ScrollRight
            | MouseEventKind::Down(_)
            | MouseEventKind::Up(_)
            | MouseEventKind::Drag(_)
            | MouseEventKind::Moved => return Ok(()),
        };

        if !self.chat_widget.no_modal_or_popup_active() {
            self.chat_widget
                .handle_key_event(KeyEvent::new(key_code, KeyModifiers::NONE));
            return Ok(());
        }

        if key_code == KeyCode::Up {
            self.open_transcript_overlay(tui);
            for _ in 0..MOUSE_WHEEL_ROWS {
                self.handle_backtrack_overlay_event(
                    tui,
                    app_server,
                    TuiEvent::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
                )
                .await?;
            }
        }
        Ok(())
    }
}
