use super::*;

define_rule! {
  ProjectRequiresPythonUpperBoundRule {
    id: "project-requires-python-bounds",
    message: "`project.requires-python` lacks an upper bound",
    default_level: RuleLevel::Off,
    run(context) {
      let Some(requires_python) = context.get("project.requires-python") else {
        return Vec::new();
      };

      let Some(string) = requires_python.as_str() else {
        return Vec::new();
      };

      let value = string.value();

      if value.trim().is_empty() {
        return Vec::new();
      }

      let Ok(specifiers) = VersionSpecifiers::from_str(value) else {
        return Vec::new();
      };

      if Self::needs_upper_bound_warning(&specifiers) {
        vec![Diagnostic::warning(
          "`project.requires-python` does not specify an upper bound; consider adding one to avoid unsupported future Python versions",
          requires_python.span(context.content()),
        )]
      } else {
        Vec::new()
      }
    }
  }
}

impl ProjectRequiresPythonUpperBoundRule {
  fn has_exact(specifiers: &VersionSpecifiers) -> bool {
    specifiers.iter().any(|specifier| {
      matches!(specifier.operator(), Operator::Equal | Operator::ExactEqual)
    })
  }

  fn has_upper_bound(specifiers: &VersionSpecifiers) -> bool {
    specifiers.iter().any(|specifier| {
      matches!(
        specifier.operator(),
        Operator::LessThan
          | Operator::LessThanEqual
          | Operator::EqualStar
          | Operator::NotEqualStar
          | Operator::TildeEqual
      )
    })
  }

  fn needs_upper_bound_warning(specifiers: &VersionSpecifiers) -> bool {
    !Self::has_upper_bound(specifiers) && !Self::has_exact(specifiers)
  }
}
