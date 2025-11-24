use super::*;

pub(crate) struct ProjectRequiresPythonRule;

impl Rule for ProjectRequiresPythonRule {
  fn header(&self) -> &'static str {
    "project.requires-python validation issues"
  }

  fn id(&self) -> &'static str {
    "project-requires-python"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(project) = context.project() else {
      return Vec::new();
    };

    if Self::listed_in_dynamic(&project) {
      return Vec::new();
    }

    let Some(requires_python) = context.get("project.requires-python") else {
      return Vec::new();
    };

    let document = context.document();

    match requires_python.as_str() {
      Some(string) => {
        let value = string.value();

        if value.trim().is_empty() {
          return vec![Diagnostic::new(
            "`project.requires-python` must not be empty",
            requires_python.span(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          )];
        }

        match VersionSpecifiers::from_str(value) {
          Ok(specifiers) => {
            if Self::needs_upper_bound_warning(&specifiers) {
              vec![Diagnostic::new(
                "`project.requires-python` does not specify an upper bound; consider adding one to avoid unsupported future Python versions",
                requires_python.span(&document.content),
                lsp::DiagnosticSeverity::WARNING,
              )]
            } else {
              Vec::new()
            }
          }
          Err(error) => vec![Diagnostic::new(
            format!(
              "`project.requires-python` must be a valid PEP 440 version specifier: {error}"
            ),
            requires_python.span(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          )],
        }
      }
      None => vec![Diagnostic::new(
        "`project.requires-python` must be a string",
        requires_python.span(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )],
    }
  }
}

impl ProjectRequiresPythonRule {
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

  fn listed_in_dynamic(project: &Node) -> bool {
    let Some(dynamic) = project.try_get("dynamic").ok() else {
      return false;
    };

    let Some(items) = dynamic.as_array().map(|array| array.items().read())
    else {
      return false;
    };

    items.iter().any(|item| {
      item
        .as_str()
        .is_some_and(|string| string.value() == "requires-python")
    })
  }

  fn needs_upper_bound_warning(specifiers: &VersionSpecifiers) -> bool {
    !Self::has_upper_bound(specifiers) && !Self::has_exact(specifiers)
  }
}
