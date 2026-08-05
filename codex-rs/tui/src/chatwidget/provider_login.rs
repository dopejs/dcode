use super::ChatWidget;
use crate::app_event::AppEvent;
use crate::app_event::SensitiveApiKey;
use crate::bottom_pane::ApiKeyInputView;

impl ChatWidget {
    pub(super) fn open_provider_login(&mut self) {
        let provider = codex_model_provider::create_model_provider(
            self.config.model_provider.clone(),
            /*auth_manager*/ None,
        );
        if !provider.capabilities().api_key_login {
            self.add_error_message(
                "The active model provider does not support interactive API key login.".to_string(),
            );
            return;
        }

        let provider_name = self.config.model_provider.name.clone();
        let app_event_tx = self.app_event_tx.clone();
        let view = ApiKeyInputView::new(
            provider_name,
            Box::new(move |api_key| {
                app_event_tx.send(AppEvent::ProviderApiKeyLogin {
                    api_key: SensitiveApiKey::new(api_key),
                });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
        self.request_redraw();
    }
}
