use super::*;

pub(crate) struct ProjectLicenseValueRule;

impl Rule for ProjectLicenseValueRule {
  fn header(&self) -> &'static str {
    "invalid project.license"
  }

  fn id(&self) -> &'static str {
    "project-license"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
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
  ) -> Vec<Diagnostic> {
    match license {
      Node::Str(string) => {
        Self::check_license_string(document, license, string.value())
      }
      Node::Table(_) if license_files_present => vec![Diagnostic::new(
        "`project.license` must be a string SPDX expression when `project.license-files` is present",
        license.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )],
      Node::Table(_) => {
        let mut diagnostics = Vec::new();

        diagnostics.push(Diagnostic::new(
          "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`",
          license.range(&document.content),
          lsp::DiagnosticSeverity::WARNING,
        ));

        diagnostics.extend(Self::check_table(document, license));

        diagnostics
      }
      _ => vec![Diagnostic::new(
        "`project.license` must be a string or table",
        license.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )],
    }
  }

  fn check_license_string(
    document: &Document,
    license: &Node,
    value: &str,
  ) -> Vec<Diagnostic> {
    if value.trim().is_empty() {
      return vec![Diagnostic::new(
        "`project.license` must not be empty",
        license.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )];
    }

    let mut diagnostics = Vec::new();

    match spdx::Expression::parse(value) {
      Ok(expression) => {
        if let Ok(Some(canonical)) = spdx::Expression::canonicalize(value) {
          diagnostics.push(Diagnostic::new(
            format!(
              "`project.license` must use a case-normalized SPDX expression (use `{canonical}`)"
            ),
            license.range(&document.content),
            lsp::DiagnosticSeverity::ERROR,
          ));
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

        diagnostics.push(Diagnostic::new(
          format!(
            "`project.license` must be a valid SPDX expression: {reason}{suggestion}"
          ),
          license.range(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        ));
      }
    }

    diagnostics
  }

  fn check_table(document: &Document, license: &Node) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let file = license.try_get("file").ok();
    let text = license.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(Diagnostic::new(
        "`project.license` must specify only one of `file` or `text`",
        license.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )),
      (None, None) => diagnostics.push(Diagnostic::new(
        "missing required key `project.license.file` or `project.license.text`",
        license.range(&document.content),
        lsp::DiagnosticSeverity::ERROR,
      )),
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
        _ => diagnostics.push(Diagnostic::new(
          "`project.license.file` must be a string",
          file.range(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        )),
      }
    }

    if let Some(text) = text {
      match text {
        Node::Str(_) => {}
        _ => diagnostics.push(Diagnostic::new(
          "`project.license.text` must be a string",
          text.range(&document.content),
          lsp::DiagnosticSeverity::ERROR,
        )),
      }
    }

    diagnostics
  }

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
        diagnostics.push(Diagnostic::new(
          format!(
            "license identifier `{}` in `project.license` is deprecated",
            id.name
          ),
          license.range(&document.content),
          lsp::DiagnosticSeverity::WARNING,
        ));
      }

      if let Some(addition) = &requirement.req.addition
        && let Some(id) = addition.id()
        && id.is_deprecated()
        && seen_exceptions.insert(id.name)
      {
        diagnostics.push(Diagnostic::new(
          format!(
            "license exception `{}` in `project.license` is deprecated",
            id.name
          ),
          license.range(&document.content),
          lsp::DiagnosticSeverity::WARNING,
        ));
      }
    }

    diagnostics
  }
}
