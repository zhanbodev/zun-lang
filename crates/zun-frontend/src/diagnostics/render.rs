use super::diagnostic::{Diagnostic, Severity};

pub fn render_diagnostic(diagnostic: &Diagnostic) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "{severity}[{}]: {}",
        diagnostic.code, diagnostic.message
    ));

    let mut location = String::new();
    if let Some(file) = &diagnostic.file {
        location.push_str(file);
    }
    if let (Some(line), Some(column)) = (diagnostic.line, diagnostic.column) {
        if !location.is_empty() {
            location.push(':');
        }
        location.push_str(&format!("{line}:{column}"));
    }
    if !location.is_empty() {
        lines.push(format!("  --> {location}"));
    }

    for note in &diagnostic.notes {
        lines.push(format!("  = note: {note}"));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Diagnostic;

    #[test]
    fn renders_location_and_note_lines() {
        let diagnostic = Diagnostic::error("ZUN-T001", "boom")
            .with_file("main.zun")
            .with_location(3, 7)
            .with_note("check token");

        let rendered = render_diagnostic(&diagnostic);
        assert!(rendered.contains("error[ZUN-T001]: boom"));
        assert!(rendered.contains("main.zun:3:7"));
        assert!(rendered.contains("check token"));
    }
}
