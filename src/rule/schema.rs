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
    static VALIDATORS: OnceLock<
      Mutex<HashMap<Vec<(String, String)>, Result<Validator, String>>>,
    > = OnceLock::new();

    let mut schemas = config
      .schemas
      .iter()
      .map(|(tool, url)| (tool.clone(), url.clone()))
      .collect::<Vec<_>>();

    schemas.sort_unstable();

    let validator = VALIDATORS
      .get_or_init(Default::default)
      .lock()
      .unwrap()
      .entry(schemas)
      .or_insert_with(|| {
        let mut tool_properties = SCHEMAS
          .iter()
          .filter_map(|schema| schema.tool.map(|tool| (tool, schema.url)))
          .map(|(tool, url)| (tool.to_string(), json!({ "$ref": url })))
          .collect::<Map<_, _>>();

        for (tool, url) in &config.schemas {
          tool_properties.insert(tool.clone(), json!({ "$ref": url }));
        }

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
          .map_err(|error| error.to_string())
      })
      .clone();

    validator.map_err(Error::msg)
  }
}
