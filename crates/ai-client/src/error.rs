//! AI 客户端错误类型。
//!
//! 所有错误变体的 [`Display`] 实现均**不暴露** API Key、URL 或请求体内容。
//! 遵循项目安全规范：对外错误信息仅描述故障类别，不泄露内部细节。

/// AI 客户端可能产生的错误。
///
/// 调用方应将错误原样传播到 decision engine，
/// 由 engine 按 70/20/10 → 90/10/0 策略处理 AI 不可用的情况。
#[derive(Debug, thiserror::Error)]
pub enum AiClientError {
    /// 请求超时。
    #[error("AI service request timed out after {seconds} seconds")]
    Timeout {
        /// 超时秒数。
        seconds: u64,
    },

    /// HTTP 传输层错误（DNS 解析、TLS 握手、连接被拒、或响应流中断）。
    ///
    /// 涵盖 `reqwest` 在连接建立与响应体读取两个阶段的所有 IO 错误。
    /// 调用方无需区分具体阶段——任何传输错误都意味着 AI 服务不可达。
    #[error("AI service request failed")]
    Transport(#[source] reqwest::Error),

    /// API 返回非成功状态码（4xx/5xx）。
    #[error("AI service returned HTTP {status}")]
    HttpStatus {
        /// HTTP 状态码。
        status: u16,
    },

    /// 响应体不是有效的 JSON。
    #[error("AI service response was not valid JSON")]
    InvalidJson(#[source] serde_json::Error),

    /// JSON 结构不符合预期格式（缺少 choices、content 等）。
    #[error("AI service response had unexpected structure")]
    UnexpectedStructure,

    /// 模型返回的内容无法解析为情绪值。
    #[error("AI service returned unparseable sentiment value")]
    ParseFailure,

    /// 模型返回了空内容。
    #[error("AI service returned empty content")]
    EmptyResponse,

    /// 当前已部署的 provider 未声明所请求的受限能力。
    #[error("AI service does not support this capability")]
    UnsupportedCapability,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_display_is_safe() {
        let error = AiClientError::Timeout { seconds: 30 };
        let display = error.to_string();
        assert!(display.contains("30 seconds"));
        assert!(!display.contains("http"));
        assert!(!display.contains("api_key"));
        assert!(!display.contains("Bearer"));
    }

    // Transport 的 Display 是固定字符串 "AI service request failed"，
    // 由 `#[error("...")]` 编译期保证，不包含任何运行时动态内容。

    #[test]
    fn http_status_display_is_safe() {
        let error = AiClientError::HttpStatus { status: 401 };
        assert_eq!(error.to_string(), "AI service returned HTTP 401");
    }

    #[test]
    fn invalid_json_display_is_safe() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").expect_err("");
        let error = AiClientError::InvalidJson(json_err);
        let display = error.to_string();
        assert_eq!(display, "AI service response was not valid JSON");
    }

    #[test]
    fn unexpected_structure_display_is_safe() {
        assert_eq!(
            AiClientError::UnexpectedStructure.to_string(),
            "AI service response had unexpected structure"
        );
    }

    #[test]
    fn parse_failure_display_is_safe() {
        assert_eq!(
            AiClientError::ParseFailure.to_string(),
            "AI service returned unparseable sentiment value"
        );
    }

    #[test]
    fn empty_response_display_is_safe() {
        assert_eq!(
            AiClientError::EmptyResponse.to_string(),
            "AI service returned empty content"
        );
    }

    #[test]
    fn all_error_variants_never_contain_secret_patterns() {
        let errors: Vec<AiClientError> = vec![
            AiClientError::Timeout { seconds: 10 },
            AiClientError::HttpStatus { status: 500 },
            AiClientError::UnexpectedStructure,
            AiClientError::ParseFailure,
            AiClientError::EmptyResponse,
            AiClientError::UnsupportedCapability,
        ];

        for error in errors {
            let display = error.to_string();
            let debug = format!("{error:?}");
            for text in [&display, &debug] {
                assert!(!text.contains("sk-"));
                assert!(!text.contains("Bearer"));
                assert!(!text.contains("dashscope"));
                assert!(!text.contains("api_key"));
                assert!(!text.contains("key="));
            }
        }
    }

    #[test]
    fn unsupported_capability_display_is_safe() {
        assert_eq!(
            AiClientError::UnsupportedCapability.to_string(),
            "AI service does not support this capability"
        );
    }
}
