use super::*;

pub(crate) struct ProjectLicenseValueRule;

impl Rule for ProjectLicenseValueRule {
  fn display_name(&self) -> &'static str {
    "Project License"
  }

  fn id(&self) -> &'static str {
    "project-license"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<lsp::Diagnostic> {
    if !context.tree().errors.is_empty() {
      return Vec::new();
    }

    let Some(license) = context.get("project.license") else {
      return Vec::new();
    };

    let license_files_present = context.get("project.license-files").is_some();

    Self::check_license(context.document(), &license, license_files_present)
  }
}

impl ProjectLicenseValueRule {
  fn check_license(
    document: &Document,
    license: &Node,
    license_files_present: bool,
  ) -> Vec<lsp::Diagnostic> {
    match license {
      Node::Str(string) => {
        Self::check_license_string(document, license, string.value())
      }
      Node::Table(_) if license_files_present => vec![
        lsp::Diagnostic {
          message: "`project.license` must be a string SPDX expression when `project.license-files` is present".to_string(),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        },
      ],
      Node::Table(_) => {
        let mut diagnostics = Vec::new();

        diagnostics.push(lsp::Diagnostic {
          message:
            "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`"
              .to_string(),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        });

        diagnostics.extend(Self::check_table(document, license));

        diagnostics
      }
      _ => vec![lsp::Diagnostic {
        message: "`project.license` must be a string or table".to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }],
    }
  }

  fn check_license_string(
    document: &Document,
    license: &Node,
    value: &str,
  ) -> Vec<lsp::Diagnostic> {
    if value.trim().is_empty() {
      return vec![lsp::Diagnostic {
        message: "`project.license` must not be empty".to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }];
    }

    let mut diagnostics = Vec::new();

    match spdx::Expression::parse(value) {
      Ok(expression) => {
        if let Ok(Some(canonical)) = spdx::Expression::canonicalize(value) {
          diagnostics.push(lsp::Diagnostic {
            message: format!(
              "`project.license` must use a case-normalized SPDX expression (use `{canonical}`)"
            ),
            range: license.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          });
        }

        diagnostics.extend(Self::deprecation_warnings(
          document,
          license,
          &expression,
        ));
      }
      Err(error)
        if matches!(error.reason, spdx::error::Reason::DeprecatedLicenseId) =>
      {
        if let Ok(expression) =
          spdx::Expression::parse_mode(value, spdx::ParseMode::LAX)
        {
          diagnostics.extend(Self::deprecation_warnings(
            document,
            license,
            &expression,
          ));
        }
      }
      Err(error) => {
        let reason = error.reason.to_string();

        let suggestion = spdx::Expression::canonicalize(value)
          .ok()
          .flatten()
          .map(|canonical| format!(" (did you mean `{canonical}`?)"))
          .unwrap_or_default();

        diagnostics.push(lsp::Diagnostic {
          message: format!(
            "`project.license` must be a valid SPDX expression: {reason}{suggestion}"
          ),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        });
      }
    }

    diagnostics
  }

  fn check_table(document: &Document, license: &Node) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let file = license.try_get("file").ok();
    let text = license.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(lsp::Diagnostic {
        message: "`project.license` must specify only one of `file` or `text`"
          .to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }),
      (None, None) => diagnostics.push(lsp::Diagnostic {
        message:
          "missing required key `project.license.file` or `project.license.text`"
            .to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }),
      _ => {}
    }

    if let Some(ref file) = file {
      match file {
        Node::Str(string) => {
          diagnostics.extend(
            document
              .validate_relative_path(
                string.value(),
                "project.license.file",
                file,
              )
              .err()
              .into_iter()
              .flatten(),
          );
        }
        _ => diagnostics.push(lsp::Diagnostic {
          message: "`project.license.file` must be a string".to_string(),
          range: file.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }),
      }
    }

    if let Some(text) = text {
      match text {
        Node::Str(_) => {}
        _ => diagnostics.push(lsp::Diagnostic {
          message: "`project.license.text` must be a string".to_string(),
          range: text.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }),
      }
    }

    diagnostics
  }

  fn deprecation_warnings(
    document: &Document,
    license: &Node,
    expression: &spdx::Expression,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let mut seen_licenses = HashSet::new();
    let mut seen_exceptions = HashSet::new();

    for requirement in expression.requirements() {
      if let Some(id) = requirement.req.license.id()
        && id.is_deprecated()
        && seen_licenses.insert(id.name)
      {
        diagnostics.push(lsp::Diagnostic {
          message: format!(
            "license identifier `{}` in `project.license` is deprecated",
            id.name
          ),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        });
      }

      if let Some(addition) = &requirement.req.addition
        && let Some(id) = addition.id()
        && id.is_deprecated()
        && seen_exceptions.insert(id.name)
      {
        diagnostics.push(lsp::Diagnostic {
          message: format!(
            "license exception `{}` in `project.license` is deprecated",
            id.name
          ),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        });
      }
    }

    diagnostics
  }
}
