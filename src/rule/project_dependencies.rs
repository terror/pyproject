use super::*;

pub(crate) struct ProjectDependenciesRule;

impl Rule for ProjectDependenciesRule {
  fn display_name(&self) -> &'static str {
    "Project Dependencies"
  }

  fn id(&self) -> &'static str {
    "project-dependencies"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let document = context.document();

    let tree = context.tree().clone().into_dom();

    let Some(project) = tree.try_get("project").ok() else {
      return Vec::new();
    };

    let Some(dependencies) = project.try_get("dependencies").ok() else {
      return Vec::new();
    };

    let mut diagnostics = Vec::new();

    let Some(array) = dependencies.as_array() else {
      diagnostics.push(lsp::Diagnostic {
        message: "`project.dependencies` must be an array of PEP 508 strings"
          .to_string(),
        range: dependencies.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      });

      return diagnostics;
    };

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        diagnostics.push(lsp::Diagnostic {
          message: "`project.dependencies` items must be strings".to_string(),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });

        continue;
      };

      let value = string.value();

      match Requirement::from_str(value) {
        Ok(requirement) => {
          if let Some(raw_name) = Self::extract_name(value) {
            let normalized = requirement.name.to_string();

            if raw_name != normalized {
              diagnostics.push(lsp::Diagnostic {
                message: format!(
                  "`project.dependencies` package name `{raw_name}` must be normalized (use `{normalized}`)"
                ),
                range: item.range(&document.content),
                severity: Some(lsp::DiagnosticSeverity::ERROR),
                ..Default::default()
              });
            }
          }

          if let Some(reason) =
            Self::deprecated_or_insecure(requirement.name.as_ref())
          {
            diagnostics.push(lsp::Diagnostic {
              message: format!(
                "`project.dependencies` includes deprecated/insecure package `{}`: {}",
                requirement.name,
                reason.to_lowercase()
              ),
              range: item.range(&document.content),
              severity: Some(lsp::DiagnosticSeverity::WARNING),
              ..Default::default()
            });
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
            diagnostics.push(lsp::Diagnostic {
              message: format!(
                "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
                requirement.name
              ),
              range: item.range(&document.content),
              severity: Some(lsp::DiagnosticSeverity::WARNING),
              ..Default::default()
            });
          }
        }
        Err(error) => diagnostics.push(lsp::Diagnostic {
          message: format!(
            "`project.dependencies` item `{value}` is not a valid PEP 508 dependency: {}",
            error.message.to_string().to_lowercase()
          ),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }),
      }
    }

    diagnostics
  }
}

impl ProjectDependenciesRule {
  const DEPRECATED_OR_INSECURE_PACKAGES: &[(&str, &str)] = &[
    (
      "pycrypto",
      "package is unmaintained and insecure; consider `pycryptodome`",
    ),
    ("pil", "package is deprecated; use `pillow` instead"),
  ];

  fn check_version_constraints(
    requirement: &Requirement,
    specifiers: &pep508_rs::pep440_rs::VersionSpecifiers,
    item: &Node,
    document: &Document,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    if specifiers.is_empty() {
      diagnostics.push(lsp::Diagnostic {
        message: format!(
          "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
          requirement.name
        ),
        range: item.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::WARNING),
        ..Default::default()
      });

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
      diagnostics.push(lsp::Diagnostic {
        message: format!(
          "`project.dependencies` entry `{}` does not specify an upper version bound; consider adding an upper constraint to avoid future breaking changes",
          requirement.name
        ),
        range: item.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::WARNING),
        ..Default::default()
      });
    }

    diagnostics
  }

  fn deprecated_or_insecure(name: &str) -> Option<&'static str> {
    Self::DEPRECATED_OR_INSECURE_PACKAGES.iter().find_map(
      |(package, reason)| {
        (PackageName::from_str(name).is_ok_and(|pkg| pkg.as_ref() == *package))
          .then_some(*reason)
      },
    )
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

  #[test]
  fn deprecated_or_insecure_pycrypto() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("pycrypto"),
      Some("package is unmaintained and insecure; consider `pycryptodome`")
    );
  }

  #[test]
  fn deprecated_or_insecure_pil() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("pil"),
      Some("package is deprecated; use `pillow` instead")
    );
  }

  #[test]
  fn deprecated_or_insecure_pil_uppercase() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("PIL"),
      Some("package is deprecated; use `pillow` instead")
    );
  }

  #[test]
  fn deprecated_or_insecure_safe_package() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("requests"),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_pillow() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("pillow"),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_pycryptodome() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("pycryptodome"),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_invalid_package_name() {
    assert_eq!(
      ProjectDependenciesRule::deprecated_or_insecure("!!!invalid!!!"),
      None
    );
  }
}
