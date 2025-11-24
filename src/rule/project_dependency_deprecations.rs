use super::*;

pub(crate) struct ProjectDependencyDeprecationsRule;

impl Rule for ProjectDependencyDeprecationsRule {
  fn header(&self) -> &'static str {
    "project.dependencies deprecated packages"
  }

  fn id(&self) -> &'static str {
    "project-dependency-deprecations"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(dependencies) = context.get("project.dependencies") else {
      return Vec::new();
    };

    let Some(array) = dependencies.as_array() else {
      return Vec::new();
    };

    let document = context.document();

    let mut diagnostics = Vec::new();

    for item in array.items().read().iter() {
      let Some(string) = item.as_str() else {
        continue;
      };

      let Ok(requirement) =
        Requirement::<VerbatimUrl>::from_str(string.value())
      else {
        continue;
      };

      if let Some(reason) =
        Self::deprecated_or_insecure(requirement.name.as_ref())
      {
        diagnostics.push(Diagnostic::new(
          format!(
            "`project.dependencies` includes deprecated/insecure package `{}`: {}",
            requirement.name,
            reason.to_lowercase()
          ),
          item.span(&document.content),
          lsp::DiagnosticSeverity::WARNING,
        ));
      }
    }

    diagnostics
  }
}

impl ProjectDependencyDeprecationsRule {
  const DEPRECATED_OR_INSECURE_PACKAGES: &[(&str, &str)] = &[
    (
      "pycrypto",
      "package is unmaintained and insecure; consider `pycryptodome`",
    ),
    ("pil", "package is deprecated; use `pillow` instead"),
  ];

  fn deprecated_or_insecure(name: &str) -> Option<&'static str> {
    Self::DEPRECATED_OR_INSECURE_PACKAGES.iter().find_map(
      |(package, reason)| {
        (PackageName::from_str(name).is_ok_and(|pkg| pkg.as_ref() == *package))
          .then_some(*reason)
      },
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn deprecated_or_insecure_pycrypto() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("pycrypto"),
      Some("package is unmaintained and insecure; consider `pycryptodome`")
    );
  }

  #[test]
  fn deprecated_or_insecure_pil() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("pil"),
      Some("package is deprecated; use `pillow` instead")
    );
  }

  #[test]
  fn deprecated_or_insecure_pil_uppercase() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("PIL"),
      Some("package is deprecated; use `pillow` instead")
    );
  }

  #[test]
  fn deprecated_or_insecure_safe_package() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("requests"),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_pillow() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("pillow"),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_pycryptodome() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure("pycryptodome"),
      None
    );
  }

  #[test]
  fn deprecated_or_insecure_invalid_package_name() {
    assert_eq!(
      ProjectDependencyDeprecationsRule::deprecated_or_insecure(
        "!!!invalid!!!"
      ),
      None
    );
  }
}
