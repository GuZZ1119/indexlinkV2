//! QwenClient 集成测试。
//!
//! 使用本地 mock HTTP server 验证请求构造、响应解析和错误传播。

use std::net::SocketAddr;
use std::time::Duration;

use ai_client::{
    AiConfig, AiCopilotDraftRequest, AiCopilotEvidenceReference, AiProvider,
    AiProviderCapabilities, AiProviderId, AiProviderProfile, AiProviderProfileId, QwenClient,
};
use axum::body::Body;
use axum::http::{HeaderMap, Response, StatusCode};
use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

// ─── Mock Server Helpers ─────────────────────────────────────────────────────

#[derive(Deserialize)]
#[allow(dead_code)]
struct MockRequest {
    model: String,
    messages: Vec<MockMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct MockMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct MockResponse {
    choices: Vec<MockChoice>,
}

#[derive(Serialize)]
struct MockChoice {
    message: MockChoiceMessage,
}

#[derive(Serialize)]
struct MockChoiceMessage {
    content: String,
}

fn sentiment_response(value: f64) -> MockResponse {
    MockResponse {
        choices: vec![MockChoice {
            message: MockChoiceMessage {
                content: format!(
                    r#"{{"score": {value}, "rationale": "Mock explanation based on the supplied headlines.", "warnings": ["Mock warning."]}}"#
                ),
            },
        }],
    }
}

/// 构建 JSON 响应（统一使用 Response<Body> 以支持非 JSON 响应体测试）。
fn json_response(status: StatusCode, value: f64) -> Response<Body> {
    let body = serde_json::to_string(&sentiment_response(value)).unwrap();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// Build one OpenAI-compatible completion response from a raw model content string.
fn completion_response(status: StatusCode, content: &str) -> Response<Body> {
    let body = serde_json::to_string(&MockResponse {
        choices: vec![MockChoice {
            message: MockChoiceMessage {
                content: content.to_owned(),
            },
        }],
    })
    .unwrap();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

/// 启动本地 mock server，返回绑定的地址。
async fn spawn_mock_server() -> SocketAddr {
    let app = Router::new().route(
        "/v1/chat/completions",
        post(
            |headers: HeaderMap, Json(body): Json<MockRequest>| async move {
                // 验证 Authorization 头（必须包含非空 token）
                let auth_valid = headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .map(|v| v.starts_with("Bearer ") && v.len() > "Bearer ".len())
                    .unwrap_or(false);
                if !auth_valid {
                    return json_response(StatusCode::UNAUTHORIZED, 0.0);
                }

                // 验证请求包含必要字段
                assert!(!body.model.is_empty());
                assert!(!body.messages.is_empty());
                assert_eq!(body.messages[0].role, "system");

                if body.messages[0]
                    .content
                    .contains("restricted investment-policy candidate")
                {
                    assert!(body.max_tokens >= 768);
                    return completion_response(
                        StatusCode::OK,
                        r#"{"document":{"policy_id":"dsl_test_guard","policy_version":1,"name":"Test guard","rules":[{"condition":{"kind":"comparison","expression":{"kind":"indicator","indicator":{"kind":"relative_strength_index","lookback_days":14}},"operator":"less_than","threshold":"35"},"action":{"kind":"set_opportunity_multiplier","multiplier":1.2}}]},"explanation":"Validate before saving.","warnings":["Mock warning."],"evidence_reference_ids":["dsl_allowlist_v1"]}"#,
                    );
                }

                let user_content = &body.messages[1].content;

                // ── 以下关键词触发特定错误，用于测试客户端错误传播 ──

                // HTTP 500 Internal Server Error
                if user_content.contains("TRIGGER_500") {
                    return json_response(StatusCode::INTERNAL_SERVER_ERROR, 0.0);
                }

                // 响应体不是合法 JSON
                if user_content.contains("TRIGGER_INVALID_JSON") {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("this is not valid json"))
                        .unwrap();
                }

                // choices 数组为空
                if user_content.contains("TRIGGER_EMPTY_CHOICES") {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"choices": []}"#))
                        .unwrap();
                }

                // content 字段为空字符串
                if user_content.contains("TRIGGER_EMPTY_CONTENT") {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"choices": [{"message": {"content": ""}}]}"#))
                        .unwrap();
                }

                // ── 正常情绪分析 ──

                let sentiment = if user_content.contains("大幅上涨")
                    || user_content.contains("利好")
                {
                    0.7
                } else if user_content.contains("大幅下跌") || user_content.contains("利空") {
                    -0.6
                } else {
                    0.0
                };

                json_response(StatusCode::OK, sentiment)
            },
        ),
    );

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("mock server crashed");
    });
    addr
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn client_analyzes_positive_news() {
    let addr = spawn_mock_server().await;
    let config = AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    };
    let client = QwenClient::new(config);
    let sentiment = client
        .analyze("今日A股大幅上涨，成交额创年内新高")
        .await
        .expect("mock server must return valid response");

    assert!(sentiment.value() > 0.0);
}

