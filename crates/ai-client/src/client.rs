//! OpenAI 兼容 API 客户端。
//!
//! [`QwenClient`] 实现 [`AiProvider`] trait，
//! 对接 Qwen DashScope / OpenAI / 任何兼容 `/v1/chat/completions` 的服务。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{AiClientError, AiConfig, AiProvider, AiProviderProfile, Sentiment, SentimentAnalysis};

// ─── System Prompt ───────────────────────────────────────────────────────────

/// 系统提示词：指导 LLM 输出可审计的结构化情绪 JSON。
///
/// 设计要点：
/// - 强制 JSON-only 输出，禁止附带解释文本
/// - 保守打分：无明确方向信号时倾向近 0
/// - 分类指引覆盖财报、宏观、政策等主要场景
const SYSTEM_PROMPT: &str = "\
You are a financial sentiment analyzer. Analyze the given financial news and \
output ONLY a JSON object with \"score\", \"rationale\", and \"warnings\" fields.

Output format (exactly):
{\"score\": <float between -1.0 and +1.0>, \"rationale\": \"<brief evidence-based explanation>\", \"warnings\": [\"<optional concise risk warning>\"]}

Scoring guide:
- +1.0: Strong bullish signal (major earnings beat, positive macro surprise, \
central bank dovish pivot)
- +0.5: Moderate bullish (minor beat, favorable guidance, sector tailwind)
- +0.1 to +0.3: Slightly positive tone (in-line results with optimistic commentary)
- 0.0: Neutral or mixed signals, no clear directional bias
- -0.1 to -0.3: Slightly negative tone (minor miss, cautious commentary)
- -0.5: Moderate bearish (guidance cut, sector headwinds, trade friction)
- -1.0: Strong bearish signal (major miss, systemic risk event, credit event)

IMPORTANT: Be conservative. Unless there is a clear directional signal, output a \
value close to 0. The rationale must only summarize the supplied headlines, not \
invent facts, forecasts, URLs, or sources. Return at most five warnings. Do NOT \
include any text other than the JSON object.";

// ─── Request / Response Types ────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct SentimentResponse {
    score: f64,
    rationale: String,
    #[serde(default)]
    warnings: Vec<String>,
}

// ─── QwenClient ────────────────────────────────────────────────────────────

/// OpenAI 兼容 API 客户端（支持 Qwen / OpenAI / 任何兼容服务）。
///
/// # 错误处理
///
/// 任何错误（超时、网络、HTTP 状态码、解析失败）都以 [`AiClientError`]
/// 返回给调用方。ai-client 不自行降级——由上层 decision engine
/// 按 70/20/10 → 90/10/0 策略统一处理。
///
/// # 示例
///
/// ```rust,no_run
/// use ai_client::{AiConfig, AiProvider, QwenClient, Sentiment};
///
/// # async fn example() {
/// let config = AiConfig::default();
/// let client = QwenClient::new(config);
/// match client
///     .analyze("央行宣布降准50bp，释放长期流动性约1万亿元")
///     .await
/// {
///     Ok(sentiment) => { /* engine 使用 70/20/10 */ }
///     Err(err) => { /* engine 降级到 90/10/0 */ }
/// }
/// # }
/// ```
pub struct QwenClient {
    http: reqwest::Client,
    config: AiConfig,
}

impl QwenClient {
    /// 使用给定配置创建客户端。
    ///
    /// 若 `api_key` 为空字符串，客户端仍可创建但所有请求将因认证失败而返回错误。
    /// 这遵循「延迟失败」原则——直到实际调用时才报错，便于测试和配置热更新。
    #[must_use]
    pub fn new(config: AiConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("reqwest::Client::builder with standard options must not fail");
        Self { http, config }
    }

    /// 构造请求体。
    fn build_request(&self, prompt: &str) -> ChatRequest {
        ChatRequest {
            model: self.config.model.clone(),
            messages: vec![
                Message {
                    role: "system",
                    content: SYSTEM_PROMPT.to_owned(),
                },
                Message {
                    role: "user",
                    content: prompt.to_owned(),
                },
            ],
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        }
    }

