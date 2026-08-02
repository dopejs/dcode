use pretty_assertions::assert_eq;

use super::*;

#[test]
fn deepseek_login_event_redacts_api_key_from_debug_output() {
    let event = AppEvent::DeepSeekLogin {
        api_key: SensitiveApiKey::new("sk-secret-value".to_string()),
    };

    assert_eq!(
        format!("{event:?}"),
        "DeepSeekLogin { api_key: SensitiveApiKey(***) }"
    );
}
