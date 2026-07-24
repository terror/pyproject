use super::*;

define_rule! {
  TopLevelUnknownKeysRule {
    id: "top-level-unknown-keys",
    message: "document contains unknown top-level keys",
    run(context) {
      if !context.tree().errors.is_empty()
        || context.tree().clone().into_dom().validate().is_err()
      {
        return Vec::new();
      }

      let Some(root) = context.get("") else {
        return Vec::new();
      };

      let Some(table) = root.as_table() else {
        return Vec::new();
      };

      table
        .entries()
        .read()
        .iter()
        .filter(|(key, _)| !Self::is_allowed(key.value()))
        .map(|(key, _)| {
          let name = key.value();

          Diagnostic::error(
            format!(
              "`{name}` is not allocated by a PyPA specification; move tool-specific settings under `[tool.NAME]`"
            ),
            key.span(context.content()),
          )
        })
        .collect()
    }
  }
}

impl TopLevelUnknownKeysRule {
  fn is_allowed(key: &str) -> bool {
    matches!(
      key,
      "build-system" | "dependency-groups" | "project" | "tool"
    )
  }
}
