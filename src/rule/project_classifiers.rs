use super::*;

define_rule! {
  ProjectClassifiersRule {
    id: "project-classifiers",
    message: "invalid `project.classifiers` configuration",
    run(context) {
      let Some(classifiers) = context.get("project.classifiers") else {
        return Vec::new();
      };

      let mut diagnostics = Vec::new();

      let Some(array) = classifiers.as_array() else {
        diagnostics.push(Diagnostic::error(
          "`project.classifiers` must be an array of strings",
          classifiers.span(context.content()),
        ));

        return diagnostics;
      };

      let mut seen = HashSet::new();

      for item in array.items().read().iter() {
        match item.as_str() {
          Some(string) => {
            let value = string.value();

            if !seen.insert(value) {
              diagnostics.push(Diagnostic::error(
                format!(
                  "`project.classifiers` contains duplicate classifier `{value}`"
                ),
                item.span(context.content()),
              ));

              continue;
            }

            if !Self::classifiers().contains(value) {
              diagnostics.push(Diagnostic::error(
                format!(
                  "`project.classifiers` contains an unknown classifier `{value}`"
                ),
                item.span(context.content()),
              ));
            }
          }
          None => diagnostics.push(Diagnostic::error(
            "`project.classifiers` items must be strings",
            item.span(context.content()),
          )),
        }
      }

      diagnostics
    }
  }
}

impl ProjectClassifiersRule {
  fn classifiers() -> &'static HashSet<&'static str> {
    static CLASSIFIERS: OnceLock<HashSet<&'static str>> = OnceLock::new();

    CLASSIFIERS.get_or_init(|| {
      include_str!("classifiers.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
    })
  }
}
