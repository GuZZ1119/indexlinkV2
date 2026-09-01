//! 新闻获取模块。
//!
//! 从 CNBC 等 RSS 源拉取财经新闻，格式化后喂给 [`AiProvider`]。
//! 与 ai-client 的其他部分组合使用时：
//!
//! ```rust,no_run
//! use ai_client::{AiProvider, MockAiProvider, news::{RssNewsSource, NewsSource, fetch_market_sentiment}};
//!
//! # async fn example() {
//! let source = RssNewsSource::default();
//! let ai = MockAiProvider::new();
//! let sentiment = fetch_market_sentiment(&source, &ai).await.unwrap();
//! println!("market sentiment: {sentiment}");
//! # }
//! ```

use std::time::Duration as StdDuration;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use quick_xml::events::Event;
use quick_xml::Reader;
use tracing::{debug, warn};

use crate::{AiClientError, AiProvider, AiProviderProfile, Sentiment, SentimentAnalysis};

/// CNBC US Top News RSS 地址。
pub const CNBC_TOP_NEWS_RSS: &str =
    "https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=100003114";

/// RSS HTTP 请求默认超时（30 秒）。
const DEFAULT_HTTP_TIMEOUT: StdDuration = StdDuration::from_secs(30);

// ─── NewsItem ─────────────────────────────────────────────────────────────────

/// 一条已解析的财经新闻。
#[derive(Debug, Clone, PartialEq)]
pub struct NewsItem {
    /// 新闻标题。
    pub title: String,
    /// 新闻摘要或首段内容。
    pub description: String,
    /// 原始新闻链接；仅在 RSS 提供 HTTP(S) URL 时保留。
    pub url: String,
    /// 发布时间。
    pub pub_date: DateTime<Utc>,
}

// ─── NewsSourceError ──────────────────────────────────────────────────────────

/// 新闻获取可能产生的错误。
#[derive(Debug, thiserror::Error)]
pub enum NewsSourceError {
    /// HTTP 请求失败。
    #[error("failed to fetch news feed")]
    Http(#[source] reqwest::Error),

    /// 服务器返回非成功状态码。
    #[error("news feed server returned HTTP {status}")]
    HttpStatus {
        /// HTTP 状态码。
        status: u16,
    },

    /// RSS XML 解析失败。
    #[error("failed to parse RSS feed")]
    Parse(String),

    /// Feed 中没有新闻条目。
    #[error("news feed returned no items")]
    Empty,
}

// ─── NewsSource trait ─────────────────────────────────────────────────────────

/// 新闻源抽象，与 [`AiProvider`] 同一层级的可替换 trait。
///
/// 当前实现：[`RssNewsSource`]。
#[async_trait]
pub trait NewsSource: Send + Sync {
    /// 拉取并解析新闻，返回按时间降序排列的新闻列表。
    ///
    /// 实现方可在此处完成过滤（如只保留最近 N 小时的新闻）。
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError>;
}

// ─── RssNewsSource ────────────────────────────────────────────────────────────

/// 基于 RSS 的新闻源。
///
/// 从给定的 RSS URL 拉取 XML，解析 `<item>` 中的 `<title>`、`<description>`、
/// `<link>` 与 `<pubDate>`，只保留最近 24 小时内的新闻，最多返回 10 条。
pub struct RssNewsSource {
    url: String,
    http: reqwest::Client,
    max_age: Duration,
    max_items: usize,
}

impl RssNewsSource {
    /// 使用默认 CNBC RSS URL 创建新闻源。
    #[must_use]
    pub fn new() -> Self {
        Self {
            url: CNBC_TOP_NEWS_RSS.to_owned(),
            http: reqwest::Client::builder()
                .timeout(DEFAULT_HTTP_TIMEOUT)
                .build()
                .expect("reqwest::Client::builder with standard options must not fail"),
            max_age: Duration::hours(24),
            max_items: 10,
        }
    }

