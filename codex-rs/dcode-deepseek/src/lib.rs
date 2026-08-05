use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use codex_api::SharedAuthProvider;
use codex_http_client::ClientRouteClass;
use codex_http_client::HttpClientFactory;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_login::default_client::create_client_for_route_async;
use codex_model_provider::DownstreamModelProviderFactory;
use codex_model_provider::ModelProvider;
use codex_model_provider::ModelProviderFuture;
use codex_model_provider::ProviderAccountResult;
use codex_model_provider::ProviderAccountState;
use codex_model_provider::ProviderBalance;
use codex_model_provider::ProviderCapabilities;
use codex_model_provider::RemoteCompactionSupport;
use codex_model_provider::SharedModelProvider;
use codex_model_provider::auth_provider_from_auth;
use codex_model_provider::register_downstream_model_provider_factory;
use codex_model_provider::unauthenticated_auth_provider;
use codex_model_provider_info::ModelProviderInfo;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::register_downstream_built_in_provider;
use codex_models_manager::bundled_models_response;
use codex_models_manager::collaboration_mode_presets::builtin_collaboration_mode_presets;
use codex_models_manager::manager::ModelsManager;
use codex_models_manager::manager::ModelsManagerFuture;
use codex_models_manager::manager::RefreshStrategy;
use codex_models_manager::manager::SharedModelsManager;
use codex_protocol::account::ProviderAccount;
use codex_protocol::config_types::CollaborationModeMask;
use codex_protocol::error::CodexErrorDetails;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::ModelsResponse;
use codex_protocol::openai_models::ReasoningEffort;
use codex_protocol::openai_models::ReasoningEffortPreset;
use codex_protocol::openai_models::WebSearchToolType;
use codex_protocol::protocol::MultiAgentVersion;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio::sync::TryLockError;

pub const DEEPSEEK_PROVIDER_ID: &str = "deepseek";
pub const DEEPSEEK_DEFAULT_MODEL: &str = "deepseek-v4-flash";
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com";
const DEEPSEEK_PROVIDER_NAME: &str = "DeepSeek";
const DEEPSEEK_API_KEY_INSTRUCTIONS: &str = "Create an API key at https://platform.deepseek.com/api_keys and export it as DEEPSEEK_API_KEY.";
const DEEPSEEK_CONTEXT_WINDOW: i64 = 1_048_576;
const DEEPSEEK_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ERROR_BODY_BYTES: usize = 4 * 1024;

/// Registers DCode's provider metadata and runtime implementation.
pub fn register() {
    register_downstream_built_in_provider(DEEPSEEK_PROVIDER_ID, create_deepseek_provider);
    register_downstream_model_provider_factory(Arc::new(DeepSeekProviderFactory));
}

pub fn create_deepseek_provider() -> ModelProviderInfo {
    ModelProviderInfo {
        name: DEEPSEEK_PROVIDER_NAME.into(),
        base_url: Some(DEEPSEEK_BASE_URL.into()),
        env_key: Some("DEEPSEEK_API_KEY".into()),
        env_key_instructions: Some(DEEPSEEK_API_KEY_INSTRUCTIONS.into()),
        experimental_bearer_token: None,
        auth: None,
        aws: None,
        wire_api: WireApi::Responses,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        requires_openai_auth: false,
        supports_websockets: false,
        supports_standalone_web_search: false,
    }
}

fn is_deepseek_provider(info: &ModelProviderInfo) -> bool {
    info.name == DEEPSEEK_PROVIDER_NAME
        && info.env_key_instructions.as_deref() == Some(DEEPSEEK_API_KEY_INSTRUCTIONS)
}

#[derive(Debug)]
struct DeepSeekProviderFactory;

impl DownstreamModelProviderFactory for DeepSeekProviderFactory {
    fn id(&self) -> &'static str {
        DEEPSEEK_PROVIDER_ID
    }

    fn matches(&self, provider_info: &ModelProviderInfo) -> bool {
        is_deepseek_provider(provider_info)
    }

    fn create(
        &self,
        provider_info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
    ) -> SharedModelProvider {
        Arc::new(DeepSeekModelProvider::new(provider_info, auth_manager))
    }
}

