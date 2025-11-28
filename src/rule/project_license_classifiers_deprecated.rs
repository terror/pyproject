use super::*;

define_rule! {
  ProjectLicenseClassifiersDeprecatedRule {
    id: "project-license-classifiers-deprecated",
    message: "deprecated license classifiers in `project.classifiers`",
    run(context) {
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
}
