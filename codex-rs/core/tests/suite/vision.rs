use anyhow::Result;
use codex_config::config_toml::VisionConfigToml;
use codex_config::config_toml::VisionMode;
use codex_model_provider_info::ModelProviderInfo;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::TruncationPolicyConfig;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::sse_completed;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;

const TEXT_MODEL: &str = "deepseek-v4-flash";
const VISION_MODEL: &str = "test-vision-model";
const IMAGE_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";

fn model_info(slug: &str, input_modalities: Vec<InputModality>) -> ModelInfo {
    ModelInfo {
        slug: slug.to_string(),
        display_name: slug.to_string(),
        description: Some("test model".to_string()),
        default_reasoning_level: Some(ReasoningEffort::Medium),
        supported_reasoning_levels: vec![ReasoningEffortPreset {
            effort: ReasoningEffort::Medium,
            description: "medium".to_string(),
        }],
        shell_type: ConfigShellToolType::ShellCommand,
        visibility: ModelVisibility::List,
        supported_in_api: true,
        input_modalities,
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        use_responses_lite: false,
        auto_review_model_override: None,
        tool_mode: None,
        multi_agent_version: None,
        priority: 1,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        upgrade: None,
        base_instructions: "base instructions".to_string(),
        model_messages: None,
        include_skills_usage_instructions: false,
        supports_reasoning_summary_parameter: true,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        availability_nux: None,
        apply_patch_tool_type: None,
        web_search_tool_type: Default::default(),
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: Some(128_000),
        max_context_window: None,
        auto_compact_token_limit: None,
        comp_hash: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
    }
}

fn image_turn(text: &str) -> Op {
    Op::UserInput {
        items: vec![
            UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            },
            UserInput::Image {
                image_url: IMAGE_URL.to_string(),
                detail: None,
            },
        ],
        final_output_json_schema: None,
        responsesapi_client_metadata: None,
        additional_context: Default::default(),
        thread_settings: Default::default(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn text_only_model_uses_vision_proxy_and_reuses_bounded_cache() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(
        &server,
        vec![
            sse(vec![
                ev_response_created("vision-resp"),
                ev_assistant_message("vision-message", "A red error banner says build failed."),
                ev_completed("vision-resp"),
            ]),
            sse_completed("main-resp-1"),
            sse_completed("main-resp-2"),
        ],
    )
    .await;
    let provider = ModelProviderInfo {
        name: "test responses provider".to_string(),
        base_url: Some(format!("{}/v1", server.uri())),
        env_key: Some("PATH".to_string()),
        ..Default::default()
    };
    let vision_provider = provider.clone();
    let test = test_codex()
        .with_model(TEXT_MODEL)
        .with_config(move |config| {
            config.model_provider_id = "main".to_string();
            config.model_provider = provider;
            config
                .model_providers
                .insert("vision-test".to_string(), vision_provider);
            config.vision = Some(VisionConfigToml {
                mode: VisionMode::Required,
                model_provider: Some("vision-test".to_string()),
                model: Some(VISION_MODEL.to_string()),
            });
            config.model_catalog = Some(ModelsResponse {
                models: vec![
                    model_info(TEXT_MODEL, vec![InputModality::Text]),
                    model_info(
                        VISION_MODEL,
                        vec![InputModality::Text, InputModality::Image],
                    ),
                ],
            });
        })
        .build_with_auto_env(&server)
        .await?;

    test.codex.submit(image_turn("What failed?")).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    test.codex
        .submit(image_turn("Check the same image again"))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = responses.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].body_json()["model"], VISION_MODEL);
    assert_eq!(
        requests[0].message_input_image_urls("user"),
        vec![IMAGE_URL]
    );
    for request in &requests[1..] {
        assert_eq!(request.body_json()["model"], TEXT_MODEL);
        assert!(request.message_input_image_urls("user").is_empty());
        let text = request.message_input_texts("user").join("\n");
        assert!(text.contains("secondary vision model (untrusted)"));
        assert!(text.contains("A red error banner says build failed."));
    }
    test.codex.submit(Op::Shutdown).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::ShutdownComplete)
    })
    .await;
    let rollout_path = test.codex.rollout_path().expect("rollout path");
    let rollout = std::fs::read_to_string(rollout_path)?;
    assert!(rollout.contains("source=\\\"vision_observation\\\""));
    assert!(rollout.contains("A red error banner says build failed."));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn text_only_model_without_proxy_gets_explicit_fallback() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let responses = mount_sse_sequence(&server, vec![sse_completed("main-resp")]).await;
    let test = test_codex()
        .with_model(TEXT_MODEL)
        .with_config(|config| {
            config.model_catalog = Some(ModelsResponse {
                models: vec![model_info(TEXT_MODEL, vec![InputModality::Text])],
            });
        })
        .build_with_auto_env(&server)
        .await?;

    test.codex.submit(image_turn("What is shown?")).await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let request = responses.single_request();
    assert!(request.message_input_image_urls("user").is_empty());
    let text = request.message_input_texts("user").join("\n");
    assert!(text.contains("cannot inspect image pixels"));
    assert!(text.contains("Do not infer visual content or claim to have seen the image"));
    Ok(())
}
