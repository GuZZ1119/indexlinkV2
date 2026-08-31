//! AI 服务提供者抽象与配置。
//!
//! [`AiProvider`] 是 LLM 后端的可替换 trait，
//! 遵循与 [`ReadinessCheck`] 相同的适配器模式。
//!
//! [`ReadinessCheck`]: indexlink_api::state::ReadinessCheck

use std::{collections::BTreeMap, fmt, time::Duration};

use async_trait::async_trait;
use serde::Serialize;

use crate::{AiClientError, Sentiment, SentimentAnalysis};

/// Stable identifier for one deployed AI provider implementation.
///
/// It is intentionally unrelated to an API key, URL, or account. A provider
/// ID describes an implementation such as `qwen`; users select only a server
/// registered profile that refers to such an ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AiProviderId(String);

impl AiProviderId {
    /// Construct a normalized provider identifier from a safe ASCII slug.
    pub fn new(value: impl Into<String>) -> Result<Self, AiProviderProfileError> {
        let value = value.into().trim().to_ascii_lowercase();
        if value.is_empty()
            || value.len() > 64
            || !value.is_ascii()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(AiProviderProfileError::InvalidIdentifier);
        }
        Ok(Self(value))
    }

    /// Return the stable provider slug.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AiProviderId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Stable identifier for one server-deployed, user-selectable AI profile.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct AiProviderProfileId(String);

impl AiProviderProfileId {
    /// Construct a normalized profile identifier from a safe ASCII slug.
    pub fn new(value: impl Into<String>) -> Result<Self, AiProviderProfileError> {
        AiProviderId::new(value).map(|identifier| Self(identifier.0))
    }

    /// Return the stable profile slug.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AiProviderProfileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Explicit, non-authorising capabilities advertised by an AI provider profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AiProviderCapabilities {
    /// Whether the profile can return structured news-grounded AI evidence.
    pub market_evidence: bool,
    /// Whether the profile may later propose a restricted DSL draft.
    ///
    /// This describes output shape only. It never grants save, activation, or order authority.
    pub restricted_policy_drafts: bool,
}

impl AiProviderCapabilities {
    /// Declare a profile that can provide structured market evidence only.
    #[must_use]
    pub const fn market_evidence_only() -> Self {
        Self {
            market_evidence: true,
            restricted_policy_drafts: false,
        }
    }
}

/// Credential-free metadata for one deployed AI profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiProviderProfile {
    id: AiProviderProfileId,
    provider: AiProviderId,
    display_name: String,
    model: String,
    capabilities: AiProviderCapabilities,
}

impl AiProviderProfile {
    /// Create one display-safe server profile.
    ///
    /// The constructor deliberately does not accept credentials, endpoint URLs,
    /// account identifiers, or secret-manager references.
    pub fn new(
        id: AiProviderProfileId,
        provider: AiProviderId,
        display_name: String,
        model: String,
        capabilities: AiProviderCapabilities,
    ) -> Result<Self, AiProviderProfileError> {
        let display_name = normalize_profile_text(display_name)?;
        let model = normalize_profile_text(model)?;
        Ok(Self {
            id,
            provider,
            display_name,
            model,
            capabilities,
        })
    }

    /// Return the user-selectable deployed profile identifier.
    #[must_use]
    pub fn id(&self) -> &AiProviderProfileId {
        &self.id
    }

    /// Return the provider implementation identifier.
    #[must_use]
    pub fn provider(&self) -> &AiProviderId {
        &self.provider
    }

    /// Return the safe display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Return the configured public model name.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return declared non-authorising capabilities.
    #[must_use]
    pub const fn capabilities(&self) -> AiProviderCapabilities {
        self.capabilities
    }

    /// Build the default Qwen profile for a configured compatible model.
    #[must_use]
    pub fn qwen(model: String) -> Self {
        Self::new(
            AiProviderProfileId::new("qwen-default").expect("static profile ID is valid"),
            AiProviderId::new("qwen").expect("static provider ID is valid"),
            "Qwen".to_owned(),
            model,
            AiProviderCapabilities::market_evidence_only(),
        )
        .expect("static Qwen profile is valid")
    }

    fn external_default() -> Self {
        Self::new(
            AiProviderProfileId::new("external-default").expect("static profile ID is valid"),
            AiProviderId::new("external").expect("static provider ID is valid"),
            "External AI provider".to_owned(),
            "unspecified".to_owned(),
            AiProviderCapabilities::market_evidence_only(),
        )
        .expect("static external profile is valid")
    }
}

