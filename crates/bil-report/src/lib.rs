use bil_core::{ReceiptVerificationStatus, VerificationReport};

pub fn render_markdown(report: &VerificationReport) -> String {
    let mut output = String::new();
    output.push_str("# BIL Verification Report\n\n");
    output.push_str("| Field | Value |\n");
    output.push_str("| --- | --- |\n");
    output.push_str(&format!("| Bundle path | `{}` |\n", report.bundle_path));
    output.push_str(&format!(
        "| Bundle id | {} |\n",
        report.bundle_id.as_deref().unwrap_or("_unknown_")
    ));
    output.push_str(&format!(
        "| Bundle verified | `{}` |\n",
        report.bundle_verified
    ));
    output.push_str(&format!(
        "| Receipt present | `{}` |\n",
        report.receipt_present
    ));
    output.push_str(&format!(
        "| Receipt status | `{}` |\n",
        receipt_status_label(report.receipt_status)
    ));
    output.push_str(&format!(
        "| Signature valid | `{}` |\n",
        report.signature_valid
    ));
    output.push_str(&format!(
        "| Trust verified | `{}` |\n",
        report.trust_verified
    ));
    output.push_str(&format!(
        "| Overall verified | `{}` |\n\n",
        report.overall_verified
    ));

    output.push_str("## Coverage\n\n");
    output.push_str(&format!("- Payload count: `{}`\n", report.payload_count));
    output.push_str(&format!(
        "- Covered file count: `{}`\n\n",
        report.covered_file_count
    ));

    output.push_str("## Findings\n\n");
    if report.findings.is_empty() {
        output.push_str("- None\n");
    } else {
        for finding in &report.findings {
            match &finding.logical_path {
                Some(path) => output.push_str(&format!(
                    "- `{}`: {} (`{}`)\n",
                    finding.code, finding.message, path
                )),
                None => output.push_str(&format!("- `{}`: {}\n", finding.code, finding.message)),
            }
        }
    }

    output
}

fn receipt_status_label(status: ReceiptVerificationStatus) -> &'static str {
    match status {
        ReceiptVerificationStatus::Missing => "missing",
        ReceiptVerificationStatus::Verified => "verified",
        ReceiptVerificationStatus::Untrusted => "untrusted",
        ReceiptVerificationStatus::Invalid => "invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bil_core::{ManifestEntry, VerificationFinding};

    #[test]
    fn markdown_report_renders_findings() {
        let report = VerificationReport {
            schema_version: "v0".to_string(),
            bundle_path: "proof.bil".to_string(),
            bundle_id: Some("bil:v0:sha256:abc".to_string()),
            bundle_kind: None,
            profile_version: None,
            payload_count: 1,
            verified_entries: Vec::<ManifestEntry>::new(),
            merkle_roots: None,
            receipt_present: true,
            receipt_status: ReceiptVerificationStatus::Invalid,
            receipt_mode: None,
            receipt_path: None,
            signature_algorithm: None,
            key_id: None,
            covered_file_count: 4,
            bundle_verified: true,
            signature_valid: false,
            trust_verified: false,
            overall_verified: false,
            findings: vec![VerificationFinding {
                code: "invalid-signature".to_string(),
                message: "signature mismatch".to_string(),
                logical_path: None,
            }],
        };

        let markdown = render_markdown(&report);
        assert!(markdown.contains("# BIL Verification Report"));
        assert!(markdown.contains("invalid-signature"));
    }
}
