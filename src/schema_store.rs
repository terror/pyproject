use super::*;

pub(crate) struct SchemaStore;

impl SchemaStore {
  pub(crate) fn documents() -> &'static HashMap<&'static str, Value> {
    static DOCUMENTS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();

    DOCUMENTS.get_or_init(|| {
      SCHEMAS
        .iter()
        .map(|schema| (schema.url, Self::parse_schema(schema)))
        .collect()
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
      let rule_level = json!({ "enum": RuleLevel::VALUES });

      let rule_config = json!({
        "additionalProperties": false,
        "properties": {
          "level": rule_level,
        },
        "anyOf": [
          {
            "type": "string",
            "enum": RuleLevel::VALUES,
          },
          {
            "type": "object",
          },
        ],
      });

      let rule_properties = inventory::iter::<&dyn Rule>
        .into_iter()
        .map(|rule| (rule.id().to_string(), rule_config.clone()))
        .collect::<Map<String, Value>>();

      let mut tool_properties = SCHEMAS
        .iter()
        .filter_map(|schema| schema.tool.map(|tool| (tool, schema.url)))
        .map(|(tool, url)| (tool.to_string(), json!({ "$ref": url })))
        .collect::<Map<String, Value>>();

      tool_properties.insert(
        "pyproject".to_string(),
        json!({
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "rules": {
              "type": "object",
              "additionalProperties": false,
              "properties": rule_properties,
            },
          },
        }),
      );

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
