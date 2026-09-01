//! 受限策略 Copilot 的 provider 边界 DTO。
//!
//! 本模块刻意不依赖 `strategy-dsl`：AI adapter 只能返回 JSON 文档与可审计说明，
//! HTTP 层必须将 JSON 重新反序列化为 `StrategySpecDocument` 并调用领域校验器。

use serde_json::Value;

const MAX_OBJECTIVE_LEN: usize = 600;
const MAX_EXPLANATION_LEN: usize = 600;
const MAX_WARNING_COUNT: usize = 5;
const MAX_WARNING_LEN: usize = 180;
const MAX_EVIDENCE_COUNT: usize = 8;
const MAX_EVIDENCE_ID_LEN: usize = 64;
const MAX_EVIDENCE_LABEL_LEN: usize = 240;

/// Server-supplied evidence reference that an AI draft may cite.
///
/// The model is never allowed to invent a source URL or arbitrary reference.
/// The caller provides this closed list and later checks every returned ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCopilotEvidenceReference {
    id: String,
    label: String,
}

impl AiCopilotEvidenceReference {
    /// Construct one safe, displayable evidence reference.
    pub fn new(id: String, label: String) -> Result<Self, AiCopilotDraftError> {
        let id = normalize_identifier(id)?;
        let label = normalize_text(label, MAX_EVIDENCE_LABEL_LEN)?;
        Ok(Self { id, label })
    }

    /// Return the stable reference identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Return the trusted display label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Constrained input supplied to an AI provider for one read-only DSL draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiCopilotDraftRequest {
    policy_id: String,
    policy_version: u32,
    objective: String,
    evidence: Vec<AiCopilotEvidenceReference>,
}

impl AiCopilotDraftRequest {
    /// Construct a bounded draft request with a caller-selected policy reference.
    ///
    /// The provider must repeat this exact policy ID and version in its JSON document.
    /// The API layer later validates the returned document with `strategy-dsl`.
    pub fn new(
        policy_id: String,
        policy_version: u32,
        objective: String,
        evidence: Vec<AiCopilotEvidenceReference>,
    ) -> Result<Self, AiCopilotDraftError> {
        if policy_version == 0 {
            return Err(AiCopilotDraftError::InvalidPolicyVersion);
        }
        if evidence.is_empty() || evidence.len() > MAX_EVIDENCE_COUNT {
            return Err(AiCopilotDraftError::InvalidEvidence);
        }
        let policy_id = normalize_identifier(policy_id)?;
        let objective = normalize_text(objective, MAX_OBJECTIVE_LEN)?;
        Ok(Self {
            policy_id,
            policy_version,
            objective,
            evidence,
        })
    }

    /// Return the required policy identifier.
    #[must_use]
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    /// Return the required positive policy version.
    #[must_use]
    pub const fn policy_version(&self) -> u32 {
        self.policy_version
    }

    /// Return the operator's bounded strategy objective.
    #[must_use]
    pub fn objective(&self) -> &str {
        &self.objective
    }

    /// Return the only evidence references that may be cited by the draft.
    #[must_use]
    pub fn evidence(&self) -> &[AiCopilotEvidenceReference] {
        &self.evidence
    }
}

/// Raw provider output that must still pass the `strategy-dsl` boundary.
///
/// `document` is deliberately JSON rather than a runtime strategy. This crate cannot
/// persist, activate, evaluate, or execute it; callers must reconstruct and validate a
/// `StrategySpecDocument` before exposing the draft to users.
#[derive(Debug, Clone, PartialEq)]
pub struct AiCopilotDraft {
    document: Value,
    explanation: String,
    warnings: Vec<String>,
    evidence_reference_ids: Vec<String>,
}