    /// 指定 RSS URL 与最大年龄（小时）。
    #[must_use]
    pub fn with_config(url: String, max_age_hours: i64, max_items: usize) -> Self {
        Self {
            url,
            http: reqwest::Client::builder()
                .timeout(DEFAULT_HTTP_TIMEOUT)
                .build()
                .expect("reqwest::Client::builder with standard options must not fail"),
            max_age: Duration::hours(max_age_hours),
            max_items,
        }
    }

    async fn fetch_xml(&self) -> Result<String, NewsSourceError> {
        debug!(url = %self.url, "fetching news RSS feed");

        let response = self.http.get(&self.url).send().await.map_err(|err| {
            warn!(?err, "news feed HTTP request failed");
            NewsSourceError::Http(err)
        })?;

        let status = response.status();
        if !status.is_success() {
            warn!(
                status = status.as_u16(),
                "news feed returned non-success status"
            );
            return Err(NewsSourceError::HttpStatus {
                status: status.as_u16(),
            });
        }

        response.text().await.map_err(|err| {
            warn!(?err, "failed to read news feed response body");
            NewsSourceError::Http(err)
        })
    }

    fn parse_items(xml: &str) -> Result<Vec<RawNewsItem>, NewsSourceError> {
        let mut reader = Reader::from_str(xml);

        let mut items = Vec::new();
        let mut in_item = false;
        let mut capture_title = false;
        let mut capture_description = false;
        let mut capture_link = false;
        let mut capture_pubdate = false;
        let mut current = RawNewsItem::default();

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "item" => {
                            in_item = true;
                            current = RawNewsItem::default();
                            capture_title = false;
                            capture_description = false;
                            capture_link = false;
                            capture_pubdate = false;
                        }
                        "title" if in_item => capture_title = true,
                        "description" if in_item => capture_description = true,
                        "link" if in_item => capture_link = true,
                        "pubDate" if in_item => capture_pubdate = true,
                        _ => {}
                    }
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().map_err(|err| {
                        warn!(?err, "RSS XML entity decode error");
                        NewsSourceError::Parse(err.to_string())
                    })?;
                    append_text(
                        &text,
                        capture_title,
                        capture_description,
                        capture_link,
                        capture_pubdate,
                        &mut current,
                    );
                }
                Ok(Event::CData(ref e)) => {
                    let text = String::from_utf8_lossy(e.as_ref());
                    append_text(
                        text.as_ref(),
                        capture_title,
                        capture_description,
                        capture_link,
                        capture_pubdate,
                        &mut current,
                    );
                }
                Ok(Event::End(ref e)) => {
                    let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    match tag.as_str() {
                        "item" if in_item => {
                            in_item = false;
                            // 条目解析完成后统一 trim，避免内联标签
                            // 导致碎片间空格丢失。
                            current.title = current.title.trim().to_string();
                            current.description = current.description.trim().to_string();
                            current.url = normalize_news_url(&current.url);
                            current.pub_date_str = current.pub_date_str.trim().to_string();
                            if !current.title.is_empty() && !current.pub_date_str.is_empty() {
                                items.push(RawNewsItem {
                                    title: std::mem::take(&mut current.title),
                                    description: std::mem::take(&mut current.description),
                                    url: std::mem::take(&mut current.url),
                                    pub_date_str: std::mem::take(&mut current.pub_date_str),
                                });
                            }
                            capture_title = false;
                            capture_description = false;
                            capture_link = false;
                            capture_pubdate = false;
                        }
                        "title" => capture_title = false,
                        "description" => capture_description = false,
                        "link" => capture_link = false,
                        "pubDate" => capture_pubdate = false,
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(err) => {
                    warn!(?err, "RSS XML parse error");
                    return Err(NewsSourceError::Parse(err.to_string()));
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(items)
    }

    fn filter_and_convert(&self, raw: Vec<RawNewsItem>) -> Result<Vec<NewsItem>, NewsSourceError> {
        let now = Utc::now();
        let cutoff = now - self.max_age;

        let mut converted: Vec<NewsItem> = raw
            .into_iter()
            .filter_map(|raw_item| raw_item.with_parsed_date())
            .filter(|item| item.pub_date >= cutoff)
            .collect();

        // 按发布时间降序排列，确保截断时保留最新的 N 条。
        converted.sort_by_key(|item| std::cmp::Reverse(item.pub_date));
        converted.truncate(self.max_items);

        if converted.is_empty() {
            return Err(NewsSourceError::Empty);
        }

        debug!(count = converted.len(), "filtered news items");

        Ok(converted)
    }
}

impl Default for RssNewsSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NewsSource for RssNewsSource {
    async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
        let xml = self.fetch_xml().await?;
        let raw = Self::parse_items(&xml)?;
        self.filter_and_convert(raw)
    }
}

// ─── Internal raw item for parsing ───────────────────────────────────────────

/// XML 解析阶段的中间产物，pub_date 还是字符串。
#[derive(Debug, Clone, Default)]
struct RawNewsItem {
    title: String,
    description: String,
    url: String,
    pub_date_str: String,
}

impl RawNewsItem {
    /// 尝试将 RSS pubDate 字符串解析为 UTC 时间。
    ///
    /// 支持的格式按尝试顺序：
    /// 1. RFC 2822（如 `Mon, 06 Jul 2026 14:30:00 GMT`）
    /// 2. RFC 3339（如 `2026-07-06T14:30:00Z`）
    fn parse_date(raw: &str) -> Option<DateTime<Utc>> {
        // RFC 2822
        if let Ok(dt) = DateTime::parse_from_rfc2822(raw) {
            return Some(dt.with_timezone(&Utc));
        }
        // RFC 3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
            return Some(dt.with_timezone(&Utc));
        }
        debug!(raw, "failed to parse news pubDate");
        None
    }

