use super::*;

pub(crate) struct ProjectLicenseClassifiersDeprecatedRule;

impl Rule for ProjectLicenseClassifiersDeprecatedRule {
  fn message(&self) -> &'static str {
    "deprecated license classifiers in `project.classifiers`"
  }

  fn id(&self) -> &'static str {
    "project-license-classifiers-deprecated"
  }

  fn run(&self, context: &RuleContext<'_>) -> Vec<Diagnostic> {
    let Some(classifiers) = context.get("project.classifiers") else {
      return Vec::new();
    };

    let Some(array) = classifiers.as_array() else {
      return Vec::new();
    };

    let license_is_string = context
      .get("project.license")
      .is_some_and(|node| node.is_str());

    let mut diagnostics = Vec::new();

    for item in array.items().read().iter() {
      let Some(value) = item.as_str() else {
        continue;
      };

      if value.value().starts_with("License ::") {
        diagnostics.push(Diagnostic::warning(
          if license_is_string {
            "`project.classifiers` license classifiers are deprecated when `project.license` is present (use only `project.license`)"
          } else {
            "`project.classifiers` license classifiers are deprecated; use `project.license` instead"
          },
          item.span(&context.document().content),
        ));
      }
    }

    diagnostics
  }
}
