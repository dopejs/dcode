use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::manager::ModelsEndpointClient;
use codex_models_manager::manager::ModelsManager;
use codex_models_manager::manager::RefreshStrategy;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::header;
use wiremock::matchers::method;
use wiremock::matchers::path;

use super::DeepSeekModelsEndpoint;
use super::DeepSeekModelsManager;
use super::deepseek_flash_model;
use super::get_balance;
use crate::auth::resolve_provider_auth;
use crate::provider::ProviderBalance;
use crate::provider::ProviderBalanceInfo;

fn http_client_factory() -> HttpClientFactory {
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
}

fn test_provider(base_url: String) -> ModelProviderInfo {
    ModelProviderInfo {
        base_url: Some(base_url),
        env_key: None,
        experimental_bearer_token: Some("deepseek-test-key".to_string()),
        ..ModelProviderInfo::create_deepseek_provider()
    }
}

#[tokio::test]
async fn model_manager_forces_flash_even_when_another_model_is_configured() {
    let manager = DeepSeekModelsManager::new(
        ModelProviderInfo::create_deepseek_provider(),
        /*auth_manager*/ None,
        /*config_model_catalog*/ None,
    );

    let model = manager
        .get_default_model(
            &Some("deepseek-v4-pro".to_string()),
            /*allow_provider_model_fallback*/ false,
            RefreshStrategy::Offline,
            http_client_factory(),
        )
        .await;

    assert_eq!(model, "deepseek-v4-flash");
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
                {"id": "deepseek-v4-flash", "object": "model", "owned_by": "deepseek"},
                {"id": "deepseek-v4-pro", "object": "model", "owned_by": "deepseek"}
            ]
        })))
        .mount(&server)
        .await;

    let endpoint = DeepSeekModelsEndpoint::new(test_provider(server.uri()), None);
    let (models, etag) = endpoint
        .list_models("test-client", http_client_factory())
        .await
        .expect("DeepSeek model list should parse");

    assert_eq!(models, vec![deepseek_flash_model()]);
    assert_eq!(etag, None);
}

#[tokio::test]
async fn get_balance_returns_all_currency_components() {
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
    let auth = resolve_provider_auth(/*auth*/ None, &provider).expect("test auth should resolve");
    let actual = get_balance(&provider, auth, http_client_factory())
        .await
        .expect("DeepSeek balance should parse");

    assert_eq!(
        actual,
        ProviderBalance {
            is_available: true,
            balance_infos: vec![ProviderBalanceInfo {
                currency: "CNY".to_string(),
                total_balance: "110.00".to_string(),
                granted_balance: "10.00".to_string(),
                topped_up_balance: "100.00".to_string(),
            }],
        }
    );
}
