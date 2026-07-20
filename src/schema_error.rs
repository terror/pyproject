use super::*;

pub(crate) struct SchemaError<'a>(pub(crate) &'a ValidationError<'a>);

impl Display for SchemaError<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let path = self
      .0
      .instance_path()
      .as_str()
      .trim_start_matches('/')
      .split('/')
      .filter(|segment| !segment.is_empty())
      .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
      .collect::<Vec<_>>()
      .join(".");

    let target = if path.is_empty() {
      "value".to_string()
    } else {
      format!("`{path}`")
    };

    let message = match self.0.kind() {
      ValidationErrorKind::AdditionalItems { limit } => {
        let count = self.0.instance().as_array().map(Vec::len);

        count.map_or_else(
          || format!("{target} allows at most {limit} items"),
          |count| {
            format!("{target} allows at most {limit} items, found {count}")
          },
        )
      }
      ValidationErrorKind::AdditionalProperties { unexpected } => {
        let setting = unexpected.first().map_or_else(
          || target.clone(),
          |property| {
            let path = if path.is_empty() {
              property.clone()
            } else {
              format!("{path}.{property}")
            };

            format!("`{path}`")
          },
        );

        format!("unknown setting {setting}")
      }
      ValidationErrorKind::AnyOf { context } => context
        .iter()
        .flatten()
        .find(|error| matches!(error.kind(), ValidationErrorKind::Enum { .. }))
        .map_or_else(
          || format!("{target} must be a rule level or configuration table"),
          |error| SchemaError(error).to_string(),
        ),
      ValidationErrorKind::Enum { options } => {
        let options = options.as_array().map_or_else(
          || options.to_string(),
          |options| {
            options
              .iter()
              .map(|value| match value {
                Value::Null => "null".to_string(),
                Value::Bool(value) => value.to_string(),
                Value::Number(value) => value.to_string(),
                Value::String(value) => format!("\"{value}\""),
                Value::Array(_) | Value::Object(_) => value.to_string(),
              })
              .collect::<Vec<_>>()
              .join(", ")
          },
        );

        format!("{target} must be one of: {options}")
      }
      ValidationErrorKind::MinLength { limit } => {
        let count = self
          .0
          .instance()
          .as_str()
          .map(|value| value.chars().count());

        count.map_or_else(
          || format!("{target} must be at least {limit} characters long"),
          |count| {
            format!(
              "{target} must be at least {limit} characters long, found {count}"
            )
          },
        )
      }
      ValidationErrorKind::Required { property } => {
        let property = property
          .as_str()
          .map_or_else(|| property.to_string(), str::to_string);

        let path = if path.is_empty() {
          property
        } else {
          format!("{path}.{property}")
        };

        format!("missing required setting `{path}`")
      }
      ValidationErrorKind::Type { kind } => {
        let expected = match kind {
          TypeKind::Single(type_) => type_.to_string(),
          TypeKind::Multiple(types) => {
            let mut types = types
              .iter()
              .map(|type_| type_.to_string())
              .collect::<Vec<_>>();

            types.sort();

            match types.len() {
              0 => String::new(),
              1 => types.pop().unwrap(),
              2 => format!("{} or {}", types[0], types[1]),
              _ => {
                let last = types.pop().unwrap();
                format!("{}, or {last}", types.join(", "))
              }
            }
          }
        };

        let actual = match self.0.instance().as_ref() {
          Value::Null => "null".to_string(),
          Value::Bool(value) => format!("boolean {value}"),
          Value::Number(value) => {
            if value.is_i64() || value.is_u64() {
              format!("integer {value}")
            } else {
              format!("number {value}")
            }
          }
          Value::String(value) => format!("string \"{value}\""),
          Value::Array(value) => format!("array of length {}", value.len()),
          Value::Object(_) => "object".to_string(),
        };

        format!("expected {expected} for {target}, got {actual}")
      }
      ValidationErrorKind::UniqueItems => {
        format!("items in {target} must be unique")
      }
      _ => format!("{target}: {}", self.0.masked()),
    };

    let mut chars = message.chars();

    if let Some(first) = chars.next() {
      write!(f, "{}{}", first.to_lowercase(), chars.as_str())
    } else {
      f.write_str(&message)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn message(schema: Value, instance: Value) -> String {
    let schema = jsonschema::options()
      .with_draft(jsonschema::Draft::Draft7)
      .build(&schema)
      .unwrap();

    let error = schema.iter_errors(&instance).next().unwrap();

    SchemaError(&error).to_string()
  }

  #[test]
  fn formats_additional_properties_error() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "tool": {
            "type": "object",
            "properties": {
              "black": {
                "type": "object",
                "properties": {
                  "line-length": { "type": "integer" }
                },
                "additionalProperties": false
              }
            }
          }
        }
      }),
      json!({ "tool": { "black": { "unknown": true } } }),
    );

    assert_eq!(message, "unknown setting `tool.black.unknown`");
  }

  #[test]
  fn formats_type_mismatch_error() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "tool": {
            "type": "object",
            "properties": {
              "black": {
                "type": "object",
                "properties": {
                  "line-length": { "type": "integer" }
                }
              }
            }
          }
        }
      }),
      json!({ "tool": { "black": { "line-length": "eighty" } } }),
    );

    assert_eq!(
      message,
      "expected integer for `tool.black.line-length`, got string \"eighty\""
    );
  }

  #[test]
  fn formats_enum_error() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "color": {
            "type": "string",
            "enum": ["red", "green", "blue"]
          }
        }
      }),
      json!({ "color": "orange" }),
    );

    assert_eq!(
      message,
      "`color` must be one of: \"red\", \"green\", \"blue\""
    );
  }

  #[test]
  fn formats_additional_items_error() {
    let message = message(
      json!({
        "type": "array",
        "items": [
          { "type": "integer" }
        ],
        "additionalItems": false
      }),
      json!([1, 2]),
    );

    assert_eq!(message, "value allows at most 1 items, found 2");
  }

  #[test]
  fn formats_multiple_type_error() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "choice": {
            "type": ["string", "integer"]
          }
        }
      }),
      json!({ "choice": true }),
    );

    assert_eq!(
      message,
      "expected integer or string for `choice`, got boolean true"
    );
  }

  #[test]
  fn decodes_pointer_segments_in_paths() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "path~to/setting": {
            "type": "integer"
          }
        }
      }),
      json!({ "path~to/setting": "wrong" }),
    );

    assert_eq!(
      message,
      "expected integer for `path~to/setting`, got string \"wrong\""
    );
  }

  #[test]
  fn formats_min_length_error_with_count() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "code": {
            "type": "string",
            "minLength": 5
          }
        }
      }),
      json!({ "code": "abc" }),
    );

    assert_eq!(
      message,
      "`code` must be at least 5 characters long, found 3"
    );
  }

  #[test]
  fn formats_unique_items_error() {
    let message = message(
      json!({
        "type": "object",
        "properties": {
          "ids": {
            "type": "array",
            "uniqueItems": true,
            "items": {
              "type": "integer"
            }
          }
        }
      }),
      json!({ "ids": [1, 1] }),
    );

    assert_eq!(message, "items in `ids` must be unique");
  }
}
