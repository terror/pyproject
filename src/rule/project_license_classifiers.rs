use super::*;

pub(crate) struct ProjectLicenseClassifiersRule;

impl Rule for ProjectLicenseClassifiersRule {
  fn header(&self) -> &'static str {
    "license classifiers deprecated or conflicting"
  }

  fn id(&self) -> &'static str {
    "project-license-classifiers"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
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
  ) -> Vec<Diagnostic> {
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

        diagnostics.push(Diagnostic::new(
          if license_is_string {
            "`project.classifiers` license classifiers are deprecated when `project.license` is present (use only `project.license`)"
          } else {
            "`project.classifiers` license classifiers are deprecated; use `project.license` instead"
          },
          item.span(&document.content),
          lsp::DiagnosticSeverity::WARNING,
        ));
      }
    }

    if license_is_string && has_license_classifier {
      diagnostics.push(Diagnostic::new(
        "`project.classifiers` must not include license classifiers when `project.license` is set",
        classifiers.span(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      ));
    }

    diagnostics
  }
}
