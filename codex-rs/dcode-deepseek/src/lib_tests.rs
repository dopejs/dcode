use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

fn http_client_factory() -> HttpClientFactory {
    HttpClientFactory::new(codex_http_client::OutboundProxyPolicy::ReqwestDefault)
}

fn test_provider(base_url: String) -> ModelProviderInfo {
    ModelProviderInfo {
        base_url: Some(base_url),
        env_key: None,
        ..create_deepseek_provider()
    }
}

#[test]
fn deepseek_provider_is_narrowly_identified() {
    let provider = create_deepseek_provider();
    assert!(is_deepseek_provider(&provider));

    let mut custom = provider;
    custom.env_key_instructions = None;
    assert!(!is_deepseek_provider(&custom));
}

#[test]
fn bundled_model_only_accepts_text() {
    let model = deepseek_flash_model();
    assert_eq!(model.slug, DEEPSEEK_DEFAULT_MODEL);
    assert_eq!(model.input_modalities, vec![InputModality::Text]);
}

#[test]
fn registration_adds_metadata_and_runtime_provider() {
    register();
    let providers =
        codex_model_provider_info::built_in_model_providers(/*openai_base_url*/ None);
    let info = providers
        .get(DEEPSEEK_PROVIDER_ID)
        .expect("registered DeepSeek provider");
    let provider = codex_model_provider::create_model_provider(info.clone(), None);

    assert!(provider.capabilities().api_key_login);
    assert!(provider.capabilities().account_balance);
}

#[tokio::test]
async fn list_models_filters_to_supported_flash_model() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .and(header("authorization", "Bearer deepseek-test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [
                {"id": "deepseek-v4-flash"},
                {"id": "deepseek-v4-pro"}
            ]
        })))
        .mount(&server)
        .await;

    let provider = test_provider(server.uri());
    let auth = auth_provider_from_auth(&CodexAuth::from_api_key("deepseek-test-key"));
    let models = list_models(&provider, auth, http_client_factory())
        .await
        .expect("DeepSeek model list should parse");

    assert_eq!(models, vec![deepseek_flash_model()]);
}

#[tokio::test]
async fn balance_returns_all_currency_components() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/balance"))
        .and(header("authorization", "Bearer deepseek-test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "is_available": true,
            "balance_infos": [{
                "currency": "CNY",
                "total_balance": "110.00",
                "granted_balance": "10.00",
                "topped_up_balance": "100.00"
            }]
        })))
        .mount(&server)
        .await;

    let provider = test_provider(server.uri());
    let actual =
        get_deepseek_balance_with_api_key(&provider, "deepseek-test-key", http_client_factory())
            .await
            .expect("DeepSeek balance should parse");

    assert_eq!(
        actual,
        ProviderBalance {
            is_available: true,
            balance_infos: vec![codex_model_provider::ProviderBalanceInfo {
                currency: "CNY".to_string(),
                total_balance: "110.00".to_string(),
                granted_balance: "10.00".to_string(),
                topped_up_balance: "100.00".to_string(),
            }],
        }
    );
}