    fn with_parsed_date(self) -> Option<NewsItem> {
        Self::parse_date(&self.pub_date_str).map(|pub_date| NewsItem {
            title: self.title,
            description: self.description,
            url: self.url,
            pub_date,
        })
    }
}

// ─── Internal helpers ────────────────────────────────────────────────────────

fn append_text(
    text: &str,
    capture_title: bool,
    capture_description: bool,
    capture_link: bool,
    capture_pubdate: bool,
    current: &mut RawNewsItem,
) {
    if capture_title {
        current.title.push_str(text);
    }
    if capture_description {
        current.description.push_str(text);
    }
    if capture_link {
        current.url.push_str(text);
    }
    if capture_pubdate {
        current.pub_date_str.push_str(text);
    }
}

/// Retain only an ordinary HTTP(S) URL for display and audit; RSS links are untrusted input.
fn normalize_news_url(value: &str) -> String {
    let normalized = value.trim();
    if normalized.starts_with("https://") || normalized.starts_with("http://") {
        normalized.to_owned()
    } else {
        String::new()
    }
}

// ─── format_sentiment_prompt ─────────────────────────────────────────────────

/// 将新闻列表格式化为 AI 情绪分析用的 prompt。
///
/// 输出格式：每行一条 `Headline: xxx. Summary: xxx`，
/// 长度控制在合理范围内（约 2000 字符），避免超出 LLM 上下文窗口。
pub fn format_sentiment_prompt(news: &[NewsItem]) -> String {
    let mut prompt =
        String::from("Analyze the following financial news headlines for US market sentiment. ");
    prompt.push_str("Focus on implications for S&P 500 and macroeconomic outlook:\n\n");

    for item in news {
        let desc = truncate_at_sentence(&item.description, 200);
        prompt.push_str(&format!("Headline: {}. Summary: {}\n", item.title, desc));
    }

    let max_len = 3000;
    if prompt.len() > max_len {
        let cutoff = prompt.floor_char_boundary(max_len);
        prompt.truncate(cutoff);
    }

    prompt
}

