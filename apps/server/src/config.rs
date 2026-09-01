use std::{
    collections::BTreeSet,
    env,
    net::{IpAddr, SocketAddr},
    num::ParseIntError,
    time::Duration,
};

use ai_client::{
    AiConfig, AiProviderCapabilities, AiProviderId, AiProviderProfile, AiProviderProfileId,
};
use axum::http::HeaderValue;
use broker::{BrokerProvider, OpenDConnectionConfig};
use serde::Deserialize;

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: &str = "8080";
const DEFAULT_DATABASE_URL: &str = "sqlite://indexlink.db?mode=rwc";
const DEFAULT_MAX_CONNECTIONS: &str = "10";
const DEFAULT_CONNECT_TIMEOUT_SECONDS: &str = "5";
const DASHSCOPE_API_KEY: &str = "DASHSCOPE_API_KEY";
const DASHSCOPE_BASE_URL: &str = "DASHSCOPE_BASE_URL";
const DASHSCOPE_MODEL: &str = "DASHSCOPE_MODEL";
const DASHSCOPE_TIMEOUT_SECONDS: &str = "DASHSCOPE_TIMEOUT_SECONDS";
const DASHSCOPE_MAX_TOKENS: &str = "DASHSCOPE_MAX_TOKENS";
const DASHSCOPE_TEMPERATURE: &str = "DASHSCOPE_TEMPERATURE";
const AI_PROVIDER_PROFILES: &str = "AI_PROVIDER_PROFILES";
const OPEND_PROVIDER: &str = "OPEND_PROVIDER";
const OPEND_HOST: &str = "OPEND_HOST";
const OPEND_PORT: &str = "OPEND_PORT";
const OPEND_ACCOUNT_ID: &str = "OPEND_ACCOUNT_ID";
const DEFAULT_OPEND_HOST: &str = "127.0.0.1";
const DEFAULT_OPEND_PORT: &str = "11111";
const SCHEDULER_ENABLED: &str = "SCHEDULER_ENABLED";
const SCHEDULER_TICK_SECONDS: &str = "SCHEDULER_TICK_SECONDS";
const DEFAULT_SCHEDULER_ENABLED: &str = "true";
const DEFAULT_SCHEDULER_TICK_SECONDS: &str = "60";

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) address: SocketAddr,
    pub(crate) database_url: String,
    pub(crate) database_max_connections: u32,
    pub(crate) database_connect_timeout: Duration,
    pub(crate) cors_allowed_origins: Vec<HeaderValue>,
    pub(crate) ai_providers: Vec<AiProviderConfiguration>,
    pub(crate) opend: Option<OpenDConnectionConfig>,
    pub(crate) scheduler: SchedulerConfig,
}

