use std::sync::mpsc::Receiver;

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::*;

#[test]
fn masks_api_key_and_submits_original_value() {
    let (mut view, submitted) = api_key_view();
    view.handle_paste("sk-secret-value".to_string());

    let area = Rect::new(0, 0, 48, 5);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);

    insta::assert_snapshot!("deepseek_api_key_input_masked", format!("{buffer:?}"));
    view.handle_key_event(KeyEvent::from(KeyCode::Enter));
    assert_eq!(submitted.try_recv(), Ok("sk-secret-value".to_string()));
    assert!(view.is_complete());
}

#[test]
fn empty_api_key_stays_open_and_shows_error() {
    let (mut view, submitted) = api_key_view();
    view.handle_key_event(KeyEvent::from(KeyCode::Enter));

    assert!(submitted.try_recv().is_err());
    assert!(!view.is_complete());

    let area = Rect::new(0, 0, 48, 5);
    let mut buffer = Buffer::empty(area);
    view.render(area, &mut buffer);
    insta::assert_snapshot!("deepseek_api_key_input_empty_error", format!("{buffer:?}"));
}

fn api_key_view() -> (ApiKeyInputView, Receiver<String>) {
    let (submitted, receiver) = std::sync::mpsc::channel();
    let view = ApiKeyInputView::new(Box::new(move |api_key| {
        submitted.send(api_key).expect("submit API key");
    }));
    (view, receiver)
}