#[derive(Debug, Deserialize)]
struct DeepSeekModelsResponse {
    data: Vec<DeepSeekModel>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekModel {
    id: String,
}

#[derive(Clone, Debug)]
struct DeepSeekModelProvider {
    info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl DeepSeekModelProvider {
    fn new(info: ModelProviderInfo, auth_manager: Option<Arc<AuthManager>>) -> Self {
        Self { info, auth_manager }
    }

    async fn resolved_auth(&self) -> CodexResult<SharedAuthProvider> {
        Ok(resolved_deepseek_auth(
            &self.info,
            self.auth_manager.as_deref(),
        ))
    }
}

impl ModelProvider for DeepSeekModelProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.info
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            namespace_tools: true,
            image_generation: false,
            web_search: true,
            external_web_access: false,
            remote_compaction: RemoteCompactionSupport::Unsupported,
            account_balance: true,
            api_key_login: true,
        }
    }

    fn approval_review_preferred_model(&self) -> &'static str {
        DEEPSEEK_DEFAULT_MODEL
    }

    fn memory_extraction_preferred_model(&self) -> &'static str {
        DEEPSEEK_DEFAULT_MODEL
    }

    fn memory_consolidation_preferred_model(&self) -> &'static str {
        DEEPSEEK_DEFAULT_MODEL
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.auth_manager.clone()
    }

    fn auth(&self) -> ModelProviderFuture<'_, Option<CodexAuth>> {
        Box::pin(async move {
            match self.auth_manager.as_ref() {
                Some(auth_manager) => auth_manager.auth().await,
                None => None,
            }
        })
    }

    fn api_auth(&self) -> ModelProviderFuture<'_, CodexResult<SharedAuthProvider>> {
        Box::pin(self.resolved_auth())
    }

    fn account_state(&self) -> ProviderAccountResult {
        Ok(ProviderAccountState {
            account: Some(ProviderAccount::ApiKey),
            requires_openai_auth: false,
        })
    }

    fn account_balance(
        &self,
        http_client_factory: HttpClientFactory,
    ) -> ModelProviderFuture<'_, CodexResult<Option<ProviderBalance>>> {
        Box::pin(async move {
            let auth = self.resolved_auth().await?;
            get_balance(&self.info, auth, http_client_factory)
                .await
                .map(Some)
        })
    }

    fn validate_api_key<'a>(
        &'a self,
        api_key: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelProviderFuture<'a, CodexResult<Option<ProviderBalance>>> {
        Box::pin(async move {
            get_deepseek_balance_with_api_key(&self.info, api_key, http_client_factory)
                .await
                .map(Some)
        })
    }

    fn models_manager(
        &self,
        _codex_home: PathBuf,
        config_model_catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        Arc::new(DeepSeekModelsManager::new(
            self.info.clone(),
            self.auth_manager.clone(),
            config_model_catalog,
        ))
    }

    fn models_manager_without_cache(
        &self,
        config_model_catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        self.models_manager(PathBuf::new(), config_model_catalog)
    }
}

#[derive(Debug)]
struct DeepSeekModelsManager {
    info: ModelProviderInfo,
    models: RwLock<Vec<ModelInfo>>,
    auth_manager: Option<Arc<AuthManager>>,
}

impl DeepSeekModelsManager {
    fn new(
        info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
        config_model_catalog: Option<ModelsResponse>,
    ) -> Self {
        let configured_flash = config_model_catalog.and_then(|catalog| {
            catalog
                .models
                .into_iter()
                .find(|model| model.slug == DEEPSEEK_DEFAULT_MODEL)
        });
        Self {
            info,
            models: RwLock::new(vec![configured_flash.unwrap_or_else(deepseek_flash_model)]),
            auth_manager,
        }
    }