/// One OpenAI-compatible AI provider composed by the server from local configuration only.
#[derive(Debug)]
pub(crate) struct AiProviderConfiguration {
    pub(crate) profile: AiProviderProfile,
    pub(crate) client: AiConfig,
    pub(crate) is_default: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AiProviderProfileEnvironment {
    id: String,
    provider: String,
    display_name: String,
    base_url: String,
    api_key_env: String,
    model: String,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    capabilities: AiProviderCapabilitiesEnvironment,
    timeout_seconds: Option<u64>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AiProviderCapabilitiesEnvironment {
    #[serde(default = "true_value")]
    market_evidence: bool,
    #[serde(default)]
    restricted_policy_drafts: bool,
}

fn true_value() -> bool {
    true
}

/// Safe periodic automatic-decision scheduler settings for weekly or monthly fixed plan dates.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SchedulerConfig {
    pub(crate) enabled: bool,
    pub(crate) tick_interval: Duration,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|name| env::var(name).ok())
    }

    pub(super) fn from_lookup(
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, ConfigError> {
        let host = value_or_default(&mut lookup, "APP_HOST", DEFAULT_HOST);
        let port = parse_u16(
            "APP_PORT",
            &value_or_default(&mut lookup, "APP_PORT", DEFAULT_PORT),
        )?;
        let ip = host
            .parse::<IpAddr>()
            .map_err(|_| ConfigError::InvalidHost)?;

        let database_url = value_or_default(&mut lookup, "DATABASE_URL", DEFAULT_DATABASE_URL);
        if database_url.trim().is_empty() || !database_url.starts_with("sqlite:") {
            return Err(ConfigError::InvalidDatabaseUrl);
        }

        let database_max_connections = parse_u32(
            "DATABASE_MAX_CONNECTIONS",
            &value_or_default(
                &mut lookup,
                "DATABASE_MAX_CONNECTIONS",
                DEFAULT_MAX_CONNECTIONS,
            ),
        )?;
        if database_max_connections == 0 {
            return Err(ConfigError::NonPositive("DATABASE_MAX_CONNECTIONS"));
        }

        let timeout_seconds = parse_u64(
            "DATABASE_CONNECT_TIMEOUT_SECONDS",
            &value_or_default(
                &mut lookup,
                "DATABASE_CONNECT_TIMEOUT_SECONDS",
                DEFAULT_CONNECT_TIMEOUT_SECONDS,
            ),
        )?;
        if timeout_seconds == 0 {
            return Err(ConfigError::NonPositive("DATABASE_CONNECT_TIMEOUT_SECONDS"));
        }

        let cors_allowed_origins = lookup("CORS_ALLOWED_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(|origin| {
                origin
                    .parse::<HeaderValue>()
                    .map_err(|_| ConfigError::InvalidCorsOrigin)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let ai_providers = ai_provider_configurations(&mut lookup)?;
        let opend = opend_config(&mut lookup)?;
        let scheduler = scheduler_config(&mut lookup)?;

        Ok(Self {
            address: SocketAddr::new(ip, port),
            database_url,
            database_max_connections,
            database_connect_timeout: Duration::from_secs(timeout_seconds),
            cors_allowed_origins,
            ai_providers,
            opend,
            scheduler,
        })
    }
}

fn scheduler_config(
    lookup: &mut impl FnMut(&str) -> Option<String>,
) -> Result<SchedulerConfig, ConfigError> {
    let enabled = parse_bool(
        SCHEDULER_ENABLED,
        &value_or_default(lookup, SCHEDULER_ENABLED, DEFAULT_SCHEDULER_ENABLED),
    )?;
    let tick_seconds = parse_u64(
        SCHEDULER_TICK_SECONDS,
        &value_or_default(
            lookup,
            SCHEDULER_TICK_SECONDS,
            DEFAULT_SCHEDULER_TICK_SECONDS,
        ),
    )?;
    if tick_seconds == 0 {
        return Err(ConfigError::NonPositive(SCHEDULER_TICK_SECONDS));
    }
    Ok(SchedulerConfig {
        enabled,
        tick_interval: Duration::from_secs(tick_seconds),
    })
}

fn opend_config(
    lookup: &mut impl FnMut(&str) -> Option<String>,
) -> Result<Option<OpenDConnectionConfig>, ConfigError> {
    let Some(provider) = lookup(OPEND_PROVIDER) else {
        return Ok(None);
    };
    let provider = match non_blank(OPEND_PROVIDER, provider)?
        .to_ascii_lowercase()
        .as_str()
    {
        "futu" => BrokerProvider::Futu,
        "moomoo" => BrokerProvider::Moomoo,
        _ => return Err(ConfigError::InvalidOpenDProvider),
    };
    let host = normalize_loopback_opend_host(non_blank(
        OPEND_HOST,
        lookup(OPEND_HOST).unwrap_or_else(|| DEFAULT_OPEND_HOST.to_owned()),
    )?)?;
    let port = parse_u16(
        OPEND_PORT,
        &lookup(OPEND_PORT).unwrap_or_else(|| DEFAULT_OPEND_PORT.to_owned()),
    )?;
    let account_id = lookup(OPEND_ACCOUNT_ID)
        .map(|value| non_blank(OPEND_ACCOUNT_ID, value))
        .transpose()?;

    let config = match account_id {
        Some(account_id) => {
            OpenDConnectionConfig::paper_with_account(provider, host, port, account_id)
        }
        None => OpenDConnectionConfig::paper(provider, host, port),
    }
    .map_err(|_| ConfigError::InvalidOpenDConfiguration)?;

    Ok(Some(config))
}

fn normalize_loopback_opend_host(host: String) -> Result<String, ConfigError> {
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(DEFAULT_OPEND_HOST.to_owned());
    }
    let address = host
        .parse::<IpAddr>()
        .map_err(|_| ConfigError::OpenDLoopbackRequired)?;
    if !address.is_loopback() {
        return Err(ConfigError::OpenDLoopbackRequired);
    }

    Ok(address.to_string())
}

