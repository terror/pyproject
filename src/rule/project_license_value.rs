use super::*;

define_rule! {
  ProjectLicenseValueRule {
    id: "project-license",
    message: "project.license value is invalid",
    run(context) {
      let Some(license) = context.get("project.license") else {
        return Vec::new();
      };

      let license_files_present =
        context.get("project.license-files").is_some();

      Self::check_license(context.document(), &license, license_files_present)
    }
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
      Node::Table(_) if license_files_present => vec![Diagnostic::error(
        "`project.license` must be a string SPDX expression when `project.license-files` is present",
        license.span(&document.content),
      )],
      Node::Table(_) => Self::check_table(document, license),
      _ => vec![Diagnostic::error(
        "`project.license` must be a string or table",
        license.span(&document.content),
      )],
    }
  }

  fn check_license_string(
    document: &Document,
    license: &Node,
    value: &str,
  ) -> Vec<Diagnostic> {
    if value.trim().is_empty() {
      return vec![Diagnostic::error(
        "`project.license` must not be empty",
        license.span(&document.content),
      )];
    }

    let mut diagnostics = Vec::new();

    match spdx::Expression::parse(value) {
      Ok(_) => {
        if let Ok(Some(canonical)) = spdx::Expression::canonicalize(value) {
          diagnostics.push(Diagnostic::error(
            format!(
              "`project.license` must use a case-normalized SPDX expression (use `{canonical}`)"
            ),
            license.span(&document.content),
          ));
        }
      }
      Err(error)
        if !matches!(
          error.reason,
          spdx::error::Reason::DeprecatedLicenseId
        ) =>
      {
        let reason = error.reason.to_string();

        let suggestion = spdx::Expression::canonicalize(value)
          .ok()
          .flatten()
          .map(|canonical| format!(" (did you mean `{canonical}`?)"))
          .unwrap_or_default();

        diagnostics.push(Diagnostic::error(
          format!(
            "`project.license` must be a valid SPDX expression: {reason}{suggestion}"
          ),
          license.span(&document.content),
        ));
      }
      _ => {}
    }

    diagnostics
  }

  fn check_table(document: &Document, license: &Node) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let file = license.try_get("file").ok();
    let text = license.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(Diagnostic::error(
        "`project.license` must specify only one of `file` or `text`",
        license.span(&document.content),
      )),
      (None, None) => diagnostics.push(Diagnostic::error(
        "missing required key `project.license.file` or `project.license.text`",
        license.span(&document.content),
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
        _ => diagnostics.push(Diagnostic::error(
          "`project.license.file` must be a string",
          file.span(&document.content),
        )),
      }
    }

    if let Some(text) = text {
      match text {
        Node::Str(_) => {}
        _ => diagnostics.push(Diagnostic::error(
          "`project.license.text` must be a string",
          text.span(&document.content),
        )),
      }
    }

    diagnostics
  }
}
