use super::*;

pub(crate) struct ProjectLicenseRule;

impl Rule for ProjectLicenseRule {
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

    let document = context.document();

    let tree = context.tree().clone().into_dom();

    let Some(project) = tree.try_get("project").ok() else {
      return Vec::new();
    };

    let license = project.try_get("license").ok();
    let license_files = project.try_get("license-files").ok();

    let classifiers = project.try_get("classifiers").ok();

    let mut diagnostics = Vec::new();

    if let Some(license_files) = &license_files {
      diagnostics.extend(self.check_license_files(document, license_files));
    }

    if let Some(license) = &license {
      diagnostics.extend(self.check_license(
        document,
        license,
        license_files.is_some(),
      ));
    }

    diagnostics.extend(self.check_license_classifiers(
      document,
      license.as_ref(),
      classifiers,
    ));

    diagnostics
  }
}

impl ProjectLicenseRule {
  fn check_license(
    &self,
    document: &Document,
    license: &Node,
    license_files_present: bool,
  ) -> Vec<lsp::Diagnostic> {
    match license {
      Node::Str(string) => {
        self.check_license_string(document, license, string.value())
      }
      Node::Table(_) if license_files_present => vec![self.diagnostic(
        lsp::Diagnostic {
          message: "`project.license` must be a string SPDX expression when `project.license-files` is present".to_string(),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        },
      )],
      Node::Table(_) => {
        let mut diagnostics = Vec::new();

        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message:
            "`project.license` tables are deprecated; prefer a SPDX expression string and `project.license-files`"
              .to_string(),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        }));

        diagnostics.extend(self.check_table(document, license));