/// Error returned when profile metadata would be unsafe or ambiguous to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AiProviderProfileError {
    /// An identifier is blank, oversized, non-ASCII, or not a lowercase slug.
    #[error("AI provider identifier is invalid")]
    InvalidIdentifier,
    /// A display name or model is blank, oversized, or contains a control character.
    #[error("AI provider profile text is invalid")]
    InvalidText,
    /// A profile ID was registered more than once.
    #[error("AI provider profile is already registered")]
    DuplicateProfile,
    /// The requested default profile was not among the deployed registrations.
    #[error("AI provider profile is not registered")]
    UnregisteredProfile,
}

fn normalize_profile_text(value: String) -> Result<String, AiProviderProfileError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 120 || value.chars().any(char::is_control) {
        Err(AiProviderProfileError::InvalidText)
    } else {
        Ok(value.to_owned())
    }
}

/// Credential-free registry of AI profiles deployed by the server operator.
///
/// This type contains metadata only. The API state separately retains the
/// matching provider clients, so listing a registry can never reveal a key or
/// connection URL.
#[derive(Debug, Clone, Default)]
pub struct AiProviderRegistry {
    profiles: BTreeMap<AiProviderProfileId, AiProviderProfile>,
}

impl AiProviderRegistry {
    /// Register one unique server-deployed profile.
    pub fn register(&mut self, profile: AiProviderProfile) -> Result<(), AiProviderProfileError> {
        if self.profiles.contains_key(profile.id()) {
            return Err(AiProviderProfileError::DuplicateProfile);
        }
        self.profiles.insert(profile.id().clone(), profile);
        Ok(())
    }

    /// Return deployed profiles in stable identifier order.
    #[must_use]
    pub fn profiles(&self) -> Vec<AiProviderProfile> {
        self.profiles.values().cloned().collect()
    }

    /// Return a deployed profile by its stable identifier.
    #[must_use]
    pub fn get(&self, id: &AiProviderProfileId) -> Option<&AiProviderProfile> {
        self.profiles.get(id)
    }
}

/// LLM 后端的可替换抽象。
///
/// 当前实现：
/// - [`QwenClient`]：兼容 Qwen / OpenAI API。
/// - 测试：`MockAiProvider`（不发起网络请求）。
///
/// [`QwenClient`]: crate::QwenClient
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Return credential-free metadata for this server-deployed profile.
    ///
    /// Third-party test adapters retain a safe generic profile until they
    /// choose to override this method. Production adapters should always
    /// return their concrete profile.
    fn profile(&self) -> AiProviderProfile {
        AiProviderProfile::external_default()
    }

    /// 分析新闻/财报文本，返回有界情绪得分。
    ///
    /// # 错误
    ///
    /// 超时、网络错误、API 错误或响应解析失败时返回 [`AiClientError`]。
    /// ai-client 不在此层降级——由上层 decision engine 按 70/20/10 → 90/10/0
    /// 策略处理错误（AI 权重归零，仅用基本面和趋势数据决策）。
    async fn analyze(&self, prompt: &str) -> Result<Sentiment, AiClientError>;

    /// 分析新闻文本并返回分数、依据与风险提示。
    ///
    /// 旧 provider 只实现 [`Self::analyze`] 时会得到一个明确标识为通用说明的
    /// 兼容结果；真实 Qwen adapter 会覆盖此方法并返回模型的结构化输出。
    async fn analyze_with_evidence(
        &self,
        prompt: &str,
    ) -> Result<SentimentAnalysis, AiClientError> {
        let sentiment = self.analyze(prompt).await?;
        SentimentAnalysis::new(
            sentiment,
            "The configured AI provider returned a bounded sentiment score without a detailed rationale."
                .to_owned(),
            Vec::new(),
        )
        .map_err(|_| AiClientError::ParseFailure)
    }
}

