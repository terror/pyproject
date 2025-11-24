use super::*;

pub(crate) struct ProjectDependenciesRule;

impl Rule for ProjectDependenciesRule {
  fn header(&self) -> &'static str {
    "project.dependencies validation issues"
  }

  fn id(&self) -> &'static str {
    "project-dependencies"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(dependencies) = context.get("project.dependencies") else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    let Some(array) = dependencies.as_array() else {
      diagnostics.push(Diagnostic::new(
        "`project.dependencies` must be an array of PEP 508 strings",
        dependencies.span(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      ));

      return diagnostics;
    };

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(Diagnostic::new(
          "`project.dependencies` items must be strings",
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));

        continue;
      };

      let value = string.value();

      match Requirement::from_str(value) {
        Ok(requirement) => {
          if let Some(raw_name) = Self::extract_name(value) {
            let normalized = requirement.name.to_string();

            if raw_name != normalized {
              diagnostics.push(Diagnostic::new(
                format!(
                  "`project.dependencies` package name `{raw_name}` must be normalized (use `{normalized}`)"
                ),
                item.span(&document.content),
                lsp::DiagnosticSeverity::ERROR,
              ));
            }
          }

          if let Some(version) = &requirement.version_or_url {
            if let VersionOrUrl::VersionSpecifier(specifiers) = version {
              diagnostics.extend(Self::check_version_constraints(
                &requirement,
                specifiers,
                item,
                document,
              ));
            }
          } else {
            diagnostics.push(Diagnostic::new(
              format!(
                "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
                requirement.name
              ),
              item.span(&document.content),
              lsp::DiagnosticSeverity::WARNING,
            ));
          }
        }
        Err(error) => diagnostics.push(Diagnostic::new(
          format!(
            "`project.dependencies` item `{value}` is not a valid PEP 508 dependency: {}",
            error.message.to_string().to_lowercase()
          ),
          item.span(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        )),
      }
    }

    diagnostics
  }
}

impl ProjectDependenciesRule {
  fn check_version_constraints(
    requirement: &Requirement,
    specifiers: &pep508_rs::pep440_rs::VersionSpecifiers,
    item: &Node,
    document: &Document,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if specifiers.is_empty() {
      diagnostics.push(Diagnostic::new(
        format!(
          "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
          requirement.name
        ),
        item.span(&document.content),
        lsp::DiagnosticSeverity::WARNING,
      ));

      return diagnostics;
    }

    let has_exact = specifiers.iter().any(|specifier| {
      matches!(specifier.operator(), Operator::Equal | Operator::ExactEqual)
    });

    let has_upper_bound = specifiers.iter().any(|specifier| {
      matches!(
        specifier.operator(),
        Operator::LessThan
          | Operator::LessThanEqual
          | Operator::EqualStar
          | Operator::NotEqualStar
          | Operator::TildeEqual
      )
    });

    if !has_upper_bound && !has_exact {
      diagnostics.push(Diagnostic::new(
        format!(
          "`project.dependencies` entry `{}` does not specify an upper version bound; consider adding an upper constraint to avoid future breaking changes",
          requirement.name
        ),
        item.span(&document.content),
        lsp::DiagnosticSeverity::WARNING,
      ));
    }

    diagnostics
  }

  fn extract_name(value: &str) -> Option<&str> {
    let trimmed = value.trim_start();

    let end = trimmed
      .find([' ', '\t', '[', '(', '!', '=', '<', '>', '~', ';', '@', ','])
      .unwrap_or(trimmed.len());

    let name = trimmed[..end].trim_end();

    (!name.is_empty()).then_some(name)
  }
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn extract_name_simple_package() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_version_specifier() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests>=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_exact_version() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests==2.28.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_extras() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests[security]>=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_environment_marker() {
    assert_eq!(
      ProjectDependenciesRule::extract_name(
        "requests>=2.0.0; python_version >= '3.8'"
      ),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_url() {
    assert_eq!(
      ProjectDependenciesRule::extract_name(
        "package @ https://example.com/package.tar.gz"
      ),
      Some("package")
    );
  }

  #[test]
  fn extract_name_with_leading_whitespace() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("  requests>=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_trailing_whitespace_before_version() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests >=2.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_comma() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests>=2.0.0,<3.0.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_tilde_equal() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests~=2.28.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_with_not_equal() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests!=2.27.0"),
      Some("requests")
    );
  }

  #[test]
  fn extract_name_empty_string() {
    assert_eq!(ProjectDependenciesRule::extract_name(""), None);
  }

  #[test]
  fn extract_name_only_whitespace() {
    assert_eq!(ProjectDependenciesRule::extract_name("   "), None);
  }

  #[test]
  fn extract_name_with_parentheses() {
    assert_eq!(
      ProjectDependenciesRule::extract_name("requests (>=2.0.0)"),
      Some("requests")
    );
  }
}