    /// 拼接 chat completions 端点 URL。
    ///
    /// 若 `base_url` 已包含 `/v1` 则不再重复拼接，避免出现
    /// `/v1/v1/chat/completions` 的重复路径。
    fn chat_url(&self) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        if base.ends_with("/v1") {
            format!("{base}/chat/completions")
        } else {
            format!("{base}/v1/chat/completions")
        }
    }

    /// 执行 HTTP 请求并解析响应。
    async fn call_api(&self, prompt: &str) -> Result<SentimentAnalysis, AiClientError> {
        let url = self.chat_url();
        let body = self.build_request(prompt);

        debug!(url = %url, model = %self.config.model, "sending AI sentiment request");

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    warn!(
                        seconds = self.config.timeout.as_secs(),
                        "AI service request timed out"
                    );
                    AiClientError::Timeout {
                        seconds: self.config.timeout.as_secs(),
                    }
                } else {
                    warn!(?err, "AI service transport error");
                    AiClientError::Transport(err)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            warn!(
                status = status.as_u16(),
                "AI service returned non-success status"
            );
            return Err(AiClientError::HttpStatus {
                status: status.as_u16(),
            });
        }

        let body = response.text().await.map_err(|err| {
            warn!(?err, "failed to read AI service response body");
            AiClientError::Transport(err)
        })?;

        let chat: ChatResponse = serde_json::from_str(&body).map_err(|err| {
            warn!(?err, "failed to parse AI service response as JSON");
            AiClientError::InvalidJson(err)
        })?;

        let content = chat
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                warn!("AI service returned empty or missing choices");
                AiClientError::EmptyResponse
            })?;

        debug!(content = %content, "received AI model output");

        let sentiment = parse_sentiment_from_llm_output(content)?;

        Ok(sentiment)
    }
}

/// 从 LLM 原始输出中提取结构化情绪分析。
///
/// LLM 输出不可靠——可能返回纯 JSON，
/// 也可能在 JSON 外包了 markdown 或解释文本。此函数优先直接解析，
/// 失败时尝试从文本中提取 `{...}` 块再解析。两者都失败才返回错误。
fn parse_sentiment_from_llm_output(content: &str) -> Result<SentimentAnalysis, AiClientError> {
    // 第一遍：直接解析
    if let Ok(parsed) = serde_json::from_str::<SentimentResponse>(content) {
        return sentiment_analysis_from_response(parsed);
    }

    // 第二遍：尝试提取 JSON 对象 { ... }
    if let Some(json_block) = extract_json_object(content) {
        if let Ok(parsed) = serde_json::from_str::<SentimentResponse>(&json_block) {
            debug!("extracted sentiment from embedded JSON block");
            return sentiment_analysis_from_response(parsed);
        }
    }

    warn!(content, "failed to parse sentiment from model output");
    Err(AiClientError::ParseFailure)
}

/// Validate one model JSON object before it crosses the provider boundary.
fn sentiment_analysis_from_response(
    response: SentimentResponse,
) -> Result<SentimentAnalysis, AiClientError> {
    SentimentAnalysis::new(
        Sentiment::new_clamped(response.score),
        response.rationale,
        response.warnings,
    )
    .map_err(|_| AiClientError::ParseFailure)
}

/// 从文本中提取第一个 `{ ... }` JSON 对象（平衡括号匹配）。
fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0u32;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + 1].to_owned());
                }
            }
            _ => {}
        }
    }
    None
}

#[async_trait]
impl AiProvider for QwenClient {
    fn profile(&self) -> AiProviderProfile {
        AiProviderProfile::qwen(self.config.model.clone())
    }

    async fn analyze(&self, prompt: &str) -> Result<Sentiment, AiClientError> {
        Ok(self.call_api(prompt).await?.sentiment())
    }