/// AI 服务连接配置。
///
/// `Debug` 和 `Display` 实现**不暴露** `api_key`。
/// 遵循项目安全规范：连接凭证不可出现在日志或错误消息中。
pub struct AiConfig {
    /// API 基础 URL（如 `https://dashscope.aliyuncs.com/compatible-mode`）。
    pub base_url: String,
    /// API 密钥（不在 Debug/Display 中暴露）。
    pub api_key: String,
    /// 模型名称（如 `qwen-plus`、`qwen-max`）。
    pub model: String,
    /// 单次请求超时。
    pub timeout: Duration,
    /// 最大生成 token 数（为结构化理由和风险提示预留空间，默认 256）。
    pub max_tokens: u32,
    /// 生成温度（建议 0.0~0.3，降低随机性以保持信号稳定）。
    pub temperature: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://dashscope.aliyuncs.com/compatible-mode".to_owned(),
            api_key: String::new(),
            model: "qwen-plus".to_owned(),
            timeout: Duration::from_secs(30),
            max_tokens: 256,
            temperature: 0.0,
        }
    }
}

impl fmt::Debug for AiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AiConfig")
            .field("base_url", &redact_url_userinfo(&self.base_url))
            .field("api_key", &"<redacted>")
            .field("model", &self.model)
            .field("timeout", &self.timeout)
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .finish()
    }
}

/// 去除 URL 中的 `user:password@` 部分，防止 Debug/Display 输出泄露嵌入的凭据。
fn redact_url_userinfo(url: &str) -> String {
    match url.find('@') {
        Some(at_pos) if url.contains("://") => {
            let scheme_end = url.find("://").unwrap();
            format!(
                "{}<redacted>@{}",
                &url[..=scheme_end + 2],
                &url[at_pos + 1..]
            )
        }
        _ => url.to_owned(),
    }
}

impl fmt::Display for AiConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AiConfig(model={}, base_url={})",
            self.model,
            redact_url_userinfo(&self.base_url)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_values() {
        let config = AiConfig::default();
        assert!(config.base_url.contains("dashscope"));
        assert_eq!(config.model, "qwen-plus");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_tokens, 256);
        assert_eq!(config.temperature, 0.0);
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn config_debug_redacts_api_key() {
        let config = AiConfig {
            api_key: "sk-secret-key-12345".to_owned(),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("sk-secret-key-12345"));
        assert!(debug.contains("qwen-plus"));
        assert!(debug.contains("dashscope"));
    }

    #[test]
    fn config_display_hides_api_key_and_url_credentials() {
        let config = AiConfig {
            base_url: "https://user:password@evil.example.com/v1".to_owned(),
            api_key: "sk-secret-key-12345".to_owned(),
            ..Default::default()
        };
        let display = format!("{config}");
        assert!(display.contains("evil.example.com"));
        assert!(
            !display.contains("user:password"),
            "URL 凭据不应出现在 Display 中"
        );
        assert!(display.contains("<redacted>"));
        assert!(!display.contains("sk-secret-key-12345"));
    }

    #[test]
    fn config_debug_redacts_embedded_url_credentials() {
        let config = AiConfig {
            base_url: "https://user:password@evil.example.com/v1".to_owned(),
            api_key: "sk-abc".to_owned(),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        // URL 凭据被 redact，但 host 可保留（Debug 是给开发者看的）
        assert!(debug.contains("evil.example.com"));
        assert!(
            !debug.contains("user:password"),
            "URL 凭据不应出现在 Debug 中"
        );
        assert!(debug.contains("<redacted>"), "应有 redact 标记");
        // api_key 同样被 redact
        assert!(!debug.contains("sk-abc"));
    }

    #[test]
    fn registry_keeps_only_credential_free_qwen_metadata() {
        let profile = AiProviderProfile::qwen("qwen-plus".to_owned());
        let mut registry = AiProviderRegistry::default();
        registry.register(profile).unwrap();

        let listed = registry.profiles();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id().as_str(), "qwen-default");
        assert_eq!(listed[0].provider().as_str(), "qwen");
        assert_eq!(listed[0].model(), "qwen-plus");
        assert!(listed[0].capabilities().market_evidence);
        assert!(!listed[0].capabilities().restricted_policy_drafts);
        assert!(!format!("{listed:?}").contains("sk-"));
    }

    #[test]
    fn registry_rejects_duplicate_profile_ids() {
        let mut registry = AiProviderRegistry::default();
        registry
            .register(AiProviderProfile::qwen("qwen-plus".to_owned()))
            .unwrap();
        assert_eq!(
            registry
                .register(AiProviderProfile::qwen("qwen-max".to_owned()))
                .unwrap_err(),
            AiProviderProfileError::DuplicateProfile
        );
    }
}