    async fn refresh(&self, http_client_factory: HttpClientFactory) {
        match list_models(&self.info, self.resolved_auth(), http_client_factory).await {
            Ok(models) if !models.is_empty() => *self.models.write().await = models,
            Ok(_) => tracing::warn!(
                model = DEEPSEEK_DEFAULT_MODEL,
                "DeepSeek /models omitted the supported model; using bundled metadata"
            ),
            Err(err) => tracing::warn!(error = %err, "failed to refresh DeepSeek models"),
        }
    }

    fn resolved_auth(&self) -> SharedAuthProvider {
        resolved_deepseek_auth(&self.info, self.auth_manager.as_deref())
    }
}

fn resolved_deepseek_auth(
    provider_info: &ModelProviderInfo,
    auth_manager: Option<&AuthManager>,
) -> SharedAuthProvider {
    if let Some(env_key) = provider_info.env_key.as_deref()
        && let Ok(api_key) = std::env::var(env_key)
        && !api_key.trim().is_empty()
    {
        return auth_provider_from_auth(&CodexAuth::from_api_key(&api_key));
    }
    auth_manager
        .and_then(AuthManager::auth_cached)
        .as_ref()
        .map(auth_provider_from_auth)
        .unwrap_or_else(unauthenticated_auth_provider)
}

impl ModelsManager for DeepSeekModelsManager {
    fn get_default_model<'a>(
        &'a self,
        _model: &'a Option<String>,
        _allow_provider_model_fallback: bool,
        _refresh_strategy: RefreshStrategy,
        _http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'a, String> {
        Box::pin(async { DEEPSEEK_DEFAULT_MODEL.to_string() })
    }

    fn raw_model_catalog(
        &self,
        refresh_strategy: RefreshStrategy,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'_, ModelsResponse> {
        Box::pin(async move {
            if refresh_strategy != RefreshStrategy::Offline {
                self.refresh(http_client_factory).await;
            }
            ModelsResponse {
                models: self.models.read().await.clone(),
            }
        })
    }

    fn get_remote_models(&self) -> ModelsManagerFuture<'_, Vec<ModelInfo>> {
        Box::pin(async move { self.models.read().await.clone() })
    }

    fn try_get_remote_models(&self) -> Result<Vec<ModelInfo>, TryLockError> {
        Ok(self.models.try_read()?.clone())
    }

    fn auth_manager(&self) -> Option<&AuthManager> {
        self.auth_manager.as_deref()
    }

    fn list_collaboration_modes(&self) -> Vec<CollaborationModeMask> {
        builtin_collaboration_mode_presets()
    }

    fn refresh_if_new_etag(
        &self,
        _etag: String,
        http_client_factory: HttpClientFactory,
    ) -> ModelsManagerFuture<'_, ()> {
        Box::pin(async move { self.refresh(http_client_factory).await })
    }
}

async fn list_models(
    provider_info: &ModelProviderInfo,
    auth: SharedAuthProvider,
    http_client_factory: HttpClientFactory,
) -> CodexResult<Vec<ModelInfo>> {
    let response = get_deepseek_json(provider_info, auth, "models", http_client_factory).await?;
    let response: DeepSeekModelsResponse = serde_json::from_slice(&response)?;
    Ok(response
        .data
        .into_iter()
        .filter(|model| model.id == DEEPSEEK_DEFAULT_MODEL)
        .map(|_| deepseek_flash_model())
        .collect())
}

async fn get_balance(
    provider_info: &ModelProviderInfo,
    auth: SharedAuthProvider,
    http_client_factory: HttpClientFactory,
) -> CodexResult<ProviderBalance> {
    let response =
        get_deepseek_json(provider_info, auth, "user/balance", http_client_factory).await?;
    Ok(serde_json::from_slice(&response)?)
}

pub async fn validate_deepseek_api_key(
    provider_info: &ModelProviderInfo,
    api_key: &str,
    http_client_factory: HttpClientFactory,
) -> CodexResult<()> {
    get_deepseek_balance_with_api_key(provider_info, api_key, http_client_factory)
        .await
        .map(|_| ())
}