impl AiCopilotDraft {
    /// Construct bounded provider output without granting strategy authority.
    pub fn new(
        document: Value,
        explanation: String,
        warnings: Vec<String>,
        evidence_reference_ids: Vec<String>,
    ) -> Result<Self, AiCopilotDraftError> {
        if !document.is_object() {
            return Err(AiCopilotDraftError::InvalidDocument);
        }
        if warnings.len() > MAX_WARNING_COUNT {
            return Err(AiCopilotDraftError::InvalidWarnings);
        }
        if evidence_reference_ids.is_empty() || evidence_reference_ids.len() > MAX_EVIDENCE_COUNT {
            return Err(AiCopilotDraftError::InvalidEvidence);
        }
        let explanation = normalize_text(explanation, MAX_EXPLANATION_LEN)?;
        let warnings = warnings
            .into_iter()
            .map(|warning| normalize_text(warning, MAX_WARNING_LEN))
            .collect::<Result<Vec<_>, _>>()?;
        let evidence_reference_ids = evidence_reference_ids
            .into_iter()
            .map(normalize_identifier)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            document,
            explanation,
            warnings,
            evidence_reference_ids,
        })
    }

    /// Return the untrusted JSON document for immediate domain reconstruction.
    #[must_use]
    pub fn document(&self) -> &Value {
        &self.document
    }

    /// Return the bounded explanation accompanying this candidate.
    #[must_use]
    pub fn explanation(&self) -> &str {
        &self.explanation
    }

    /// Return bounded risk warnings accompanying this candidate.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Return model-selected IDs from the trusted evidence-reference set.
    #[must_use]
    pub fn evidence_reference_ids(&self) -> &[String] {
        &self.evidence_reference_ids
    }
}

fn normalize_identifier(value: String) -> Result<String, AiCopilotDraftError> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty()
        || value.len() > MAX_EVIDENCE_ID_LEN
        || !value.is_ascii()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
    {
        Err(AiCopilotDraftError::InvalidIdentifier)
    } else {
        Ok(value)
    }
}

fn normalize_text(value: String, max_len: usize) -> Result<String, AiCopilotDraftError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_len || value.chars().any(char::is_control) {
        Err(AiCopilotDraftError::InvalidText)
    } else {
        Ok(value.to_owned())
    }
}

/// Error returned when Copilot request or output escapes its bounded DTO contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AiCopilotDraftError {
    /// A policy or evidence identifier is unsafe or outside its bounded format.
    #[error("AI Copilot identifier is invalid")]
    InvalidIdentifier,
    /// A requested policy version is zero.
    #[error("AI Copilot policy version is invalid")]
    InvalidPolicyVersion,
    /// Objective, explanation, warning, or evidence label is blank or unsafe.
    #[error("AI Copilot text is invalid")]
    InvalidText,
    /// The raw document is not a JSON object.
    #[error("AI Copilot strategy document is invalid")]
    InvalidDocument,
    /// Warnings exceed the bounded response contract.
    #[error("AI Copilot warnings are invalid")]
    InvalidWarnings,
    /// Evidence references are empty, too numerous, or malformed.
    #[error("AI Copilot evidence references are invalid")]
    InvalidEvidence,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_rejects_unbounded_model_output() {
        assert_eq!(
            AiCopilotDraft::new(
                Value::Null,
                "A reason".to_owned(),
                Vec::new(),
                vec!["dsl_allowlist_v1".to_owned()],
            ),
            Err(AiCopilotDraftError::InvalidDocument)
        );
    }

    #[test]
    fn request_keeps_a_closed_evidence_reference_set() {
        let request = AiCopilotDraftRequest::new(
            "dsl_copilot_guard".to_owned(),
            1,
            "Prefer a conservative RSI opportunity rule".to_owned(),
            vec![AiCopilotEvidenceReference::new(
                "operator_objective".to_owned(),
                "The operator supplied this objective.".to_owned(),
            )
            .unwrap()],
        )
        .unwrap();

        assert_eq!(request.policy_id(), "dsl_copilot_guard");
        assert_eq!(request.evidence()[0].id(), "operator_objective");
    }
}
