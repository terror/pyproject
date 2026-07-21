use super::*;

define_rule! {
  /// Validates the document against JSON schemas for `pyproject.toml`.
  ///
  /// Uses JSON Schema validation to check tool-specific configuration
  /// sections against their published schemas.
  SchemaRule {
    id: "json-schema",
    message: "schema mismatch",
    run(context) {
      let document = context.document();

      let Ok((instance, pointers)) = SchemaPointer::build(document) else {
        return Vec::new();
      };

      let validator = match Self::validator(&document.config) {
        Ok(validator) => validator,
        Err(error) => {
          let end = u32::try_from(document.content.len_bytes()).unwrap_or(u32::MAX);

          return vec![Diagnostic::error(
            format!("failed to load schema: {error}"),
            (0, end).span(&document.content),
          )];
        }
      };

      validator
        .iter_errors(&instance)
        .map(|error| pointers.diagnostic(error))
        .collect()
    }
  }
}

impl SchemaRule {
  pub(crate) fn validator(config: &Config) -> Result<Validator> {
    static VALIDATOR: OnceLock<Result<Validator>> = OnceLock::new();

    let mut tool_properties = SCHEMAS
      .iter()
      .filter_map(|schema| schema.tool.map(|tool| (tool, schema.url)))
      .map(|(tool, url)| (tool.to_string(), json!({ "$ref": url })))
      .collect::<Map<_, _>>();

    let root = |tool_properties: Map<String, Value>| {
      jsonschema::options()
        .with_retriever(SchemaStore)
        .build(&json!({
          "$schema": "http://json-schema.org/draft-07/schema#",
          "type": "object",
          "additionalProperties": true,
          "properties": {
            "tool": {
              "type": "object",
              "additionalProperties": true,
              "properties": tool_properties,
            }
          }
        }))
        .map_err(Error::new)
    };

    if config.schemas.is_empty() {
      return VALIDATOR
        .get_or_init(|| root(tool_properties))
        .as_ref()
        .cloned()
        .map_err(|error| Error::msg(error.to_string()));
    }

    for (tool, url) in &config.schemas {
      tool_properties.insert(tool.clone(), json!({ "$ref": url }));
    }

    root(tool_properties)
  }
}