fn ai_provider_configurations(
    lookup: &mut impl FnMut(&str) -> Option<String>,
) -> Result<Vec<AiProviderConfiguration>, ConfigError> {
    let Some(profiles) = lookup(AI_PROVIDER_PROFILES) else {
        return legacy_qwen_configuration(lookup)
            .map(|configuration| configuration.into_iter().collect());
    };
    let profiles = serde_json::from_str::<Vec<AiProviderProfileEnvironment>>(&profiles)
        .map_err(|_| ConfigError::InvalidAiProviderProfiles)?;
    if profiles.is_empty() {
        return Err(ConfigError::InvalidAiProviderProfiles);
    }

    let defaults = AiConfig::default();
    let mut ids = BTreeSet::new();
    let mut default_count = 0usize;
    let mut configured = Vec::with_capacity(profiles.len());
    for profile in profiles {
        let id = AiProviderProfileId::new(profile.id)
            .map_err(|_| ConfigError::InvalidAiProviderProfiles)?;
        if !ids.insert(id.clone()) {
            return Err(ConfigError::InvalidAiProviderProfiles);
        }
        let provider = AiProviderId::new(profile.provider)
            .map_err(|_| ConfigError::InvalidAiProviderProfiles)?;
        let base_url = normalize_ai_base_url(profile.base_url)?;
        let api_key_env = normalize_environment_name(profile.api_key_env)?;
        let api_key = lookup(&api_key_env).ok_or(ConfigError::MissingAiProviderKey)?;
        let api_key = non_blank(AI_PROVIDER_PROFILES, api_key)
            .map_err(|_| ConfigError::MissingAiProviderKey)?;
        let model = non_blank(AI_PROVIDER_PROFILES, profile.model)
            .map_err(|_| ConfigError::InvalidAiProviderProfiles)?;
        let timeout_seconds = profile
            .timeout_seconds
            .unwrap_or(defaults.timeout.as_secs());
        let max_tokens = profile.max_tokens.unwrap_or(defaults.max_tokens);
        let temperature = profile.temperature.unwrap_or(defaults.temperature);
        if timeout_seconds == 0
            || max_tokens == 0
            || !temperature.is_finite()
            || !(0.0..=2.0).contains(&temperature)
        {
            return Err(ConfigError::InvalidAiProviderProfiles);
        }
        let configured_profile = AiProviderProfile::new(
            id,
            provider,
            profile.display_name,
            model.clone(),
            AiProviderCapabilities {
                market_evidence: profile.capabilities.market_evidence,
                restricted_policy_drafts: profile.capabilities.restricted_policy_drafts,
            },
        )
        .map_err(|_| ConfigError::InvalidAiProviderProfiles)?;
        if profile.default {
            default_count += 1;
        }
        configured.push(AiProviderConfiguration {
            profile: configured_profile,
            client: AiConfig {
                base_url,
                api_key,
                model,
                timeout: Duration::from_secs(timeout_seconds),
                max_tokens,
                temperature,
            },
            is_default: profile.default,
        });
    }
    if default_count != 1 {
        return Err(ConfigError::InvalidAiProviderProfiles);
    }
    Ok(configured)
}

