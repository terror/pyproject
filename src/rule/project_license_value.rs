use super::*;

pub(crate) struct ProjectLicenseValueRule;

impl Rule for ProjectLicenseValueRule {
  fn header(&self) -> &'static str {
    "project.license value is invalid"
  }

  fn id(&self) -> &'static str {
    "project-license"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(license) = context.get("project.license") else {
      return Vec::new();
    };

    let license_files_present = context.get("project.license-files").is_some();

    Self::check_license(context.document(), &license, license_files_present)
  }
}

impl ProjectLicenseValueRule {
  const SUPPORTED_KEYS: [&'static str; 2] = ["file", "text"];

  fn check_license(
    document: &Document,
    license: &Node,
    license_files_present: bool,
  ) -> Vec<Diagnostic> {
    match license {
      Node::Str(string) => {
        let mut diagnostics = Vec::new();

        diagnostics.push(Diagnostic::warning(
          "`project.license` should be a table with `file` or `text` per PEP 621; SPDX strings are accepted but non-standard",
          license.span(&document.content),
        ));

        diagnostics.extend(Self::check_license_string(
          document,
          license,
          string.value(),
        ));

        diagnostics
      }
      Node::Table(_) if license_files_present => vec![Diagnostic::error(
        "`project.license` must be a string SPDX expression when `project.license-files` is present",
        license.span(&document.content),
      )],
      Node::Table(_) => {
        let mut diagnostics = Vec::new();

        diagnostics.extend(Self::check_table_keys(document, license));
        diagnostics.extend(Self::check_table(document, license));

        diagnostics
      }
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
      Ok(expression) => {
        if let Ok(Some(canonical)) = spdx::Expression::canonicalize(value) {
          diagnostics.push(Diagnostic::error(
            format!(
              "`project.license` must use a case-normalized SPDX expression (use `{canonical}`)"
            ),
            license.span(&document.content),
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

        diagnostics.push(Diagnostic::error(
          format!(
            "`project.license` must be a valid SPDX expression: {reason}{suggestion}"
          ),
          license.span(&document.content),
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

  fn check_table_keys(document: &Document, license: &Node) -> Vec<Diagnostic> {
    let Some(table) = license.as_table() else {
      return Vec::new();
    };

    table
      .entries()
      .read()
      .iter()
      .filter_map(|(key, _)| {
        (!Self::SUPPORTED_KEYS.contains(&key.value())).then(|| {
          Diagnostic::error(
            "`project.license` only supports `file` or `text` keys",
            key.span(&document.content),
          )
        })
      })
      .collect()
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
}
