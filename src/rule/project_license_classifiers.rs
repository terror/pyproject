use super::*;

define_rule! {
  ProjectLicenseClassifiersRule {
    id: "project-license-classifiers",
    message: "`project.classifiers` conflicts with `project.license`",
    run(context) {
      let Some(classifiers) = context.get("project.classifiers") else {
        return Vec::new();
      };

      let license = context.get("project.license");

      Self::check_license_classifiers(
        context.content(),
        license.as_ref(),
        classifiers,
      )
    }
  }
}

impl ProjectLicenseClassifiersRule {
  fn check_license_classifiers(
    content: &Rope,
    license: Option<&Node>,
    classifiers: Node,
  ) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

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
      }
    }

    if license_is_string && has_license_classifier {
      diagnostics.push(Diagnostic::error(
        "`project.classifiers` must not include license classifiers when `project.license` is set",
        classifiers.span(content),
      ));
    }

    diagnostics
  }
}