        diagnostics
      }
      _ => vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.license` must be a string or table".to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })],
    }
  }

  fn check_license_classifiers(
    &self,
    document: &Document,
    license: Option<&Node>,
    classifiers: Option<Node>,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(classifiers) = classifiers else {
      return diagnostics;
    };

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

        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: if license_is_string {
            "`project.classifiers` license classifiers are deprecated when `project.license` is present (use only `project.license`)".to_string()
          } else {
            "`project.classifiers` license classifiers are deprecated; use `project.license` instead"
              .to_string()
          },
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        }));
      }
    }

    if license_is_string && has_license_classifier {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "`project.classifiers` must not include license classifiers when `project.license` is set"
            .to_string(),
        range: classifiers.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    diagnostics
  }

  fn check_license_files(
    &self,
    document: &Document,
    license_files: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let Some(array) = license_files.as_array() else {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "`project.license-files` must be an array of strings".to_string(),
        range: license_files.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));

      return diagnostics;
    };

    let items = array.items().read();

    if items.is_empty() {
      return diagnostics;
    }

    let Some(root) = Self::document_root(document) else {
      return diagnostics;
    };

    for item in items.iter() {
      let Some(pattern) = item.as_str() else {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.license-files` items must be strings".to_string(),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));

        continue;
      };

      let pattern_value = pattern.value();

      if pattern_value.trim().is_empty() {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message:
            "`project.license-files` patterns must not be empty".to_string(),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));

        continue;
      }

      if let Err(message) = Self::validate_license_files_pattern(pattern_value)
      {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: format!("invalid `project.license-files` pattern `{pattern_value}`: {message}"),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));

        continue;
      }

      match Self::matched_files(&root, pattern_value) {
        Ok(matches) if matches.is_empty() => diagnostics.push(self.diagnostic(
          lsp::Diagnostic {
            message: format!(
              "`project.license-files` pattern `{pattern_value}` did not match any files"
            ),
            range: item.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          },
        )),
        Ok(matches) => diagnostics.extend(
          matches
            .into_iter()
            .filter_map(|path| Self::ensure_utf8_file(&path).err().map(|message| {
              self.diagnostic(lsp::Diagnostic {
                message,
                range: item.range(&document.content),
                severity: Some(lsp::DiagnosticSeverity::ERROR),
                ..Default::default()
              })
            })),
        ),
        Err(error) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: format!(
            "failed to evaluate `project.license-files` pattern `{pattern_value}`: {error}"
          ),
          range: item.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })),
      }
    }

    diagnostics
  }

  fn check_license_string(
    &self,
    document: &Document,
    license: &Node,
    value: &str,
  ) -> Vec<lsp::Diagnostic> {
    if value.trim().is_empty() {
      return vec![self.diagnostic(lsp::Diagnostic {
        message: "`project.license` must not be empty".to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })];
    }

    let mut diagnostics = Vec::new();

    match spdx::Expression::parse(value) {
      Ok(expression) => {
        if let Ok(Some(canonical)) = spdx::Expression::canonicalize(value) {
          diagnostics.push(self.diagnostic(lsp::Diagnostic {
            message: format!(
              "`project.license` must use a case-normalized SPDX expression (use `{canonical}`)"
            ),
            range: license.range(&document.content),
            severity: Some(lsp::DiagnosticSeverity::ERROR),
            ..Default::default()
          }));
        }

        diagnostics.extend(self.deprecation_warnings(
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
          diagnostics.extend(self.deprecation_warnings(
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

        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: format!(
            "`project.license` must be a valid SPDX expression: {reason}{suggestion}"
          ),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));
      }
    }

    diagnostics
  }

  fn check_table(
    &self,
    document: &Document,
    license: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let file = license.try_get("file").ok();
    let text = license.try_get("text").ok();

    match (file.as_ref(), text.as_ref()) {
      (Some(_), Some(_)) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: "`project.license` must specify only one of `file` or `text`"
          .to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      (None, None) => diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "missing required key `project.license.file` or `project.license.text`"
            .to_string(),
        range: license.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      })),
      _ => {}
    }

    if let Some(ref file) = file {
      match file {
        Node::Str(string) => {
          diagnostics.extend(self.validate_path(
            document,
            string.value(),
            file,
          ));
        }
        _ => diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.license.file` must be a string".to_string(),
          range: file.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })),
      }
    }

    if let Some(text) = text {
      match text {
        Node::Str(_) => {}
        _ => diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: "`project.license.text` must be a string".to_string(),
          range: text.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        })),
      }
    }

    diagnostics
  }

  fn deprecation_warnings(
    &self,
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
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: format!(
            "license identifier `{}` in `project.license` is deprecated",
            id.name
          ),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        }));
      }

      if let Some(addition) = &requirement.req.addition
        && let Some(id) = addition.id()
        && id.is_deprecated()
        && seen_exceptions.insert(id.name)
      {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: format!(
            "license exception `{}` in `project.license` is deprecated",
            id.name
          ),
          range: license.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::WARNING),
          ..Default::default()
        }));
      }
    }

    diagnostics
  }

  fn document_root(document: &Document) -> Option<PathBuf> {
    let Ok(mut path) = document.uri.to_file_path() else {
      return None;
    };

    path.pop();

    Some(path)
  }

  fn ensure_utf8_file(path: &Path) -> Result<(), String> {
    fs::read_to_string(path).map(|_| ()).map_err(|error| {
      format!(
        "license file `{}` must be valid UTF-8 text ({error})",
        path.display()
      )
    })
  }

  fn glob_max_depth(pattern: &str) -> Option<usize> {
    if pattern.contains("**") {
      return None;
    }

    Some(
      pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .count()
        .max(1),
    )
  }

  fn matched_files(root: &Path, pattern: &str) -> Result<Vec<PathBuf>, String> {
    let mut builder =
      globwalk::GlobWalkerBuilder::from_patterns(root, &[pattern])
        .follow_links(false);

    if let Some(max_depth) = Self::glob_max_depth(pattern) {
      // Avoid walking the entire workspace when the pattern does not request
      // recursive matching.
      builder = builder.max_depth(max_depth);
    }

    let walker = builder.build().map_err(|error| error.to_string())?;

    let mut paths = Vec::new();

    for entry in walker {
      paths.push(entry.map_err(|error| error.to_string())?.into_path());
    }

    Ok(paths)
  }

  fn path_is_rooted(path: &Path) -> bool {
    path.has_root()
      || path
        .components()
        .any(|component| matches!(component, Component::Prefix(_)))
  }

  fn resolve_path(document: &Document, path: &str) -> Option<PathBuf> {
    let Ok(mut document_path) = document.uri.to_file_path() else {
      return None;
    };

    let path = Path::new(path);

    if Self::path_is_rooted(path) {
      return Some(path.to_path_buf());
    }

    document_path.pop();

    Some(document_path.join(path))
  }

  fn validate_license_files_pattern(pattern: &str) -> Result<(), String> {
    if pattern.starts_with('/') {
      return Err(
        "patterns must be relative; leading `/` is not allowed".into(),
      );
    }

    if pattern.contains('\\') {
      return Err("path delimiter must be `/`, not `\\`".into());
    }

    if pattern.contains("..") {
      return Err("parent directory segments (`..`) are not allowed".into());
    }

    let mut in_brackets = false;

    for (index, character) in pattern.chars().enumerate() {
      match character {
        '[' if !in_brackets => in_brackets = true,
        ']' if in_brackets => in_brackets = false,
        ']' => {
          return Err(format!(
            "unmatched closing `]` at position {}",
            index + 1
          ));
        }
        _ if in_brackets => {
          if !matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.')
          {
            return Err("`[]` character classes may only contain alphanumerics, `_`, `-`, or `.`"
              .into());
          }
        }
        '*' | '?' | '/' | '.' | '-' | '_' => {}
        c if c.is_ascii_alphanumeric() => {}
        c => {
          return Err(format!("character `{c}` is not allowed"));
        }
      }
    }

    if in_brackets {
      return Err("unclosed `[` in pattern".into());
    }

    Ok(())
  }

  fn validate_path(
    &self,
    document: &Document,
    path: &str,
    node: &Node,
  ) -> Vec<lsp::Diagnostic> {
    let mut diagnostics = Vec::new();

    let path_ref = Path::new(path);

    if path.trim().is_empty() {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "file path for `project.license.file` must not be empty".to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));

      return diagnostics;
    }

    if Self::path_is_rooted(path_ref) {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message:
          "file path for `project.license.file` must be relative".to_string(),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    let Some(resolved_path) = Self::resolve_path(document, path) else {
      return diagnostics;
    };

    if resolved_path.exists() {
      if let Err(error) = Self::ensure_utf8_file(&resolved_path) {
        diagnostics.push(self.diagnostic(lsp::Diagnostic {
          message: error,
          range: node.range(&document.content),
          severity: Some(lsp::DiagnosticSeverity::ERROR),
          ..Default::default()
        }));
      }
    } else {
      diagnostics.push(self.diagnostic(lsp::Diagnostic {
        message: format!(
          "file `{path}` for `project.license.file` does not exist"
        ),
        range: node.range(&document.content),
        severity: Some(lsp::DiagnosticSeverity::ERROR),
        ..Default::default()
      }));
    }

    diagnostics
  }
}
