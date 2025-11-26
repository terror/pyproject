use super::*;
use std::collections::HashSet;

pub(crate) struct ProjectLicenseValueDeprecationsRule;

impl Rule for ProjectLicenseValueDeprecationsRule {
  fn display(&self) -> &'static str {
    "deprecated `project.license` value"
  }

  fn id(&self) -> &'static str {
    "project-license-deprecations"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(license) = context.get("project.license") else {
      return Vec::new();
    };

    let license_files_present = context.get("project.license-files").is_some();

    Self::warnings(context.document(), &license, license_files_present)
  }
}

impl ProjectLicenseValueDeprecationsRule {
  fn deprecation_warnings(
    document: &Document,
    license: &Node,
    expression: &spdx::Expression,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let mut seen_licenses = HashSet::new();
    let mut seen_exceptions = HashSet::new();

    for requirement in expression.requirements() {
      if let Some(id) = requirement.req.license.id()
        && id.is_deprecated()
        && seen_licenses.insert(id.name)
      {
        diagnostics.push(Diagnostic::warning(
          format!(
            "license identifier `{}` in `project.license` is deprecated",
            id.name
          ),
          license.span(&document.content),
        ));
      }

      if let Some(addition) = &requirement.req.addition
        && let Some(id) = addition.id()
        && id.is_deprecated()
        && seen_exceptions.insert(id.name)
      {
        diagnostics.push(Diagnostic::warning(
          format!(
            "license exception `{}` in `project.license` is deprecated",
            id.name
          ),
          license.span(&document.content),
        ));
      }
    }

    diagnostics
  }

  fn warnings(
    document: &Document,
    license: &Node,
    license_files_present: bool,
  ) -> Vec<Diagnostic> {
    match license {
      Node::Str(string) => {
        let value = string.value();

        if value.trim().is_empty() {
          return Vec::new();
        }

        match spdx::Expression::parse(value) {
          Ok(expression) => {
            Self::deprecation_warnings(document, license, &expression)
          }
          Err(error)
            if matches!(
              error.reason,
              spdx::error::Reason::DeprecatedLicenseId
            ) =>
          {
            if let Ok(expression) =
              spdx::Expression::parse_mode(value, spdx::ParseMode::LAX)
            {
              return Self::deprecation_warnings(
                document,
                license,
                &expression,
              );
            }

            Vec::new()
          }
          Err(_) => Vec::new(),
        }
      }
      Node::Table(_) if license_files_present => Vec::new(),
      Node::Table(_) => vec![Diagnostic::warning(
        "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
        license.span(&document.content),
      )],
      _ => Vec::new(),
    }
  }
}
