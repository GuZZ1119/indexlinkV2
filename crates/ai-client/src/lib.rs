#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! AI 语义感知层（10% 权重）。
//!
//! 此 crate 封装 LLM 调用，输出有界情绪偏移 [`Sentiment`]。
//! 错误不在此层自行处理——原样返回给上层 decision engine，
//! 由 engine 按 70/20/10 → 90/10/0 降级策略统一决策。
//!
//! # 设计原则
//!
//! - **IO 边界适配器**：本 crate 是唯一进行网络 IO 的 AI 层；
//!   上层（decision engine）仅消费 [`Sentiment`] 值，不感知 HTTP 细节。
//! - **降级在 engine 层**：AI 不可用时由 decision engine 将权重从
//!   70/20/10 切换到 90/10/0（AI 权重归零）。ai-client 本身不做降级，
//!   只负责返回错误或成功结果。
//! - **密钥安全**：API Key 绝不出现在 Debug / Display / 错误消息中。
//!
//! # 示例
//!
//! ```rust,no_run
//! use ai_client::{AiConfig, AiProvider, QwenClient, Sentiment};
//!
//! # async fn example() {
//! let config = AiConfig::default();
//! let client = QwenClient::new(config);
//!
//! // ai-client 返回 Result；上层 engine 决定降级策略
//! match client
//!     .analyze("美联储维持利率不变，点阵图显示年内降息两次")
//!     .await
//! {
//!     Ok(sentiment) => println!("AI sentiment: {sentiment}"),
//!     Err(err) => eprintln!("AI 不可用，engine 将降级到 90/10/0: {err}"),
//! }
//! # }
//! ```

mod client;
mod error;
mod mock;
pub mod news;
mod provider;
mod sentiment;

pub use client::QwenClient;
pub use error::AiClientError;
pub use mock::MockAiProvider;
pub use news::{
    fetch_market_sentiment, fetch_market_sentiment_report, format_sentiment_prompt, AiEvidence,
    MarketSentimentHeadline, MarketSentimentReport, NewsItem, NewsSource, NewsSourceError,
    PipelineError, RssNewsSource, CNBC_TOP_NEWS_RSS,
};
pub use provider::{
    AiConfig, AiProvider, AiProviderCapabilities, AiProviderId, AiProviderProfile,
    AiProviderProfileError, AiProviderProfileId, AiProviderRegistry,
};
pub use sentiment::{Sentiment, SentimentAnalysis, SentimentAnalysisError, SentimentError};
