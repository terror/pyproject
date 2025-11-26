use super::*;

pub(crate) struct ProjectDependenciesVersionBoundsRule;

impl Rule for ProjectDependenciesVersionBoundsRule {
  fn display(&self) -> &'static str {
    "lenient `project.dependencies` constraints"
  }

  fn id(&self) -> &'static str {
    "project-dependencies-version-bounds"
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

      let value = string.value();

      let Ok(requirement) = Requirement::<VerbatimUrl>::from_str(value) else {
        continue;
      };

      match &requirement.version_or_url {
        Some(VersionOrUrl::VersionSpecifier(specifiers)) => {
          diagnostics.extend(Self::check_version_constraints(
            &requirement,
            specifiers,
            item,
            document,
          ));
        }
        None => diagnostics.push(Diagnostic::warning(
          format!(
            "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
            requirement.name
          ),
          item.span(&document.content),
        )),
        _ => {}
      }
    }

    diagnostics
  }
}

impl ProjectDependenciesVersionBoundsRule {
  fn check_version_constraints(
    requirement: &Requirement,
    specifiers: &pep508_rs::pep440_rs::VersionSpecifiers,
    item: &Node,
    document: &Document,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if specifiers.is_empty() {
      diagnostics.push(Diagnostic::warning(
        format!(
          "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
          requirement.name
        ),
        item.span(&document.content),
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
      diagnostics.push(Diagnostic::warning(
        format!(
          "`project.dependencies` entry `{}` does not specify an upper version bound; consider adding an upper constraint to avoid future breaking changes",
          requirement.name
        ),
        item.span(&document.content),
      ));
    }

    diagnostics
  }
}