    async fn analyze_with_evidence(
        &self,
        prompt: &str,
    ) -> Result<SentimentAnalysis, AiClientError> {
        self.call_api(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_includes_user_prompt() {
        let config = AiConfig {
            model: "test-model".to_owned(),
            ..Default::default()
        };
        let client = QwenClient::new(config);
        let req = client.build_request("沪深300指数今日大幅上涨");

        assert_eq!(req.model, "test-model");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.messages[0].role, "system");
        assert!(req.messages[0].content.contains("rationale"));
        assert_eq!(req.messages[1].role, "user");
        assert_eq!(req.messages[1].content, "沪深300指数今日大幅上涨");
        assert_eq!(req.temperature, 0.0);
        assert_eq!(req.max_tokens, 256);
    }

    #[test]
    fn chat_url_does_not_double_v1() {
        // 若 base_url 已包含 /v1，不应再拼接一次
        let config = AiConfig {
            base_url: "https://api.openai.com/v1".to_owned(),
            ..Default::default()
        };
        let client = QwenClient::new(config);
        assert_eq!(
            client.chat_url(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn chat_url_appends_v1_when_missing() {
        let config = AiConfig {
            base_url: "https://api.example.com".to_owned(),
            ..Default::default()
        };
        let client = QwenClient::new(config);
        assert_eq!(
            client.chat_url(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn chat_url_without_trailing_slash() {
        let config = AiConfig::default();
        let client = QwenClient::new(config);
        assert_eq!(
            client.chat_url(),
            "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
        );
    }

    #[test]
    fn parse_valid_sentiment_json() {
        let content = r#"{"score": 0.7, "rationale": "Inflation eased.", "warnings": []}"#;
        let parsed: SentimentResponse =
            serde_json::from_str(content).expect("valid sentiment JSON must parse");
        assert!((parsed.score - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_negative_sentiment_json() {
        let content = r#"{"score": -0.5, "rationale": "Guidance weakened.", "warnings": ["Volatility may rise."]}"#;
        let parsed: SentimentResponse =
            serde_json::from_str(content).expect("valid sentiment JSON must parse");
        assert!((parsed.score - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_sentiment_json_rejects_missing_field() {
        let content = r#"{"other": 0.5}"#;
        let result = serde_json::from_str::<SentimentResponse>(content);
        assert!(result.is_err());
    }

    #[test]
    fn client_constructs_without_panic() {
        let config = AiConfig::default();
        let client = QwenClient::new(config);
        assert!(client.chat_url().contains("dashscope"));
    }

    #[test]
    fn system_prompt_includes_required_fields() {
        assert!(SYSTEM_PROMPT.contains("rationale"));
        assert!(SYSTEM_PROMPT.contains("warnings"));
        assert!(SYSTEM_PROMPT.contains("-1.0"));
        assert!(SYSTEM_PROMPT.contains("+1.0"));
        assert!(SYSTEM_PROMPT.contains("JSON"));
    }

    // ── extract_json_object ─────────────────────────────────────────────────

    #[test]
    fn extract_pure_json() {
        let content = r#"{"score": 0.7, "rationale": "Test rationale.", "warnings": []}"#;
        let extracted = extract_json_object(content).unwrap();
        assert_eq!(extracted, content);
    }

    #[test]
    fn extract_json_from_markdown_code_block() {
        let content =
            "```json\n{\"score\": 0.5, \"rationale\": \"Test rationale.\", \"warnings\": []}\n```";
        let extracted = extract_json_object(content).unwrap();
        assert_eq!(
            extracted,
            r#"{"score": 0.5, "rationale": "Test rationale.", "warnings": []}"#
        );
    }

    #[test]
    fn extract_json_with_prefix_text() {
        let content = "以下是分析结果：{\"score\": -0.3, \"rationale\": \"Test rationale.\", \"warnings\": []}，仅供参考。";
        let extracted = extract_json_object(content).unwrap();
        assert_eq!(
            extracted,
            r#"{"score": -0.3, "rationale": "Test rationale.", "warnings": []}"#
        );
    }

    #[test]
    fn extract_nested_json() {
        let content = r#"{"outer": {"score": 0.8}}"#;
        let extracted = extract_json_object(content).unwrap();
        assert_eq!(extracted, content);
    }

    #[test]
    fn extract_json_no_braces_returns_none() {
        assert!(extract_json_object("no json here").is_none());
    }

    #[test]
    fn extract_json_unclosed_brace_returns_none() {
        assert!(extract_json_object(r#"{"score": 0.5"#).is_none());
    }

    // ── parse_sentiment_from_llm_output ──────────────────────────────────────

    #[test]
    fn parse_sentiment_from_pure_json() {
        let analysis = parse_sentiment_from_llm_output(r#"{"score": 0.7, "rationale": "Inflation eased.", "warnings": ["Rates remain uncertain."]}"#).unwrap();
        assert!((analysis.sentiment().value() - 0.7).abs() < f64::EPSILON);
        assert_eq!(analysis.rationale(), "Inflation eased.");
        assert_eq!(analysis.warnings(), ["Rates remain uncertain."]);
    }

    #[test]
    fn parse_sentiment_from_markdown_wrapped_json() {
        // LLM 常见输出：在 markdown 代码块中返回 JSON
        let content = "```json\n{\"score\": -0.5, \"rationale\": \"Guidance weakened.\", \"warnings\": []}\n```";
        let analysis = parse_sentiment_from_llm_output(content).unwrap();
        assert!((analysis.sentiment().value() - (-0.5)).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_sentiment_with_explanatory_prefix() {
        // LLM 有时在 JSON 前加解释文本
        let content = "根据分析，我认为市场情绪偏正面。\n{\"score\": 0.3, \"rationale\": \"Earnings improved.\", \"warnings\": []}";
        let analysis = parse_sentiment_from_llm_output(content).unwrap();
        assert!((analysis.sentiment().value() - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_sentiment_fails_on_unparseable_output() {
        let content = "抱歉，我无法分析这条新闻，因为它不包含足够的金融信息。";
        let result = parse_sentiment_from_llm_output(content);
        assert!(result.is_err());
    }

    #[test]
    fn parse_sentiment_fails_on_missing_field() {
        let content = r#"{"score": 0.5}"#;
        let result = parse_sentiment_from_llm_output(content);
        assert!(result.is_err());
    }
}
