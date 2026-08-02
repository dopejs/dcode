use codex_model_provider_info::DEEPSEEK_DEFAULT_MODEL;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_reasoning_item;
use core_test_support::responses::ev_reasoning_item_added;
use core_test_support::responses::ev_reasoning_text_delta;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deepseek_responses_provider_streams_reasoning_and_uses_flash_model() {
    skip_if_no_network!();

    let server = start_mock_server().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [
                {"id": DEEPSEEK_DEFAULT_MODEL, "object": "model", "owned_by": "deepseek"},
                {"id": "deepseek-v4-pro", "object": "model", "owned_by": "deepseek"}
            ]
        })))
        .mount(&server)
        .await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_reasoning_item_added("reasoning-1", &[]),
            ev_reasoning_text_delta("checking the repository"),
            ev_reasoning_item("reasoning-1", &[], &["checking the repository"]),
            ev_completed("resp-1"),
        ]),
    )
    .await;

    let mut provider = ModelProviderInfo::create_deepseek_provider();
    provider.base_url = Some(format!("{}/v1", server.uri()));
    provider.env_key = Some("PATH".to_string());
    let test = test_codex()
        .with_config(move |config| {
            config.model = Some(DEEPSEEK_DEFAULT_MODEL.to_string());
            config.model_provider_id = "deepseek".to_string();
            config.model_provider = provider;
            config.show_raw_agent_reasoning = true;
        })
        .build_with_auto_env(&server)
        .await
        .expect("build DeepSeek test Codex");

    test.codex
        .submit(Op::UserInput {
            items: vec![UserInput::Text {
                text: "inspect this repository".to_string(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .expect("submit DeepSeek turn");

    let mut reasoning = String::new();
    loop {
        match wait_for_event(&test.codex, |_| true).await {
            EventMsg::ReasoningRawContentDelta(event) => reasoning.push_str(&event.delta),
            EventMsg::TurnComplete(_) => break,
            _ => {}
        }
    }

    assert_eq!(reasoning, "checking the repository");
    let request = response_mock.single_request();
    let body = request.body_json();
    assert_eq!(body["model"], DEEPSEEK_DEFAULT_MODEL);
    assert_eq!(body["store"], false);
    assert_eq!(body["reasoning"]["effort"], "high");
    let tools = body["tools"]
        .as_array()
        .expect("DeepSeek request should include coding tools");
    assert!(
        tools
            .iter()
            .any(|tool| { tool["type"] == "function" && tool["name"] == "exec_command" })
    );
    assert!(
        tools
            .iter()
            .any(|tool| { tool["type"] == "custom" && tool["name"] == "apply_patch" })
    );
}
