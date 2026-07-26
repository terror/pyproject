use super::*;

pub(crate) struct SchemaStore;

impl SchemaStore {
  pub(crate) fn documents() -> &'static HashMap<&'static str, Value> {
    static DOCUMENTS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();

    DOCUMENTS.get_or_init(|| {
      let started = Instant::now();

      let documents = SCHEMAS
        .iter()
        .map(|schema| (schema.url, Self::parse_schema(schema)))
        .collect::<HashMap<_, _>>();

      debug!(
        schema_count = documents.len(),
        elapsed = ?started.elapsed(),
        "loaded bundled schemas"
      );

      documents
    })
  }

  fn parse_schema(schema: &Schema) -> Value {
    serde_json::from_str(schema.contents).unwrap_or_else(|error| {
      panic!("failed to parse bundled schema {}: {error}", schema.url)
    })
  }

  pub(crate) fn root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();

    ROOT.get_or_init(|| {
      let tool_properties = SCHEMAS
        .iter()
        .filter_map(|schema| schema.tool.map(|tool| (tool, schema.url)))
        .map(|(tool, url)| (tool.to_string(), json!({ "$ref": url })))
        .collect::<Map<String, Value>>();

      json!({
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
      })
    })
  }

  pub(crate) fn validator() -> Result<&'static Validator> {
    static VALIDATOR: OnceLock<Result<Validator, String>> = OnceLock::new();

    VALIDATOR
      .get_or_init(|| {
        let started = Instant::now();

        let validator = jsonschema::options()
          .with_retriever(Self)
          .build(Self::root())
          .map_err(|error| {
            debug!(%error, "failed to compile schema validator");
            error.to_string()
          });

        debug!(
          success = validator.is_ok(),
          elapsed = ?started.elapsed(),
          "compiled schema validator"
        );

        validator
      })
      .as_ref()
      .map_err(|error| Error::SchemaCompile {
        error: error.clone(),
      })
  }
}

impl Retrieve for SchemaStore {
  fn retrieve(
    &self,
    uri: &Uri<String>,
  ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Self::documents()
      .get(uri.as_str())
      .cloned()
      .ok_or_else(|| format!("schema not found for `{uri}`").into())
  }
}
