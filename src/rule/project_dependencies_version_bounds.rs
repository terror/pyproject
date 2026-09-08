use super::*;

define_rule! {
  /// Warns when `project.dependencies` entries lack version constraints or upper bounds.
  ///
  /// Encourages specifying version ranges with upper bounds to prevent
  /// unexpected breakage from future major releases of dependencies.
  /// Disabled by default.
  ProjectDependenciesVersionBoundsRule {
    id: "project-dependencies-version-bounds",
    message: "lenient `project.dependencies` constraints",
    default_level: RuleLevel::Off,
    run(context) {
      let mut diagnostics = Vec::new();

      for dependency in context.project_dependencies().into_iter().flatten() {
        let requirement = &dependency.requirement;

        match &requirement.version_or_url {
          Some(VersionOrUrl::VersionSpecifier(specifiers)) => {
            diagnostics.extend(Self::check_version_constraints(
              &dependency,
              specifiers,
            ));
          }
          None => diagnostics.push(Diagnostic::warning(
            format!(
              "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
              requirement.name
            ),
            dependency.range,
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
    dependency: &ParsedDependency,
    specifiers: &VersionSpecifiers,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if specifiers.is_empty() {
      diagnostics.push(Diagnostic::warning(
        format!(
          "`project.dependencies` entry `{}` does not pin a version; add a version range with an upper bound to avoid future breaking changes",
          dependency.requirement.name
        ),
        dependency.range,
      ));

      return diagnostics;
    }

    if !specifiers.has_upper_bound() {
      diagnostics.push(Diagnostic::warning(
        format!(
          "`project.dependencies` entry `{}` does not specify an upper version bound; consider adding an upper constraint to avoid future breaking changes",
          dependency.requirement.name
        ),
        dependency.range,
      ));
    }

    diagnostics
  }
}