pub async fn get_deepseek_balance_with_api_key(
    provider_info: &ModelProviderInfo,
    api_key: &str,
    http_client_factory: HttpClientFactory,
) -> CodexResult<ProviderBalance> {
    let auth = auth_provider_from_auth(&CodexAuth::from_api_key(api_key));
    get_balance(provider_info, auth, http_client_factory).await
}

async fn get_deepseek_json(
    provider_info: &ModelProviderInfo,
    auth: SharedAuthProvider,
    path: &str,
    http_client_factory: HttpClientFactory,
) -> CodexResult<Vec<u8>> {
    let base_url = provider_info.base_url.as_deref().ok_or_else(|| {
        CodexErrorDetails::InvalidRequest("DeepSeek provider base URL is missing".to_string())
    })?;
    let url = format!("{}/{path}", base_url.trim_end_matches('/'));
    let client =
        create_client_for_route_async(http_client_factory, url.clone(), ClientRouteClass::Api)
            .await
            .map_err(|err| {
                CodexErrorDetails::Fatal(format!("failed to create DeepSeek HTTP client: {err}"))
            })?;
    let response = client
        .get(&url)
        .headers(auth.to_auth_headers())
        .timeout(DEEPSEEK_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|err| CodexErrorDetails::Fatal(format!("DeepSeek request failed: {err}")))?;
    let status = response.status();
    let body = response.bytes().await.map_err(|err| {
        CodexErrorDetails::Fatal(format!("failed to read DeepSeek response: {err}"))
    })?;
    if !status.is_success() {
        let body = String::from_utf8_lossy(&body);
        let mut end = body.len().min(MAX_ERROR_BODY_BYTES);
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        let body = &body[..end];
        return Err(CodexErrorDetails::InvalidRequest(format!(
            "DeepSeek returned HTTP {status}: {body}"
        ))
        .into());
    }
    Ok(body.to_vec())
}

fn deepseek_flash_model() -> ModelInfo {
    let mut model = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"))
        .models
        .into_iter()
        .find(|model| model.slug == "gpt-5.4-mini")
        .unwrap_or_else(|| panic!("bundled models.json should include gpt-5.4-mini"));
    model.slug = DEEPSEEK_DEFAULT_MODEL.to_string();
    model.display_name = "DeepSeek-V4-Flash".to_string();
    model.description = Some("DeepSeek's fast agentic coding model.".to_string());
    model.default_reasoning_level = Some(ReasoningEffort::High);
    model.supported_reasoning_levels = vec![
        ReasoningEffortPreset {
            effort: ReasoningEffort::High,
            description: "Extra reasoning depth for complex problems".to_string(),
        },
        ReasoningEffortPreset {
            effort: ReasoningEffort::Max,
            description: "Maximum reasoning depth for the hardest problems".to_string(),
        },
    ];
    model.visibility = ModelVisibility::List;
    model.priority = 1;
    model.additional_speed_tiers.clear();
    model.service_tiers.clear();
    model.default_service_tier = None;
    model.availability_nux = None;
    model.upgrade = None;
    model.supports_reasoning_summary_parameter = false;
    model.support_verbosity = false;
    model.default_verbosity = None;
    model.supports_parallel_tool_calls = true;
    model.supports_image_detail_original = false;
    model.include_skills_usage_instructions = false;
    model.include_plugin_usage_instructions = false;
    model.context_window = Some(DEEPSEEK_CONTEXT_WINDOW);
    model.max_context_window = Some(DEEPSEEK_CONTEXT_WINDOW);
    model.comp_hash = None;
    model.effective_context_window_percent = 95;
    model.input_modalities = vec![InputModality::Text];
    model.web_search_tool_type = WebSearchToolType::Text;
    model.supports_search_tool = true;
    model.multi_agent_version = Some(MultiAgentVersion::V2);
    model.used_fallback_model_metadata = false;
    model
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
