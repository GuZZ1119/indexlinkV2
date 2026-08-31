//! 开发期 Mock AI Provider。
//!
//! [`MockAiProvider`] 不发起任何网络请求，基于简单关键词匹配返回情绪值。
//! 用于开发调试、CI 测试，以及没有 API Key 时跑通全链路。

use async_trait::async_trait;
use tracing::debug;

use crate::{
    AiClientError, AiProvider, AiProviderCapabilities, AiProviderId, AiProviderProfile,
    AiProviderProfileId, Sentiment,
};

/// 开发期 mock，基于关键词匹配返回情绪值。
///
/// # 匹配规则
///
/// | 关键词 | 情绪值 |
/// |--------|--------|
/// | 大涨 / 暴涨 / 利好 / 突破 / 牛市 / 反弹 | +0.6 |
/// | 大跌 / 暴跌 / 利空 / 崩盘 / 熊市 / 危机 / 衰退 | -0.6 |
/// | 上涨 / 增长 / 盈利超预期 / 降息 / 宽松 | +0.3 |
/// | 下跌 / 下滑 / 亏损 / 加息 / 紧缩 / 通胀超预期 | -0.3 |
/// | 其他 | 0.0（中性） |
///
/// # 示例
///
/// ```rust
/// use ai_client::{AiProvider, MockAiProvider, Sentiment};
///
/// # async fn example() {
/// let mock = MockAiProvider::new();
/// let s = mock.analyze("央行宣布降准释放流动性").await.unwrap();
/// assert!(s.value() > 0.0);
/// # }
/// ```
pub struct MockAiProvider {
    /// 未匹配到关键词时的默认情绪值。
    default_sentiment: f64,
}

impl MockAiProvider {
    /// 创建默认 mock provider（中性默认值 0.0）。
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_sentiment: 0.0,
        }
    }

    /// 自定义默认情绪值的 mock provider。
    #[must_use]
    pub fn with_default(mut self, value: f64) -> Self {
        self.default_sentiment = value;
        self
    }

    fn match_keywords(text: &str) -> Option<f64> {
        let lower = text.to_lowercase();

        // 强信号
        if lower.contains("大涨")
            || lower.contains("暴涨")
            || lower.contains("利好")
            || lower.contains("突破历史")
            || lower.contains("牛市")
            || lower.contains("强势反弹")
        {
            return Some(0.6);
        }
        if lower.contains("大跌")
            || lower.contains("暴跌")
            || lower.contains("利空")
            || lower.contains("崩盘")
            || lower.contains("熊市")
            || lower.contains("金融危机")
            || lower.contains("经济衰退")
        {
            return Some(-0.6);
        }

        // 弱信号
        if lower.contains("上涨")
            || lower.contains("增长")
            || lower.contains("盈利超预期")
            || lower.contains("降息")
            || lower.contains("宽松")
            || lower.contains("反弹")
        {
            return Some(0.3);
        }
        if lower.contains("下跌")
            || lower.contains("下滑")
            || lower.contains("亏损")
            || lower.contains("加息")
            || lower.contains("紧缩")
            || lower.contains("通胀超预期")
        {
            return Some(-0.3);
        }

        None
    }
}

impl Default for MockAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AiProvider for MockAiProvider {
    fn profile(&self) -> AiProviderProfile {
        AiProviderProfile::new(
            AiProviderProfileId::new("mock-default").expect("static profile ID is valid"),
            AiProviderId::new("mock").expect("static provider ID is valid"),
            "Mock AI".to_owned(),
            "keyword-matcher".to_owned(),
            AiProviderCapabilities::market_evidence_only(),
        )
        .expect("static mock profile is valid")
    }

    async fn analyze(&self, prompt: &str) -> Result<Sentiment, AiClientError> {
        let raw = Self::match_keywords(prompt).unwrap_or(self.default_sentiment);
        debug!(raw, prompt, "MockAiProvider returned sentiment");
        Ok(Sentiment::new_clamped(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_positive_strong() {
        let mock = MockAiProvider::new();
        let s = mock.analyze("A股今日大幅上涨，牛市信号明显").await.unwrap();
        assert!(s.value() > 0.5);
    }

    #[tokio::test]
    async fn mock_negative_strong() {
        let mock = MockAiProvider::new();
        let s = mock.analyze("美股暴跌触发熔断，市场恐慌").await.unwrap();
        assert!(s.value() < -0.5);
    }

    #[tokio::test]
    async fn mock_positive_weak() {
        let mock = MockAiProvider::new();
        let s = mock.analyze("指数小幅上涨，成交量温和放大").await.unwrap();
        assert!((0.2..0.5).contains(&s.value()));
    }

    #[tokio::test]
    async fn mock_negative_weak() {
        let mock = MockAiProvider::new();
        let s = mock
            .analyze("受加息预期影响，科技股普遍下跌")
            .await
            .unwrap();
        assert!((-0.5..-0.1).contains(&s.value()));
    }

    #[tokio::test]
    async fn mock_neutral_on_unmatched() {
        let mock = MockAiProvider::new();
        let s = mock
            .analyze("今日市场窄幅震荡，成交量与昨日持平")
            .await
            .unwrap();
        assert_eq!(s.value(), 0.0);
    }

    #[tokio::test]
    async fn mock_custom_default() {
        let mock = MockAiProvider::new().with_default(0.1);
        let s = mock.analyze("无关键词的普通文本").await.unwrap();
        assert_eq!(s.value(), 0.1);
    }

    #[tokio::test]
    async fn mock_implements_provider_trait() {
        // 编译期验证 MockAiProvider 实现了 AiProvider
        fn _assert_provider(_: &dyn AiProvider) {}
        let mock = MockAiProvider::new();
        _assert_provider(&mock);
    }

    #[tokio::test]
    async fn mock_strong_signal_overrides_weak() {
        // 回归测试：强信号关键词不能被子串弱信号覆盖。
        // "大涨" 包含 "上涨"，"强势反弹" 包含 "反弹" ——
        // 若检测顺序颠倒，强信号会被弱信号 shadow 掉。
        let mock = MockAiProvider::new();

        // "大涨" 是强信号（0.6），不能因为包含 "上涨" 而返回弱信号（0.3）
        let s = mock.analyze("A股今日大涨").await.unwrap();
        assert!(s.value() > 0.5, "大涨应是强信号(0.6), got {}", s.value());

        // "强势反弹" 是强信号，不能因为包含 "反弹" 而返回弱信号
        let s = mock.analyze("市场强势反弹").await.unwrap();
        assert!(
            s.value() > 0.5,
            "强势反弹应是强信号(0.6), got {}",
            s.value()
        );

        // 纯弱信号不受影响
        let s = mock.analyze("市场温和反弹").await.unwrap();
        assert!(
            (0.2..0.5).contains(&s.value()),
            "反弹应是弱信号(0.3), got {}",
            s.value()
        );
    }
}
