// RedNode-OS – PII Detection Pipeline
//
// Scans text for Personally Identifiable Information before ingestion into memory.
// Prevents accidental storage of credit cards, SSNs, phone numbers, etc.
//
// 14 PII types detected via regex patterns.
// Actions per detection: LOG, REDACT, BLOCK (configurable via env var).
//
// Usage:
//   let result = pii::scan("Call me at 555-123-4567");
//   // → PiiScanResult { has_pii: true, findings: [...], redacted: "Call me at [PHONE_REDACTED]" }

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// PII detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiScanResult {
    pub has_pii: bool,
    pub findings: Vec<PiiFinding>,
    pub redacted: String,
    pub original_length: usize,
    pub pii_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiFinding {
    pub pii_type: String,
    pub severity: String, // "high", "medium", "low"
    pub position: usize,
    pub length: usize,
    pub redacted_preview: String,
}

/// PII pattern with type, regex, severity, and redaction label
struct PiiPattern {
    name: &'static str,
    severity: &'static str,
    regex: Lazy<Regex>,
    redact_label: &'static str,
}

// ─── 14 PII Patterns ───

static PATTERNS: &[PiiPattern] = &[
    // HIGH severity — financial / identity
    PiiPattern {
        name: "credit_card",
        severity: "high",
        regex: Lazy::new(|| {
            Regex::new(r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b").unwrap()
        }),
        redact_label: "[CREDIT_CARD_REDACTED]",
    },
    PiiPattern {
        name: "ssn",
        severity: "high",
        regex: Lazy::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap()),
        redact_label: "[SSN_REDACTED]",
    },
    PiiPattern {
        name: "aadhaar",
        severity: "high",
        regex: Lazy::new(|| Regex::new(r"\b\d{4}\s?\d{4}\s?\d{4}\b").unwrap()),
        redact_label: "[AADHAAR_REDACTED]",
    },
    PiiPattern {
        name: "passport",
        severity: "high",
        regex: Lazy::new(|| Regex::new(r"\b[A-Z]{1,2}\d{6,9}\b").unwrap()),
        redact_label: "[PASSPORT_REDACTED]",
    },
    PiiPattern {
        name: "bank_account_iban",
        severity: "high",
        regex: Lazy::new(|| {
            Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{4}\d{7}([A-Z0-9]?){0,16}\b").unwrap()
        }),
        redact_label: "[IBAN_REDACTED]",
    },
    // MEDIUM severity — contact / location
    PiiPattern {
        name: "email",
        severity: "medium",
        regex: Lazy::new(|| {
            Regex::new(r"\b[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}\b").unwrap()
        }),
        redact_label: "[EMAIL_REDACTED]",
    },
    PiiPattern {
        name: "phone_international",
        severity: "medium",
        regex: Lazy::new(|| {
            Regex::new(r"\+\d{1,3}[-.\s]?\(?\d{1,4}\)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}").unwrap()
        }),
        redact_label: "[PHONE_REDACTED]",
    },
    PiiPattern {
        name: "phone_us",
        severity: "medium",
        regex: Lazy::new(|| Regex::new(r"\b\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap()),
        redact_label: "[PHONE_REDACTED]",
    },
    PiiPattern {
        name: "ip_address",
        severity: "medium",
        regex: Lazy::new(|| {
            Regex::new(r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b").unwrap()
        }),
        redact_label: "[IP_REDACTED]",
    },
    PiiPattern {
        name: "date_of_birth",
        severity: "medium",
        regex: Lazy::new(|| {
            Regex::new(r"\b(?:DOB|Date of Birth|Born)[:\s]*\d{1,2}[/.-]\d{1,2}[/.-]\d{2,4}\b")
                .unwrap()
        }),
        redact_label: "[DOB_REDACTED]",
    },
    // LOW severity — potentially PII in context
    PiiPattern {
        name: "api_key",
        severity: "high",
        regex: Lazy::new(|| {
            Regex::new(r"\b(?:sk-|pk-|api[_-]?key[=:\s]+)[a-zA-Z0-9]{20,}\b").unwrap()
        }),
        redact_label: "[API_KEY_REDACTED]",
    },
    PiiPattern {
        name: "aws_key",
        severity: "high",
        regex: Lazy::new(|| Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap()),
        redact_label: "[AWS_KEY_REDACTED]",
    },
    PiiPattern {
        name: "private_key",
        severity: "high",
        regex: Lazy::new(|| Regex::new(r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----").unwrap()),
        redact_label: "[PRIVATE_KEY_REDACTED]",
    },
    PiiPattern {
        name: "jwt_token",
        severity: "medium",
        regex: Lazy::new(|| {
            Regex::new(r"\beyJ[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}\.[a-zA-Z0-9_-]{20,}\b")
                .unwrap()
        }),
        redact_label: "[JWT_REDACTED]",
    },
];

/// Scan text for PII and return findings + redacted version.
pub fn scan(text: &str) -> PiiScanResult {
    let mut findings = Vec::new();
    let mut redacted = text.to_string();

    for pattern in PATTERNS {
        let regex = &*pattern.regex;
        for mat in regex.find_iter(text) {
            findings.push(PiiFinding {
                pii_type: pattern.name.to_string(),
                severity: pattern.severity.to_string(),
                position: mat.start(),
                length: mat.len(),
                redacted_preview: format!(
                    "{}...{}",
                    &text[mat.start()..mat.start().saturating_add(3).min(mat.end())],
                    pattern.redact_label
                ),
            });
        }
        // Apply redaction
        redacted = regex
            .replace_all(&redacted, pattern.redact_label)
            .to_string();
    }

    // Deduplicate overlapping findings
    findings.sort_by_key(|f| f.position);
    findings.dedup_by(|a, b| a.position == b.position && a.pii_type == b.pii_type);

    let pii_count = findings.len();

    PiiScanResult {
        has_pii: !findings.is_empty(),
        findings,
        redacted,
        original_length: text.len(),
        pii_count,
    }
}

/// Get the configured PII action: "log" (default), "redact", or "block"
pub fn get_pii_action() -> String {
    std::env::var("REDNODE_PII_ACTION")
        .unwrap_or_else(|_| "redact".to_string())
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credit_card_detection() {
        let result = scan("My card is 4111111111111111");
        assert!(result.has_pii);
        assert!(result.findings.iter().any(|f| f.pii_type == "credit_card"));
        assert!(result.redacted.contains("[CREDIT_CARD_REDACTED]"));
    }

    #[test]
    fn test_ssn_detection() {
        let result = scan("SSN: 123-45-6789");
        assert!(result.has_pii);
        assert!(result.findings.iter().any(|f| f.pii_type == "ssn"));
    }

    #[test]
    fn test_email_detection() {
        let result = scan("Contact me at user@example.com");
        assert!(result.has_pii);
        assert!(result.findings.iter().any(|f| f.pii_type == "email"));
        assert!(result.redacted.contains("[EMAIL_REDACTED]"));
    }

    #[test]
    fn test_api_key_detection() {
        let result = scan("Use sk-proj-abc123def456ghi789jkl012mno345pqr");
        assert!(result.has_pii);
        assert!(result.findings.iter().any(|f| f.pii_type == "api_key"));
    }

    #[test]
    fn test_no_pii() {
        let result = scan("The weather is nice today.");
        assert!(!result.has_pii);
        assert_eq!(result.pii_count, 0);
        assert_eq!(result.redacted, "The weather is nice today.");
    }

    #[test]
    fn test_multiple_pii() {
        let result = scan("Email user@test.com and call +1-555-123-4567, card 4111111111111111");
        assert!(result.pii_count >= 3);
    }
}