/// 在给定字符数上限处截断，并尽量落在句末。
fn truncate_at_sentence(text: &str, max_chars: usize) -> &str {
    if text.chars().count() <= max_chars {
        return text;
    }

    // 使用 char_indices 定位第 max_chars 个字符的字节边界，
    // 避免 floor_char_boundary 按字节截断对多字节字符（如中文）的语义不一致。
    let end = match text.char_indices().nth(max_chars) {
        Some((idx, _)) => idx,
        None => return text,
    };
    let truncated = &text[..end];

    // 回退到最后一个句号/问号/感叹号
    if let Some(pos) = truncated.rfind(['.', '?', '!']) {
        // 只要不是在最开头就截断在句末
        if pos > max_chars / 3 {
            return &text[..=pos];
        }
    }

    truncated
}

// ─── fetch_market_sentiment ──────────────────────────────────────────────────

/// 管线错误：fetch → format → analyze 任一环节失败。
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    /// 新闻获取失败。
    #[error(transparent)]
    Source(#[from] NewsSourceError),

    /// AI 分析失败。
    #[error(transparent)]
    Ai(#[from] AiClientError),
}

/// 供 API 展示与审计的新闻来源条目。
#[derive(Debug, Clone, PartialEq)]
pub struct MarketSentimentHeadline {
    /// 新闻标题。
    pub title: String,
    /// RSS 提供的 HTTP(S) 链接；源未提供有效链接时为空字符串。
    pub url: String,
    /// 原始新闻发布时间，统一为 UTC。
    pub published_at: DateTime<Utc>,
}

/// 一次通用 AI 证据管线的完整可审计结果。
///
/// 该类型不属于某个投资策略，也不授予模型任何策略或订单权限。
/// `CoreOpportunityV1` 可选择把其中的有界情绪作为其历史 10% 兼容输入；
/// Fixed DCA 和 DSL Runtime 则可以只展示或存档同一份证据。
#[derive(Debug, Clone, PartialEq)]
pub struct AiEvidence {
    /// Credential-free profile that produced this evidence.
    pub provider: AiProviderProfile,
    /// AI 返回的有界分数、依据与风险提示。
    pub analysis: SentimentAnalysis,
    /// 实际提供给模型的 RSS 新闻来源。
    pub headlines: Vec<MarketSentimentHeadline>,
}

/// Backward-compatible name for a market-news-specific [`AiEvidence`] value.
pub type MarketSentimentReport = AiEvidence;

/// 一站式获取市场情绪的便捷函数。
///
/// 拉取新闻 → 格式化 prompt → 调用 AI 分析 → 返回 sentiment。
///
/// # 错误
///
/// 新闻获取、AI 超时/解析等任一环节失败均返回 [`PipelineError`]。
pub async fn fetch_market_sentiment(
    source: &(impl NewsSource + ?Sized),
    provider: &(impl AiProvider + ?Sized),
) -> Result<Sentiment, PipelineError> {
    Ok(fetch_market_sentiment_report(source, provider)
        .await?
        .analysis
        .sentiment())
}

/// 拉取新闻并返回包含 AI 解释和来源的市场情绪报告。
///
/// 返回的 headlines 源自实际送入模型的 RSS 条目，而不是模型自行生成的来源。
/// 这使 API 与决策存证能审计“模型看到了什么”，同时不会保存新闻正文。
pub async fn fetch_market_sentiment_report(
    source: &(impl NewsSource + ?Sized),
    provider: &(impl AiProvider + ?Sized),
) -> Result<AiEvidence, PipelineError> {
    let news = source.fetch().await?;
    debug!(count = news.len(), "fetched news for sentiment analysis");
    let prompt = format_sentiment_prompt(&news);
    let analysis = provider.analyze_with_evidence(&prompt).await?;
    let headlines = news
        .into_iter()
        .map(|item| MarketSentimentHeadline {
            title: item.title,
            url: item.url,
            published_at: item.pub_date,
        })
        .collect();
    Ok(AiEvidence {
        provider: provider.profile(),
        analysis,
        headlines,
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小可解析的 RSS XML 片段（仿 CNBC 格式）。
    fn sample_rss(entries: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>CNBC Top News</title>
    {entries}
  </channel>
</rss>"#
        )
    }

    fn sample_item(title: &str, desc: &str, date: &str) -> String {
        format!(
            r#"<item>
      <title>{}</title>
      <description>{}</description>
      <pubDate>{}</pubDate>
      <link>https://example.com/1</link>
    </item>"#,
            title, desc, date
        )
    }

    // ── parse_items ───────────────────────────────────────────────────────

    #[test]
    fn parse_single_item() {
        let xml = sample_rss(&sample_item(
            "Fed holds rates steady",
            "The Federal Reserve kept interest rates unchanged.",
            "Mon, 06 Jul 2026 14:30:00 GMT",
        ));

        let items = RssNewsSource::parse_items(&xml).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Fed holds rates steady");
        assert_eq!(
            items[0].description,
            "The Federal Reserve kept interest rates unchanged."
        );
        assert_eq!(items[0].url, "https://example.com/1");
        assert_eq!(items[0].pub_date_str, "Mon, 06 Jul 2026 14:30:00 GMT");
    }

    #[test]
    fn parse_cdata_description_and_default_source_are_supported() {
        let xml = sample_rss(
            "<item><title>CDATA headline</title><description><![CDATA[<b>CDATA summary</b>]]></description><link>https://example.com/cdata</link><pubDate>Mon, 06 Jul 2026 14:30:00 GMT</pubDate></item>",
        );
        let items = RssNewsSource::parse_items(&xml).unwrap();
        assert_eq!(items[0].description, "<b>CDATA summary</b>");
        assert_eq!(RssNewsSource::default().url, CNBC_TOP_NEWS_RSS);
    }

    #[test]
    fn parse_multiple_items() {
        let items_xml = [
            sample_item("Title 1", "Desc 1", "Mon, 06 Jul 2026 10:00:00 GMT"),
            sample_item("Title 2", "Desc 2", "Mon, 06 Jul 2026 12:00:00 GMT"),
        ]
        .join("\n");

        let items = RssNewsSource::parse_items(&sample_rss(&items_xml)).unwrap();

        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_discards_non_http_news_link() {
        let xml = sample_rss(
            r#"<item>
      <title>Safe link test</title>
      <description>One item.</description>
      <link>javascript:alert(1)</link>
      <pubDate>Mon, 06 Jul 2026 14:30:00 GMT</pubDate>
    </item>"#,
        );

        let items = RssNewsSource::parse_items(&xml).unwrap();

        assert_eq!(items[0].url, "");
    }

    #[test]
    fn parse_skips_items_without_title_or_date() {
        // 缺少 pubDate 的 item 不应出现在结果中
        let xml = sample_rss(
            r#"<item>
      <title>No date item</title>
      <description>This item has no date</description>
      <link>https://example.com/1</link>
    </item>
    <item>
      <title>Valid item</title>
      <description>This one has a date</description>
      <pubDate>Mon, 06 Jul 2026 14:30:00 GMT</pubDate>
      <link>https://example.com/2</link>
    </item>"#,
        );

        let items = RssNewsSource::parse_items(&xml).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Valid item");
    }

    #[test]
    fn parse_empty_feed_returns_empty_vec() {
        let xml = sample_rss("");

        let items = RssNewsSource::parse_items(&xml).unwrap();

        assert!(items.is_empty());
    }

    #[test]
    fn parse_rejects_malformed_xml() {
        // 非法的 XML 字符引用
        let result = RssNewsSource::parse_items("<item><title>bad &invalid; entity</title></item>");
        assert!(result.is_err());
    }

    #[test]
    fn parse_handles_html_in_description() {
        // CNBC 的 description 经常包含 HTML 标签
        let xml = sample_rss(&sample_item(
            "Markets rally",
            "Stocks surged as <b>investors</b> cheered the <a href=\"x\">Fed decision</a>.",
            "Mon, 06 Jul 2026 14:30:00 GMT",
        ));

        let items = RssNewsSource::parse_items(&xml).unwrap();

        assert_eq!(items.len(), 1);
        // HTML 标签不会被解析为 XML 结构，而是作为文本
        assert!(items[0].description.contains("investors"));
    }

    // ── date parsing ──────────────────────────────────────────────────────

    #[test]
    fn parse_rfc2822_date() {
        let dt = RawNewsItem::parse_date("Mon, 06 Jul 2026 14:30:00 GMT");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.date_naive().to_string(), "2026-07-06");
    }

    #[test]
    fn parse_rfc3339_date() {
        let dt = RawNewsItem::parse_date("2026-07-06T14:30:00Z");
        assert!(dt.is_some());
    }

    #[test]
    fn parse_invalid_date_returns_none() {
        assert!(RawNewsItem::parse_date("not a date").is_none());
        assert!(RawNewsItem::parse_date("").is_none());
    }

    // ── filter_and_convert ────────────────────────────────────────────────

    /// 构造一个最近时间戳的 RawNewsItem。
    fn recent_raw(title: &str) -> RawNewsItem {
        let now = Utc::now();
        RawNewsItem {
            title: title.to_owned(),
            description: "Recent news".to_owned(),
            url: "https://example.com/recent".to_owned(),
            pub_date_str: now.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        }
    }

    /// 构造一个旧时间戳的 RawNewsItem。
    fn old_raw(title: &str) -> RawNewsItem {
        RawNewsItem {
            title: title.to_owned(),
            description: "Old news".to_owned(),
            url: "https://example.com/old".to_owned(),
            pub_date_str: "Mon, 01 Jan 2020 00:00:00 GMT".to_owned(),
        }
    }

    #[test]
    fn filter_keeps_recent_items() {
        let source = RssNewsSource::new();
        let raw = vec![recent_raw("Recent")];

        let items = source.filter_and_convert(raw).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Recent");
    }

    #[test]
    fn filter_removes_old_items() {
        let source = RssNewsSource::new();
        let raw = vec![old_raw("Old")];

        let result = source.filter_and_convert(raw);

        assert!(result.is_err());
        match result.unwrap_err() {
            NewsSourceError::Empty => {}
            _ => panic!("expected Empty error"),
        }
    }

    #[test]
    fn filter_mixed_old_and_recent() {
        let source = RssNewsSource::new();
        let raw = vec![recent_raw("Recent"), old_raw("Old")];

        let items = source.filter_and_convert(raw).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Recent");
    }

    #[test]
    fn filter_respects_max_items() {
        let source = RssNewsSource::with_config(
            "https://example.com/rss".to_owned(),
            24, // 24 hours
            3,  // only keep 3
        );
        let raw: Vec<_> = (1..=5).map(|i| recent_raw(&format!("News {i}"))).collect();

        let items = source.filter_and_convert(raw).unwrap();

        assert_eq!(items.len(), 3);
    }

    // ── format_sentiment_prompt ───────────────────────────────────────────

    fn make_news_item(title: &str, desc: &str) -> NewsItem {
        NewsItem {
            title: title.to_owned(),
            description: desc.to_owned(),
            url: "https://example.com/news".to_owned(),
            pub_date: Utc::now(),
        }
    }

    #[test]
    fn prompt_includes_headlines() {
        let news = vec![make_news_item("S&P 500 hits record", "Stocks rallied.")];

        let prompt = format_sentiment_prompt(&news);

        assert!(prompt.contains("S&P 500 hits record"));
        assert!(prompt.contains("Stocks rallied"));
        assert!(prompt.contains("Headline:"));
        assert!(prompt.contains("Summary:"));
    }

    #[test]
    fn prompt_includes_all_items() {
        let news: Vec<_> = (0..3)
            .map(|i| make_news_item(&format!("News {i}"), "Desc"))
            .collect();

        let prompt = format_sentiment_prompt(&news);

        assert!(prompt.contains("News 0"));
        assert!(prompt.contains("News 1"));
        assert!(prompt.contains("News 2"));
    }

    #[test]
    fn prompt_truncates_long_content() {
        let long_desc: String = "A".repeat(500);
        // 30 条长新闻会让 prompt 很长
        let news: Vec<_> = (0..30)
            .map(|i| NewsItem {
                title: format!("News {i}"),
                description: long_desc.clone(),
                url: "https://example.com/news".to_owned(),
                pub_date: Utc::now(),
            })
            .collect();

        let prompt = format_sentiment_prompt(&news);

        // 应被截断到 3000 字符左右
        assert!(prompt.len() <= 3100);
    }

    #[test]
    fn prompt_empty_news() {
        let prompt = format_sentiment_prompt(&[]);

        // 即使没有新闻，也应返回有效 prompt
        assert!(!prompt.is_empty());
        assert!(prompt.contains("S&P 500"));
    }

    // ── fetch_market_sentiment ────────────────────────────────────────────

    /// 内存中的 NewsSource mock，返回预设新闻。
    struct StubNewsSource {
        items: Vec<NewsItem>,
    }

    #[async_trait]
    impl NewsSource for StubNewsSource {
        async fn fetch(&self) -> Result<Vec<NewsItem>, NewsSourceError> {
            Ok(self.items.clone())
        }
    }

    #[tokio::test]
    async fn pipeline_fetch_and_analyze() {
        // MockAiProvider 匹配中文关键词："上涨" → +0.3
        let source = StubNewsSource {
            items: vec![make_news_item("A股今日大幅上涨", "投资者情绪乐观.")],
        };
        let ai = crate::MockAiProvider::new();

        let sentiment = fetch_market_sentiment(&source, &ai).await.unwrap();

        assert!(sentiment.value() > 0.0);
    }

    #[tokio::test]
    async fn pipeline_negative_sentiment() {
        // MockAiProvider 匹配中文关键词："暴跌" → -0.6
        let source = StubNewsSource {
            items: vec![make_news_item("美股暴跌触发熔断", "市场恐慌情绪蔓延.")],
        };
        let ai = crate::MockAiProvider::new();

        let sentiment = fetch_market_sentiment(&source, &ai).await.unwrap();

        assert!(sentiment.value() < 0.0);
    }

    #[tokio::test]
    async fn report_keeps_actual_headlines_with_generic_provider_evidence() {
        let source = StubNewsSource {
            items: vec![make_news_item("Markets steady", "No major surprise.")],
        };
        let ai = crate::MockAiProvider::new();

        let report = fetch_market_sentiment_report(&source, &ai).await.unwrap();

        assert_eq!(report.headlines.len(), 1);
        assert_eq!(report.headlines[0].title, "Markets steady");
        assert_eq!(report.headlines[0].url, "https://example.com/news");
        assert!(!report.analysis.rationale().is_empty());
    }

    // ── truncate_at_sentence ──────────────────────────────────────────────

    #[test]
    fn truncate_short_text_unchanged() {
        let text = "Short text.";
        assert_eq!(truncate_at_sentence(text, 100), text);
    }

    #[test]
    fn truncate_at_period() {
        let text = "First sentence. Second sentence here.";
        let result = truncate_at_sentence(text, 30);
        assert!(result.ends_with("First sentence."));
    }

    #[test]
    fn truncate_without_period_falls_back_to_cutoff() {
        let text = "This is a very long text without any punctuation marks in the middle";
        let result = truncate_at_sentence(text, 20);
        // 回退到字符边界截断
        assert!(result.len() <= 20);
    }
}
