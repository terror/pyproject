use super::*;

pub(crate) struct ProjectNameRule;

impl Rule for ProjectNameRule {
  fn message(&self) -> &'static str {
    "invalid value for `project.name`"
  }

  fn id(&self) -> &'static str {
    "project-name"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(project) = context.project() else {
      return Vec::new();
    };

    let document = context.document();

    let diagnostic = match context.get("project.name") {
      Some(name) if !name.is_str() => Some(Diagnostic::error(
        "`project.name` must be a string",
        name.span(&document.content),
      )),
      Some(ref name @ Node::Str(ref string)) => {
        let value = string.value();

        if value.is_empty() {
          Some(Diagnostic::error(
            "`project.name` must not be empty",
            name.span(&document.content),
          ))
        } else {
          let normalized = Self::normalize(value);

          if normalized == value {
            None
          } else {
            Some(Diagnostic::error(
              format!(
                "`project.name` must be PEP 503 normalized (use `{normalized}`)"
              ),
              name.span(&document.content),
            ))
          }
        }
      }
      None => Some(Diagnostic::error(
        "missing required key `project.name`",
        project.span(&document.content),
      )),
      _ => None,
    };

    diagnostic
      .map(|diagnostic| vec![diagnostic])
      .unwrap_or_default()
  }
}

impl ProjectNameRule {
  fn normalize(name: &str) -> String {
    static NORMALIZE_RE: OnceLock<Regex> = OnceLock::new();

    NORMALIZE_RE
      .get_or_init(|| Regex::new(r"[-_.]+").unwrap())
      .replace_all(name, "-")
      .to_ascii_lowercase()
  }
}

#[cfg(test)]
mod tests {
  use {super::*, pretty_assertions::assert_eq};

  #[test]
  fn normalize_already_normalized() {
    assert_eq!(ProjectNameRule::normalize("my-package"), "my-package");
  }

  #[test]
  fn normalize_lowercase_no_separators() {
    assert_eq!(ProjectNameRule::normalize("mypackage"), "mypackage");
  }

  #[test]
  fn normalize_uppercase() {
    assert_eq!(ProjectNameRule::normalize("MyPackage"), "mypackage");
  }

  #[test]
  fn normalize_numbers_uppercase() {
    assert_eq!(ProjectNameRule::normalize("MyPackage2"), "mypackage2");
  }

  #[test]
  fn normalize_with_underscores() {
    assert_eq!(ProjectNameRule::normalize("my_package"), "my-package");
  }

  #[test]
  fn normalize_with_dots() {
    assert_eq!(ProjectNameRule::normalize("my.package"), "my-package");
  }

  #[test]
  fn normalize_mixed_separators() {
    assert_eq!(
      ProjectNameRule::normalize("my_package.name"),
      "my-package-name"
    );
  }

  #[test]
  fn normalize_mixed_consecutive_separators() {
    assert_eq!(ProjectNameRule::normalize("my_.-package"), "my-package");
  }

  #[test]
  fn normalize_complex_mixed_separators() {
    assert_eq!(
      ProjectNameRule::normalize("My__Package.Name-Tool"),
      "my-package-name-tool"
    );
  }

  #[test]
  fn normalize_with_numbers() {
    assert_eq!(ProjectNameRule::normalize("my_package_2"), "my-package-2");
  }

  #[test]
  fn normalize_leading_separator() {
    assert_eq!(ProjectNameRule::normalize("_my_package"), "-my-package");
  }

  #[test]
  fn normalize_trailing_separator() {
    assert_eq!(ProjectNameRule::normalize("my_package_"), "my-package-");
  }

  #[test]
  fn normalize_empty_string() {
    assert_eq!(ProjectNameRule::normalize(""), "");
  }

  #[test]
  fn normalize_only_separators() {
    assert_eq!(ProjectNameRule::normalize("_.-"), "-");
  }
}
