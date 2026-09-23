//! Bounded, read-only AI explanations for deterministic product facts.

use serde::{Deserialize, Serialize};

const MAX_FACT_BYTES: usize = 48_000;
const MAX_TEXT_CHARS: usize = 1_200;
const MAX_ITEMS: usize = 5;

/// The product context being explained by an AI provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiExplanationKind {
    /// Explain one already-computed, real-data backtest.
    Backtest,
    /// Summarise the user's locally persisted plans and recent decisions.
    PersonalSummary,
}

impl AiExplanationKind {
    pub(crate) const fn prompt_label(self) -> &'static str {
        match self {
            Self::Backtest => "real backtest result",
            Self::PersonalSummary => "personal plan summary",
        }
    }
}

/// Server-built facts accepted by the read-only explanation boundary.
#[derive(Debug, Clone)]
pub struct AiExplanationRequest {
    kind: AiExplanationKind,
    facts: serde_json::Value,
}

impl AiExplanationRequest {
    /// Build an explanation request from deterministic, credential-free server facts.
    ///
    /// # Errors
    ///
    /// Returns [`AiExplanationError::InvalidFacts`] for non-object or oversized input.
    pub fn new(
        kind: AiExplanationKind,
        facts: serde_json::Value,
    ) -> Result<Self, AiExplanationError> {
        let encoded = serde_json::to_vec(&facts).map_err(|_| AiExplanationError::InvalidFacts)?;
        if !facts.is_object() || encoded.len() > MAX_FACT_BYTES {
            return Err(AiExplanationError::InvalidFacts);
        }
        Ok(Self { kind, facts })
    }

    pub(crate) const fn kind(&self) -> AiExplanationKind {
        self.kind
    }

    pub(crate) fn facts(&self) -> &serde_json::Value {
        &self.facts
    }
}

/// A bounded plain-language explanation that never carries execution authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiReadOnlyExplanation {
    /// Short heading for the generated explanation.
    pub headline: String,
    /// Plain-language interpretation of the supplied facts.
    pub summary: String,
    /// Up to five observations grounded only in the supplied facts.
    pub observations: Vec<String>,
    /// Up to five limitations or risks that help prevent over-interpretation.
    pub risks: Vec<String>,
}

impl AiReadOnlyExplanation {
    /// Validate and construct a provider-neutral read-only explanation.
    ///
    /// # Errors
    ///
    /// Returns [`AiExplanationError::InvalidOutput`] for blank, oversized, or excessive output.
    pub fn new(
        headline: String,
        summary: String,
        observations: Vec<String>,
        risks: Vec<String>,
    ) -> Result<Self, AiExplanationError> {
        let headline = normalize_text(headline)?;
        let summary = normalize_text(summary)?;
        let observations = normalize_items(observations)?;
        let risks = normalize_items(risks)?;
        Ok(Self {
            headline,
            summary,
            observations,
            risks,
        })
    }
}

/// Validation failures at the read-only explanation boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AiExplanationError {
    /// Server facts were not a bounded JSON object.
    #[error("AI explanation facts are invalid")]
    InvalidFacts,
    /// Model output did not satisfy the bounded explanation contract.
    #[error("AI explanation output is invalid")]
    InvalidOutput,
}

fn normalize_text(value: String) -> Result<String, AiExplanationError> {
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > MAX_TEXT_CHARS
        || value.chars().any(|character| character == '\0')
    {
        Err(AiExplanationError::InvalidOutput)
    } else {
        Ok(value.to_owned())
    }
}

fn normalize_items(values: Vec<String>) -> Result<Vec<String>, AiExplanationError> {
    if values.len() > MAX_ITEMS {
        return Err(AiExplanationError::InvalidOutput);
    }
    values.into_iter().map(normalize_text).collect()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AiExplanationResponse {
    pub(crate) headline: String,
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) observations: Vec<String>,
    #[serde(default)]
    pub(crate) risks: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explanation_rejects_unbounded_output() {
        assert_eq!(
            AiReadOnlyExplanation::new(
                "title".to_owned(),
                "summary".to_owned(),
                vec!["item".to_owned(); 6],
                Vec::new(),
            )
            .unwrap_err(),
            AiExplanationError::InvalidOutput
        );
    }

    #[test]
    fn request_accepts_only_bounded_objects() {
        assert!(AiExplanationRequest::new(
            AiExplanationKind::Backtest,
            serde_json::json!({"return": 12.3}),
        )
        .is_ok());
        assert_eq!(
            AiExplanationRequest::new(AiExplanationKind::Backtest, serde_json::json!([1, 2, 3]),)
                .unwrap_err(),
            AiExplanationError::InvalidFacts
        );
    }
}