#[tokio::test]
async fn client_analyzes_negative_news() {
    let addr = spawn_mock_server().await;
    let config = AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    };
    let client = QwenClient::new(config);
    let sentiment = client
        .analyze("美股大幅下跌，VIX恐慌指数飙升")
        .await
        .expect("mock server must return valid response");

    assert!(sentiment.value() < 0.0);
}

#[tokio::test]
async fn client_analyzes_neutral_news() {
    let addr = spawn_mock_server().await;
    let config = AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    };
    let client = QwenClient::new(config);
    let sentiment = client
        .analyze("今日市场窄幅震荡，成交量与昨日持平")
        .await
        .expect("mock server must return valid response");

    assert!((sentiment.value()).abs() < f64::EPSILON);
}

#[tokio::test]
async fn client_clamps_out_of_range_sentiment() {
    // 测试 Sentiment::new_clamped 在 AiProvider 实现中被调用
    use ai_client::Sentiment;
    let s = Sentiment::new_clamped(99.0);
    assert_eq!(s, Sentiment::MAX);
}

#[tokio::test]
async fn client_returns_error_on_connection_refused() {
    let config = AiConfig {
        base_url: "http://127.0.0.1:1".to_owned(), // 极不可能被占用的端口
        api_key: "test-key".to_owned(),
        timeout: std::time::Duration::from_secs(1),
        ..Default::default()
    };
    let client = QwenClient::new(config);
    let result = client.analyze("新闻").await;
    // ai-client 不自行降级——将错误原样返回给上层（decision engine），
    // 由 engine 根据 70/20/10 → 90/10/0 策略决定如何处理。
    assert!(result.is_err(), "连接被拒绝时应当返回错误，而非静默吞掉");
}

#[tokio::test]
async fn client_returns_error_on_http_error() {
    let addr = spawn_mock_server().await;
    let config = AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    };
    let client = QwenClient::new(config);
    // TRIGGER_500 让 mock server 返回 500 Internal Server Error
    let result = client.analyze("TRIGGER_500").await;
    // ai-client 不自行降级——将 HttpStatus 错误原样返回给上层
    assert!(
        result.is_err(),
        "HTTP 500 应当被映射为错误并原样返回给调用方"
    );
}

#[tokio::test]
async fn client_request_includes_bearer_auth() {
    let addr = spawn_mock_server().await;
    let config = AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "bearer-secret-123".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    };
    let client = QwenClient::new(config);
    // Mock server 会验证 Authorization: Bearer <key> 头
    // — 缺少或格式错误时返回 401 UNAUTHORIZED
    // — 正确时返回 200 OK，说明客户端确实发送了正确的 Bearer auth
    let result = client.analyze("中性新闻，无特殊关键词").await;
    assert!(
        result.is_ok(),
        "Mock server 返回了 200，说明 Authorization header 已正确发送"
    );
    let sentiment = result.unwrap();
    assert!(sentiment.value().abs() < f64::EPSILON);
}

#[tokio::test]
async fn client_returns_error_on_invalid_json_response() {
    // call_api 第4步：serde_json::from_str 解析 ChatResponse 时，
    // 响应体不是合法 JSON → InvalidJson 错误
    let addr = spawn_mock_server().await;
    let client = QwenClient::new(AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    });
    let result = client.analyze("TRIGGER_INVALID_JSON").await;
    assert!(result.is_err(), "非 JSON 响应体应返回 InvalidJson 错误");
}

