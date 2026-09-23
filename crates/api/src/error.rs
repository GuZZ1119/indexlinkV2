use ai_client::PipelineError;
use axum::{http::StatusCode, response::IntoResponse, Json};
use broker::BrokerError;
use decision_records::{DecisionRecordApplicationError, ManualExecutionApplicationError};
use investment_plans::{PlanApplicationError, PlanValidationError};
use serde::Serialize;

/// API 错误。
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// 请求参数或载荷未通过校验。
    #[error("bad request")]
    BadRequest,
    /// 请求的资源不存在。
    #[error("not found")]
    NotFound,
    /// The request conflicts with an existing immutable resource.
    #[error("conflict")]
    Conflict,
    /// 订单结果尚未确认，客户端不得自动重试。
    #[error("order outcome is unknown")]
    OrderOutcomeUnknown,
    /// 依赖服务当前不可用。
    #[error("service unavailable")]
    ServiceUnavailable,
    /// The AI provider answered, but its outer structured response was invalid.
    #[error("AI response is invalid")]
    AiResponseInvalid,
    /// The AI form candidate could not compile into the bounded Strategy Workshop contract.
    #[error("AI draft is invalid")]
    AiDraftInvalid,
}

/// 统一错误响应外层结构。
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    /// 错误详情。
    pub error: ErrorBody,
}

/// 对客户端安全的错误详情。
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// 稳定的机器可读错误码。
    pub code: &'static str,
    /// 不包含内部实现细节的错误消息。
    pub message: &'static str,
    /// 可选请求标识，供后续链路追踪扩展。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match self {
            Self::BadRequest => (
                StatusCode::BAD_REQUEST,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "bad_request",
                        message: "invalid request",
                        request_id: None,
                    },
                },
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "not_found",
                        message: "resource not found",
                        request_id: None,
                    },
                },
            ),
            Self::Conflict => (
                StatusCode::CONFLICT,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "conflict",
                        message: "resource already exists",
                        request_id: None,
                    },
                },
            ),
            Self::OrderOutcomeUnknown => (
                StatusCode::CONFLICT,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "order_outcome_unknown",
                        message: "order outcome is unknown; do not retry",
                        request_id: None,
                    },
                },
            ),
            Self::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "service_unavailable",
                        message: "service is unavailable",
                        request_id: None,
                    },
                },
            ),
            Self::AiResponseInvalid => (
                StatusCode::BAD_GATEWAY,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "ai_response_invalid",
                        message: "AI response did not match the required structure",
                        request_id: None,
                    },
                },
            ),
            Self::AiDraftInvalid => (
                StatusCode::BAD_GATEWAY,
                ErrorEnvelope {
                    error: ErrorBody {
                        code: "ai_draft_invalid",
                        message: "AI draft did not match the strategy form contract",
                        request_id: None,
                    },
                },
            ),
        };

        (status, Json(body)).into_response()
    }
}

impl From<PlanApplicationError> for ApiError {
    /// Convert application-layer errors into safe API errors.
    fn from(error: PlanApplicationError) -> Self {
        match error {
            PlanApplicationError::Validation(_) => Self::BadRequest,
            PlanApplicationError::NotFound => Self::NotFound,
            PlanApplicationError::Unavailable => Self::ServiceUnavailable,
        }
    }
}

impl From<PlanValidationError> for ApiError {
    /// Convert validation errors into the public bad-request envelope.
    fn from(_: PlanValidationError) -> Self {
        Self::BadRequest
    }
}

impl From<DecisionRecordApplicationError> for ApiError {
    /// Convert decision-record application errors into safe API errors.
    fn from(error: DecisionRecordApplicationError) -> Self {
        match error {
            DecisionRecordApplicationError::Validation(_) => Self::BadRequest,
            DecisionRecordApplicationError::NotFound => Self::NotFound,
            DecisionRecordApplicationError::Unavailable => Self::ServiceUnavailable,
        }
    }
}

impl From<ManualExecutionApplicationError> for ApiError {
    /// Convert manual execution journal errors into safe API errors.
    fn from(error: ManualExecutionApplicationError) -> Self {
        match error {
            ManualExecutionApplicationError::Validation(_) => Self::BadRequest,
            ManualExecutionApplicationError::NotFound => Self::NotFound,
            ManualExecutionApplicationError::AlreadyExists => Self::Conflict,
            ManualExecutionApplicationError::Unavailable => Self::ServiceUnavailable,
        }
    }
}

impl From<BrokerError> for ApiError {
    /// Convert broker-layer errors into safe API errors.
    fn from(error: BrokerError) -> Self {
        match error {
            BrokerError::Validation(_)
            | BrokerError::LiveTradingDisabled
            | BrokerError::EnvironmentMismatch { .. }
            | BrokerError::PaperTradingRequired { .. }
            | BrokerError::Rejected => Self::BadRequest,
            BrokerError::OutcomeUnknown => Self::OrderOutcomeUnknown,
            BrokerError::Unavailable => Self::ServiceUnavailable,
        }
    }
}

impl From<PipelineError> for ApiError {
    /// Convert news-source and AI-provider failures into the safe unavailable envelope.
    fn from(_: PipelineError) -> Self {
        Self::ServiceUnavailable
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        http::{header::CONTENT_TYPE, HeaderValue},
        response::IntoResponse,
    };
    use http_body_util::BodyExt;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn service_unavailable_response_uses_safe_json_contract() {
        let response = ApiError::ServiceUnavailable.into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers().get(CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/json"))
        );
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body,
            json!({
                "error": {
                    "code": "service_unavailable",
                    "message": "service is unavailable"
                }
            })
        );
        assert!(!body.to_string().contains("postgres://"));
    }

    #[test]
    fn envelope_omits_absent_request_id() {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: "service_unavailable",
                message: "service is unavailable",
                request_id: None,
            },
        };

        assert_eq!(
            serde_json::to_value(body).unwrap(),
            json!({
                "error": {
                    "code": "service_unavailable",
                    "message": "service is unavailable"
                }
            })
        );
    }

    #[test]
    fn envelope_serializes_present_request_id() {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: "service_unavailable",
                message: "service is unavailable",
                request_id: Some("request-123".to_owned()),
            },
        };

        assert_eq!(
            serde_json::to_value(body).unwrap()["error"]["request_id"],
            json!("request-123")
        );
    }

    /// Verify a safe broker rejection keeps the public bad-request contract.
    #[test]
    fn broker_rejection_maps_to_bad_request() {
        assert!(matches!(
            ApiError::from(BrokerError::Rejected),
            ApiError::BadRequest
        ));
    }

    /// Verify an uncertain broker result never becomes a retryable unavailable error.
    #[tokio::test]
    async fn broker_outcome_unknown_uses_non_retryable_conflict_contract() {
        let response = ApiError::from(BrokerError::OutcomeUnknown).into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body,
            json!({
                "error": {
                    "code": "order_outcome_unknown",
                    "message": "order outcome is unknown; do not retry"
                }
            })
        );
    }
}
