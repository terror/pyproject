use super::*;

pub(crate) struct ProjectLicenseClassifiersRule;

impl Rule for ProjectLicenseClassifiersRule {
  fn header(&self) -> &'static str {
    "invalid project license classifiers"
  }

  fn id(&self) -> &'static str {
    "project-license-classifiers"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(classifiers) = context.get("project.classifiers") else {
      return Vec::new();
    };

    let license = context.get("project.license");

    Self::check_license_classifiers(
      context.document(),
      license.as_ref(),
      classifiers,
    )
  }
}

impl ProjectLicenseClassifiersRule {
  fn check_license_classifiers(
    document: &Document,
    license: Option<&Node>,
    classifiers: Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(array) = classifiers.as_array() else {
      return diagnostics;
    };

    let license_is_string = license.is_some_and(Node::is_str);

    let mut has_license_classifier = false;

    for item in array.items().read().iter() {
      let Some(value) = item.as_str() else {
        continue;
      };

      if value.value().starts_with("License ::") {
        has_license_classifier = true;

        diagnostics.push(lsp::Diagnostic {
          message: if license_is_string {
            "`project.classifiers` license classifiers are deprecated when `project.license` is present (use only `project.license`)".to_string()
          } else {
            "`project.classifiers` license classifiers are deprecated; use `project.license` instead"
              .to_string()
          },
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        });
      }
    }

    if license_is_string && has_license_classifier {
      diagnostics.push(lsp::Diagnostic {
        message:
          "`project.classifiers` must not include license classifiers when `project.license` is set"
            .to_string(),
        range: classifiers.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      });
    }

    diagnostics
  }
}
