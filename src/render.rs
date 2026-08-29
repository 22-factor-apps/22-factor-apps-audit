use crate::model::{AuditReport, EvidenceState};

pub fn text(report: &AuditReport) -> String {
    let mut output = String::new();
    output.push_str("Twenty-Two-Factor evidence audit\n");
    output.push_str(&format!("Target: {}\n", report.target.locator));
    output.push_str(&format!("Edition: {}\n", report.edition));
    if let Some(revision) = &report.target.source_revision {
        output.push_str(&format!("Source revision: {revision}\n"));
    }
    if let Some(dirty) = report.target.dirty {
        output.push_str(&format!("Working tree dirty: {dirty}\n"));
    }
    output.push('\n');
    output.push_str(&report.interpretation);
    output.push_str("\n\n");

    for finding in &report.findings {
        let state = match finding.state {
            EvidenceState::Observed => "observed",
            EvidenceState::Missing => "missing",
            EvidenceState::ManualReview => "manual-review",
        };
        output.push_str(&format!(
            "[{state}] Factor {:02} / {} — {}\n",
            finding.factor.number, finding.factor.slug, finding.title
        ));
        output.push_str(&format!("  Why: {}\n", finding.rationale));
        for evidence in &finding.evidence {
            output.push_str(&format!(
                "  Evidence: {} ({}) — {}\n",
                evidence.location, evidence.kind, evidence.detail
            ));
        }
        output.push_str(&format!("  Next: {}\n\n", finding.next_step));
    }

    output
}
