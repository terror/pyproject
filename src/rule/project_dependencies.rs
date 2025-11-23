use super::*;

use {
  pep508_rs::{PackageName, Requirement, VersionOrUrl, pep440_rs::Operator},
  std::str::FromStr,
};

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

    let mut end = trimmed.len();

    for (index, character) in trimmed.char_indices() {
      match character {
        ' ' | '\t' | '[' | '(' | '!' | '=' | '<' | '>' | '~' | ';' | '@'
        | ',' => {
          end = index;

          break;
        }
        _ => {}
      }
    }

    let name = trimmed[..end].trim_end();

    (!name.is_empty()).then_some(name)
  }
}
