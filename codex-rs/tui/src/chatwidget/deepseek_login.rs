use super::ChatWidget;
use crate::app_event::AppEvent;
use crate::app_event::SensitiveApiKey;
use crate::bottom_pane::ApiKeyInputView;

impl ChatWidget {
    pub(super) fn open_deepseek_login(&mut self) {
        if !self.config.model_provider.is_deepseek() {
            self.add_error_message(
                "The /login command is only available when model_provider is deepseek.".to_string(),
            );
            return;
        }

        let app_event_tx = self.app_event_tx.clone();
        let view = ApiKeyInputView::new(Box::new(move |api_key| {
            app_event_tx.send(AppEvent::DeepSeekLogin {
                api_key: SensitiveApiKey::new(api_key),
            });
        }));
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }
}
