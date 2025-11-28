use super::*;

define_rule! {
  ProjectKeywordsRule {
    id: "project-keywords",
    message: "invalid `project.keywords` configuration",
    run(context) {
      let Some(keywords) = context.get("project.keywords") else {
        return Vec::new();
      };

      let content = context.content();

      let mut diagnostics = Vec::new();

      let Some(array) = keywords.as_array() else {
        diagnostics.push(Diagnostic::error(
          "`project.keywords` must be an array of strings",
          keywords.span(content),
        ));

        return diagnostics;
      };

      let mut seen = HashSet::new();

      for item in array.items().read().iter() {
        let Some(string) = item.as_str() else {
          diagnostics.push(Diagnostic::error(
            "`project.keywords` items must be strings",
            item.span(content),
          ));

          continue;
        };

        let value = string.value();

        if !seen.insert(value) {
          diagnostics.push(Diagnostic::error(
            format!("`project.keywords` contains duplicate keyword `{value}`"),
            item.span(content),
          ));
        }
      }

      diagnostics
    }
  }
}
