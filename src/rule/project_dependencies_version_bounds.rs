use super::*;

define_rule! {
  ProjectDependenciesVersionBoundsRule {
    id: "project-dependencies-version-bounds",
    message: "lenient `project.dependencies` constraints",
    default_level: RuleLevel::Off,
    run(context) {
      let Some(dependencies) = context.get("project.dependencies") else {
        return Vec::new();
      };

      let Some(array) = dependencies.as_array() else {
        return Vec::new();
      };

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
              context.content(),
            ));
          }
          None => diagnostics.push(Diagnostic::warning(
            format!(
              "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
              requirement.name
            ),
            item.span(context.content()),
          )),
          _ => {}
        }
      }

      diagnostics
    }
  }
}

impl ProjectDependenciesVersionBoundsRule {
  fn check_version_constraints(
    requirement: &Requirement,
    specifiers: &pep508_rs::pep440_rs::VersionSpecifiers,
    item: &Node,
    content: &Rope,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if specifiers.is_empty() {
      diagnostics.push(Diagnostic::warning(
        format!(
          "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
          requirement.name
        ),
        item.span(content),
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
        item.span(content),
      ));
    }

    diagnostics
  }
}
