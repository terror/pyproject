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
        vec![Diagnostic::warning(
          format!("`project.name` is not normalized (use `{normalized}`)"),
          name.span(context.content()),
        )]
      }
    }
  }
}