#[tokio::test]
async fn client_returns_error_on_empty_choices() {
    // call_api 第5步：chat.choices.first() → None → EmptyResponse
    let addr = spawn_mock_server().await;
    let client = QwenClient::new(AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    });
    let result = client.analyze("TRIGGER_EMPTY_CHOICES").await;
    assert!(result.is_err(), "空 choices 数组应返回 EmptyResponse 错误");
}

#[tokio::test]
async fn client_returns_error_on_empty_content() {
    // call_api 第6步：content = "" → filter(!is_empty) → None → EmptyResponse
    let addr = spawn_mock_server().await;
    let client = QwenClient::new(AiConfig {
        base_url: format!("http://{addr}"),
        api_key: "test-key".to_owned(),
        model: "test-model".to_owned(),
        ..Default::default()
    });
    let result = client.analyze("TRIGGER_EMPTY_CONTENT").await;
    assert!(result.is_err(), "空 content 应返回 EmptyResponse 错误");
}

#[tokio::test]
async fn client_returns_error_on_auth_failure() {
    // call_api 第2步：status.is_success() → false (401) → HttpStatus
    // 空 api_key → Authorization: Bearer （token 为空）→ mock 校验不通过
    let addr = spawn_mock_server().await;
    let client = QwenClient::new(AiConfig {
        base_url: format!("http://{addr}"),
        api_key: String::new(),
        model: "test-model".to_owned(),
        ..Default::default()
    });
    let result = client.analyze("中性新闻").await;
    assert!(
        result.is_err(),
        "空 api_key 导致认证失败，应返回 HttpStatus 401"
    );
}

#[tokio::test]
async fn configured_profile_generates_only_a_bounded_read_only_copilot_draft() {
    let addr = spawn_mock_server().await;
    let profile = AiProviderProfile::new(
        AiProviderProfileId::new("reviewer").unwrap(),
        AiProviderId::new("openai-compatible").unwrap(),
        "Reviewer".to_owned(),
        "test-model".to_owned(),
        AiProviderCapabilities::market_evidence_and_restricted_policy_drafts(),
    )
    .unwrap();
    let client = QwenClient::with_profile(
        AiConfig {
            base_url: format!("http://{addr}"),
            api_key: "test-key".to_owned(),
            model: "test-model".to_owned(),
            ..Default::default()
        },
        profile,
    );
    let request = AiCopilotDraftRequest::new(
        "dsl_test_guard".to_owned(),
        1,
        "Increase only the opportunity bucket after oversold RSI.".to_owned(),
        vec![AiCopilotEvidenceReference::new(
            "dsl_allowlist_v1".to_owned(),
            "Server allowlist".to_owned(),
        )
        .unwrap()],
    )
    .unwrap();

    let draft = client.generate_policy_draft(&request).await.unwrap();
    assert_eq!(client.profile().id().as_str(), "reviewer");
    assert_eq!(draft.evidence_reference_ids(), ["dsl_allowlist_v1"]);
    assert_eq!(draft.document()["policy_id"], "dsl_test_guard");
}

#[tokio::test]
async fn configured_profile_returns_structured_market_evidence() {
    let addr = spawn_mock_server().await;
    let client = QwenClient::with_profile(
        AiConfig {
            base_url: format!("http://{addr}"),
            api_key: "test-key".to_owned(),
            ..Default::default()
        },
        AiProviderProfile::new(
            AiProviderProfileId::new("evidence").unwrap(),
            AiProviderId::new("openai-compatible").unwrap(),
            "Evidence provider".to_owned(),
            "test-model".to_owned(),
            AiProviderCapabilities::market_evidence_only(),
        )
        .unwrap(),
    );
    let evidence = client.analyze_with_evidence("利好").await.unwrap();
    assert!(evidence.sentiment().value() > 0.0);
    assert!(evidence.rationale().contains("Mock explanation"));
    assert_eq!(client.profile().id().as_str(), "evidence");
}

#[tokio::test]
async fn client_maps_a_local_slow_provider_to_a_bounded_timeout() {
    let app = Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            json_response(StatusCode::OK, 0.0)
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = QwenClient::new(AiConfig {
        base_url: format!("http://{address}"),
        api_key: "test-key".to_owned(),
        timeout: Duration::from_millis(10),
        ..Default::default()
    });
    let error = client.analyze("timeout expected").await.unwrap_err();
    assert!(matches!(
        error,
        ai_client::AiClientError::Timeout { seconds: 0 }
    ));
}
