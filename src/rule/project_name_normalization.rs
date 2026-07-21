use super::*;

define_rule! {
  ProjectNameNormalizationRule {
    id: "project-name-normalization",
    message: "`project.name` is not normalized",
    default_level: RuleLevel::Off,
    run(context) {
      let Some(name) = context.get("project.name") else {
        return Vec::new();
      };

      let Some(string) = name.as_str() else {
        return Vec::new();
      };

      let value = string.value();

      let Ok(normalized) = PackageName::from_str(value) else {
        return Vec::new();
      };

      if value == normalized.as_ref() {
        Vec::new()
      } else {
        let range = name.span(context.content());

        let replacement_range = lsp::Range {
          start: lsp::Position::new(range.start.line, range.start.character + 1),
          end: lsp::Position::new(range.end.line, range.end.character - 1),
        };

        vec![
          Diagnostic::warning(
            format!("`project.name` is not normalized (use `{normalized}`)"),
            range,
          )
          .quickfix(Quickfix::replacement(
            replacement_range,
            value,
            normalized.to_string(),
          )),
        ]
      }
    }
  }
}