fn legacy_qwen_configuration(
    lookup: &mut impl FnMut(&str) -> Option<String>,
) -> Result<Option<AiProviderConfiguration>, ConfigError> {
    let Some(api_key) = lookup(DASHSCOPE_API_KEY) else {
        return Ok(None);
    };
    let api_key = non_blank(DASHSCOPE_API_KEY, api_key)?;
    let defaults = AiConfig::default();
    let base_url = non_blank(
        DASHSCOPE_BASE_URL,
        lookup(DASHSCOPE_BASE_URL).unwrap_or(defaults.base_url),
    )?;
    let model = non_blank(
        DASHSCOPE_MODEL,
        lookup(DASHSCOPE_MODEL).unwrap_or(defaults.model),
    )?;
    let timeout_seconds = parse_u64(
        DASHSCOPE_TIMEOUT_SECONDS,
        &lookup(DASHSCOPE_TIMEOUT_SECONDS)
            .unwrap_or_else(|| defaults.timeout.as_secs().to_string()),
    )?;
    if timeout_seconds == 0 {
        return Err(ConfigError::NonPositive(DASHSCOPE_TIMEOUT_SECONDS));
    }
    let max_tokens = parse_u32(
        DASHSCOPE_MAX_TOKENS,
        &lookup(DASHSCOPE_MAX_TOKENS).unwrap_or_else(|| defaults.max_tokens.to_string()),
    )?;
    if max_tokens == 0 {
        return Err(ConfigError::NonPositive(DASHSCOPE_MAX_TOKENS));
    }
    let temperature = lookup(DASHSCOPE_TEMPERATURE)
        .unwrap_or_else(|| defaults.temperature.to_string())
        .parse::<f32>()
        .map_err(|source| ConfigError::InvalidFloat {
            name: DASHSCOPE_TEMPERATURE,
            source,
        })?;
    if !temperature.is_finite() || !(0.0..=2.0).contains(&temperature) {
        return Err(ConfigError::InvalidTemperature);
    }

    Ok(Some(AiProviderConfiguration {
        profile: AiProviderProfile::qwen(model.clone()),
        client: AiConfig {
            base_url,
            api_key,
            model,
            timeout: Duration::from_secs(timeout_seconds),
            max_tokens,
            temperature,
        },
        is_default: true,
    }))
}

fn normalize_environment_name(value: String) -> Result<String, ConfigError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 80
        || !value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        Err(ConfigError::InvalidAiProviderProfiles)
    } else {
        Ok(value.to_owned())
    }
}

fn normalize_ai_base_url(value: String) -> Result<String, ConfigError> {
    let value = value.trim();
    let url = reqwest::Url::parse(value).map_err(|_| ConfigError::InvalidAiProviderBaseUrl)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigError::InvalidAiProviderBaseUrl);
    }
    match url.scheme() {
        "https" => Ok(value.trim_end_matches('/').to_owned()),
        "http" if url.host_str().is_some_and(is_loopback_host) => {
            Ok(value.trim_end_matches('/').to_owned())
        }
        _ => Err(ConfigError::InvalidAiProviderBaseUrl),
    }
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn non_blank(name: &'static str, value: String) -> Result<String, ConfigError> {
    let normalized = value.trim().to_owned();
    if normalized.is_empty() {
        Err(ConfigError::BlankValue(name))
    } else {
        Ok(normalized)
    }
}

fn value_or_default(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    name: &'static str,
    default: &'static str,
) -> String {
    lookup(name).unwrap_or_else(|| default.to_owned())
}

fn parse_u16(name: &'static str, value: &str) -> Result<u16, ConfigError> {
    value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger { name, source })
}

fn parse_u32(name: &'static str, value: &str) -> Result<u32, ConfigError> {
    value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger { name, source })
}

