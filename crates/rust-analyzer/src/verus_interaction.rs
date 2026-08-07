//! Converts Verus verifier diagnostics into proof-action error data.

use ide::{AssertFailure, PostFailure, PreFailure, VerusError};
use syntax::{TextRange, TextSize};

use crate::flycheck::Diagnostic;

pub(crate) fn diagnostic_to_verus_error(diagnostic: &Diagnostic) -> Option<VerusError> {
    if diagnostic.message.contains("precondition not satisfied") {
        let (primary, secondary) = primary_and_secondary_ranges(diagnostic)?;
        return Some(VerusError::Pre(PreFailure { failing_pre: secondary, callsite: primary }));
    }
    if diagnostic.message.contains("postcondition not satisfied") {
        let (primary, secondary) = primary_and_secondary_ranges(diagnostic)?;
        return Some(VerusError::Post(PostFailure { failing_post: primary, func_name: secondary }));
    }
    if diagnostic.message.contains("assertion failed") {
        let span =
            diagnostic.spans.iter().find(|span| span.is_primary).or(diagnostic.spans.first())?;
        return Some(VerusError::Assert(AssertFailure { range: span_range(span) }));
    }
    None
}

fn primary_and_secondary_ranges(diagnostic: &Diagnostic) -> Option<(TextRange, TextRange)> {
    let primary = diagnostic.spans.iter().find(|span| span.is_primary)?;
    let secondary = diagnostic.spans.iter().find(|span| !span.is_primary)?;
    Some((span_range(primary), span_range(secondary)))
}

fn span_range(span: &crate::flycheck::DiagnosticSpan) -> TextRange {
    TextRange::new(TextSize::from(span.byte_start), TextSize::from(span.byte_end))
}