fn parse_u64(name: &'static str, value: &str) -> Result<u64, ConfigError> {
    value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger { name, source })
}

fn parse_bool(name: &'static str, value: &str) -> Result<bool, ConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ConfigError::InvalidBoolean { name }),
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("DATABASE_URL must be a non-blank SQLite URL")]
    InvalidDatabaseUrl,
    #[error("APP_HOST must be a valid IP address")]
    InvalidHost,
    #[error("{name} must be a valid integer")]
    InvalidInteger {
        name: &'static str,
        #[source]
        source: ParseIntError,
    },
    #[error("{name} must be a valid finite number")]
    InvalidFloat {
        name: &'static str,
        #[source]
        source: std::num::ParseFloatError,
    },
    #[error("{name} must be true or false")]
    InvalidBoolean { name: &'static str },
    #[error("DASHSCOPE_TEMPERATURE must be in the range 0.0..=2.0")]
    InvalidTemperature,
    #[error("AI_PROVIDER_PROFILES must contain one safe default profile per deployment")]
    InvalidAiProviderProfiles,
    #[error("configured AI provider URL must use HTTPS or local loopback HTTP")]
    InvalidAiProviderBaseUrl,
    #[error("configured AI provider key is unavailable")]
    MissingAiProviderKey,
    #[error("OPEND_PROVIDER must be futu or moomoo")]
    InvalidOpenDProvider,
    #[error("OPEND_HOST must be a loopback OpenD address")]
    OpenDLoopbackRequired,
    #[error("OpenD paper configuration is invalid")]
    InvalidOpenDConfiguration,
    #[error("{0} must not be blank")]
    BlankValue(&'static str),
    #[error("{0} must be greater than zero")]
    NonPositive(&'static str),
    #[error("CORS_ALLOWED_ORIGINS contains an invalid origin")]
    InvalidCorsOrigin,
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATABASE_URL: &str = "sqlite://test-indexlink.db?mode=rwc";

    fn parse(values: &[(&str, &str)]) -> Result<Config, ConfigError> {
        Config::from_lookup(|name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_owned())
        })
    }

    #[test]
    fn minimal_configuration_uses_documented_defaults() {
        let config = parse(&[]).unwrap();

        assert_eq!(config.address, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.database_url, DEFAULT_DATABASE_URL);
        assert_eq!(config.database_max_connections, 10);
        assert_eq!(config.database_connect_timeout, Duration::from_secs(5));
        assert!(config.cors_allowed_origins.is_empty());
        assert!(config.ai_providers.is_empty());
        assert!(config.opend.is_none());
        assert!(config.scheduler.enabled);
        assert_eq!(config.scheduler.tick_interval, Duration::from_secs(60));
    }

    #[test]
    fn custom_network_and_pool_values_are_parsed() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("APP_HOST", "127.0.0.1"),
            ("APP_PORT", "0"),
            ("DATABASE_MAX_CONNECTIONS", "23"),
            ("DATABASE_CONNECT_TIMEOUT_SECONDS", "17"),
        ])
        .unwrap();

        assert_eq!(config.address, "127.0.0.1:0".parse().unwrap());
        assert_eq!(config.database_max_connections, 23);
        assert_eq!(config.database_connect_timeout, Duration::from_secs(17));
    }

    #[test]
    fn scheduler_values_are_parsed_and_validated() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("SCHEDULER_ENABLED", "false"),
            ("SCHEDULER_TICK_SECONDS", "15"),
        ])
        .unwrap();
        assert!(!config.scheduler.enabled);
        assert_eq!(config.scheduler.tick_interval, Duration::from_secs(15));

        assert!(matches!(
            parse(&[("SCHEDULER_ENABLED", "sometimes")]),
            Err(ConfigError::InvalidBoolean {
                name: SCHEDULER_ENABLED
            })
        ));
        assert!(matches!(
            parse(&[("SCHEDULER_TICK_SECONDS", "0")]),
            Err(ConfigError::NonPositive(SCHEDULER_TICK_SECONDS))
        ));
    }

    #[test]
    fn blank_database_url_is_rejected() {
        assert!(matches!(
            parse(&[("DATABASE_URL", "  ")]),
            Err(ConfigError::InvalidDatabaseUrl)
        ));
    }

    #[test]
    fn non_sqlite_database_url_is_rejected() {
        assert!(matches!(
            parse(&[(
                "DATABASE_URL",
                "postgres://indexlink:indexlink@localhost/indexlink"
            )]),
            Err(ConfigError::InvalidDatabaseUrl)
        ));
    }

    #[test]
    fn invalid_host_is_rejected() {
        assert!(matches!(
            parse(&[("DATABASE_URL", DATABASE_URL), ("APP_HOST", "localhost")]),
            Err(ConfigError::InvalidHost)
        ));
    }

    #[test]
    fn invalid_port_is_rejected_with_variable_name() {
        let error = parse(&[("DATABASE_URL", DATABASE_URL), ("APP_PORT", "eight")])
            .expect_err("non-numeric port must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger {
                name: "APP_PORT",
                ..
            }
        ));
    }

    #[test]
    fn invalid_max_connections_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_MAX_CONNECTIONS", "many"),
        ])
        .expect_err("non-numeric pool size must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger {
                name: "DATABASE_MAX_CONNECTIONS",
                ..
            }
        ));
    }

    #[test]
    fn zero_max_connections_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_MAX_CONNECTIONS", "0"),
        ])
        .expect_err("zero pool size must fail");

        assert!(matches!(
            error,
            ConfigError::NonPositive("DATABASE_MAX_CONNECTIONS")
        ));
    }

    #[test]
    fn invalid_connect_timeout_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_CONNECT_TIMEOUT_SECONDS", "soon"),
        ])
        .expect_err("non-numeric timeout must fail");

        assert!(matches!(
            error,
            ConfigError::InvalidInteger {
                name: "DATABASE_CONNECT_TIMEOUT_SECONDS",
                ..
            }
        ));
    }

    #[test]
    fn zero_connect_timeout_is_rejected() {
        let error = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("DATABASE_CONNECT_TIMEOUT_SECONDS", "0"),
        ])
        .expect_err("zero timeout must fail");

        assert!(matches!(
            error,
            ConfigError::NonPositive("DATABASE_CONNECT_TIMEOUT_SECONDS")
        ));
    }

    #[test]
    fn single_cors_origin_is_parsed() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("CORS_ALLOWED_ORIGINS", "https://app.example"),
        ])
        .unwrap();

        assert_eq!(
            config.cors_allowed_origins,
            vec![HeaderValue::from_static("https://app.example")]
        );
    }

    #[test]
    fn multiple_cors_origins_are_trimmed() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            (
                "CORS_ALLOWED_ORIGINS",
                " https://one.example, https://two.example ",
            ),
        ])
        .unwrap();

        assert_eq!(
            config.cors_allowed_origins,
            vec![
                HeaderValue::from_static("https://one.example"),
                HeaderValue::from_static("https://two.example")
            ]
        );
    }

    #[test]
    fn empty_cors_entries_are_filtered() {
        let config = parse(&[
            ("DATABASE_URL", DATABASE_URL),
            ("CORS_ALLOWED_ORIGINS", ", ,https://app.example,,"),
        ])
        .unwrap();

        assert_eq!(
            config.cors_allowed_origins,
            vec![HeaderValue::from_static("https://app.example")]
        );
    }

    #[test]
    fn invalid_cors_header_value_is_rejected() {
        assert!(matches!(
            parse(&[
                ("DATABASE_URL", DATABASE_URL),
                ("CORS_ALLOWED_ORIGINS", "https://ok.example\nbad"),
            ]),
            Err(ConfigError::InvalidCorsOrigin)
        ));
    }

    #[test]
    fn configuration_errors_do_not_expose_database_url() {
        let secret_url = "postgres://private-user:private-password@internal/database";
        let error = parse(&[("DATABASE_URL", secret_url), ("APP_PORT", "invalid")])
            .expect_err("invalid port must fail");
        let display = error.to_string();

        assert_eq!(display, "APP_PORT must be a valid integer");
        assert!(!display.contains(secret_url));
        assert!(!display.contains("private-password"));
    }

    #[test]
    fn qwen_configuration_is_optional_and_uses_safe_defaults() {
        let config = parse(&[(DASHSCOPE_API_KEY, "test-secret")]).unwrap();
        let qwen = config
            .ai_providers
            .into_iter()
            .next()
            .expect("key enables Qwen configuration");

        assert_eq!(
            qwen.client.base_url,
            "https://dashscope.aliyuncs.com/compatible-mode"
        );
        assert_eq!(qwen.profile.id().as_str(), "qwen-default");
        assert!(qwen.is_default);
        assert_eq!(qwen.client.model, "qwen-plus");
        assert_eq!(qwen.client.timeout, Duration::from_secs(30));
        assert_eq!(qwen.client.max_tokens, 256);
        assert_eq!(qwen.client.temperature, 0.0);
        assert_eq!(qwen.client.api_key, "test-secret");
    }

    #[test]
    fn qwen_configuration_accepts_explicit_runtime_values() {
        let config = parse(&[
            (DASHSCOPE_API_KEY, "sk-test-secret"),
            (DASHSCOPE_BASE_URL, "https://qwen.example/compatible-mode/"),
            (DASHSCOPE_MODEL, "qwen-max"),
            (DASHSCOPE_TIMEOUT_SECONDS, "9"),
            (DASHSCOPE_MAX_TOKENS, "256"),
            (DASHSCOPE_TEMPERATURE, "0.2"),
        ])
        .unwrap();
        let qwen = config
            .ai_providers
            .into_iter()
            .next()
            .expect("key enables Qwen configuration");

        assert_eq!(
            qwen.client.base_url,
            "https://qwen.example/compatible-mode/"
        );
        assert_eq!(qwen.client.model, "qwen-max");
        assert_eq!(qwen.client.timeout, Duration::from_secs(9));
        assert_eq!(qwen.client.max_tokens, 256);
        assert_eq!(qwen.client.temperature, 0.2);
    }

    #[test]
    fn blank_qwen_key_is_rejected_without_echoing_the_value() {
        let error =
            parse(&[(DASHSCOPE_API_KEY, "  ")]).expect_err("blank configured key must fail fast");

        assert_eq!(error.to_string(), "DASHSCOPE_API_KEY must not be blank");
    }

    #[test]
    fn invalid_qwen_temperature_is_rejected() {
        let error = parse(&[
            (DASHSCOPE_API_KEY, "sk-test-secret"),
            (DASHSCOPE_TEMPERATURE, "2.1"),
        ])
        .expect_err("temperature above provider range must fail");

        assert_eq!(
            error.to_string(),
            "DASHSCOPE_TEMPERATURE must be in the range 0.0..=2.0"
        );
    }

    #[test]
    fn configured_openai_compatible_profiles_are_safe_and_selectable() {
        let profiles = r#"[
          {
            "id": "qwen-research",
            "provider": "qwen",
            "display_name": "Research Qwen",
            "base_url": "https://dashscope.aliyuncs.com/compatible-mode",
            "api_key_env": "RESEARCH_QWEN_KEY",
            "model": "qwen-plus",
            "default": true,
            "capabilities": {"market_evidence": true, "restricted_policy_drafts": true}
          },
          {
            "id": "local-reviewer",
            "provider": "local-ai",
            "display_name": "Local reviewer",
            "base_url": "http://127.0.0.1:11434/v1",
            "api_key_env": "LOCAL_REVIEWER_KEY",
            "model": "reviewer",
            "capabilities": {"market_evidence": false, "restricted_policy_drafts": true},
            "max_tokens": 512
          }
        ]"#;
        let config = parse(&[
            (AI_PROVIDER_PROFILES, profiles),
            ("RESEARCH_QWEN_KEY", "first-secret"),
            ("LOCAL_REVIEWER_KEY", "second-secret"),
        ])
        .expect("safe configured profiles should parse");

        assert_eq!(config.ai_providers.len(), 2);
        assert_eq!(
            config.ai_providers[0].profile.id().as_str(),
            "qwen-research"
        );
        assert!(
            config.ai_providers[0]
                .profile
                .capabilities()
                .restricted_policy_drafts
        );
        assert!(config.ai_providers[0].is_default);
        assert_eq!(config.ai_providers[1].client.max_tokens, 512);
        let debug = format!("{config:?}");
        assert!(!debug.contains("first-secret"));
        assert!(!debug.contains("second-secret"));
    }

    #[test]
    fn unsafe_or_ambiguous_provider_profiles_fail_without_echoing_secrets() {
        let remote_http = r#"[{"id":"bad","provider":"test","display_name":"Bad","base_url":"http://example.com","api_key_env":"TEST_KEY","model":"x","default":true}]"#;
        assert!(matches!(
            parse(&[
                (AI_PROVIDER_PROFILES, remote_http),
                ("TEST_KEY", "top-secret")
            ]),
            Err(ConfigError::InvalidAiProviderBaseUrl)
        ));
        let missing_default = r#"[{"id":"one","provider":"test","display_name":"One","base_url":"https://example.com","api_key_env":"TEST_KEY","model":"x"}]"#;
        let error = parse(&[
            (AI_PROVIDER_PROFILES, missing_default),
            ("TEST_KEY", "top-secret"),
        ])
        .expect_err("one explicit default is required");
        assert!(matches!(error, ConfigError::InvalidAiProviderProfiles));
        assert!(!error.to_string().contains("top-secret"));
    }

    /// Verify OpenD stays disabled unless a supported provider is explicitly configured.
    #[test]
    fn opend_configuration_is_optional_and_paper_only() {
        let config = parse(&[
            (OPEND_PROVIDER, "moomoo"),
            (OPEND_HOST, "localhost"),
            (OPEND_PORT, "11111"),
            (OPEND_ACCOUNT_ID, " paper-account "),
        ])
        .unwrap();
        let opend = config.opend.expect("provider enables OpenD configuration");

        assert_eq!(opend.provider(), BrokerProvider::Moomoo);
        assert_eq!(opend.host(), DEFAULT_OPEND_HOST);
        assert_eq!(opend.port(), 11111);
        assert_eq!(opend.account_id(), Some("paper-account"));
        assert_eq!(opend.environment(), broker::BrokerEnvironment::Paper);
        assert!(!opend.live_trading_enabled());
    }

    /// Verify invalid OpenD provider and non-loopback hosts fail before server startup.
    #[test]
    fn unsafe_opend_configuration_is_rejected() {
        assert!(matches!(
            parse(&[(OPEND_PROVIDER, "other")]),
            Err(ConfigError::InvalidOpenDProvider)
        ));
        assert!(matches!(
            parse(&[(OPEND_PROVIDER, "futu"), (OPEND_HOST, "opend.example")]),
            Err(ConfigError::OpenDLoopbackRequired)
        ));
    }

    /// Verify every literal loopback form is normalized without hostname resolution.
    #[test]
    fn opend_configuration_normalizes_loopback_literals() {
        for (input, expected) in [
            ("LOCALHOST", "127.0.0.1"),
            ("127.0.0.2", "127.0.0.2"),
            ("0:0:0:0:0:0:0:1", "::1"),
        ] {
            let config = parse(&[(OPEND_PROVIDER, "futu"), (OPEND_HOST, input)])
                .expect("loopback host should be accepted");

            assert_eq!(
                config
                    .opend
                    .expect("provider enables OpenD configuration")
                    .host(),
                expected
            );
        }
    }
}
