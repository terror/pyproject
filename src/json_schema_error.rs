use super::*;

pub(crate) struct JsonSchemaValidationError<'a>(
  pub(crate) &'a ValidationError<'a>,
);

impl Display for JsonSchemaValidationError<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_str(&Self::format_validation_error(self.0))
  }
}

impl JsonSchemaValidationError<'_> {
  fn array_length(value: &Value) -> Option<usize> {
    value.as_array().map(Vec::len)
  }

  fn decode_segment(segment: &str) -> String {
    let mut decoded = String::with_capacity(segment.len());

    let mut chars = segment.chars().peekable();

    while let Some(ch) = chars.next() {
      if ch == '~' {
        match chars.next() {
          Some('0') | None => decoded.push('~'),
          Some('1') => decoded.push('/'),
          Some(other) => {
            decoded.push('~');
            decoded.push(other);
          }
        }
      } else {
        decoded.push(ch);
      }
    }

    decoded
  }

  fn describe_value(value: &Value) -> String {
    match value {
      Value::Null => "null".to_string(),
      Value::Bool(boolean) => format!("boolean {boolean}"),
      Value::Number(number) => {
        if number.is_i64() || number.is_u64() {
          format!("integer {number}")
        } else {
          format!("number {number}")
        }
      }
      Value::String(string) => format!("string \"{string}\""),
      Value::Array(values) => format!("array of length {}", values.len()),
      Value::Object(_) => "object".to_string(),
    }
  }

  fn dotted_path(pointer: &str) -> String {
    pointer
      .trim_start_matches('/')
      .split('/')
      .filter(|segment| !segment.is_empty())
      .map(Self::decode_segment)
      .collect::<Vec<_>>()
      .join(".")
  }

  fn expected_types(kind: &TypeKind) -> String {
    match kind {
      TypeKind::Single(type_) => type_.to_string(),
      TypeKind::Multiple(types) => {
        let mut names = types
          .iter()
          .map(|type_| type_.to_string())
          .collect::<Vec<_>>();

        names.sort();

        if names.len() <= 1 {
          names.join("")
        } else if names.len() == 2 {
          format!("{} or {}", names[0], names[1])
        } else {
          let last = names.pop().unwrap_or_default();
          format!("{}, or {last}", names.join(", "))
        }
      }
    }
  }

  fn format_enum_options(options: &Value) -> String {
    match options {
      Value::Array(values) => values
        .iter()
        .map(Self::format_literal)
        .collect::<Vec<_>>()
        .join(", "),
      _ => options.to_string(),
    }
  }

  fn format_literal(value: &Value) -> String {
    match value {
      Value::Null => "null".to_string(),
      Value::Bool(boolean) => boolean.to_string(),
      Value::Number(number) => number.to_string(),
      Value::String(string) => format!("\"{string}\""),
      Value::Array(_) | Value::Object(_) => value.to_string(),
    }
  }

  fn format_setting(path: &str) -> String {
    if path.is_empty() {
      "value".to_string()
    } else {
      format!("`{path}`")
    }
  }

  fn format_validation_error(error: &ValidationError) -> String {
    let path = Self::dotted_path(error.instance_path.as_str());

    let target = Self::format_setting(&path);

    let message = match &error.kind {
      ValidationErrorKind::AdditionalItems { limit } => {
        let count = Self::array_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} allows at most {limit} items, found {len}")
          }
          None => format!("{target} allows at most {limit} items"),
        }
      }
      ValidationErrorKind::AdditionalProperties { unexpected } => {
        let setting_path = unexpected
          .first()
          .map(|property| Self::join_path_segments(&path, property))
          .filter(|setting| !setting.is_empty())
          .unwrap_or_else(|| path.clone());

        let setting = Self::format_setting(&setting_path);

        format!("unknown setting {setting}")
      }
      ValidationErrorKind::AnyOf => {
        format!("{target} does not match any allowed schema in anyOf")
      }
      ValidationErrorKind::BacktrackLimitExceeded { error } => {
        format!("regex backtracking limit exceeded: {error}")
      }
      ValidationErrorKind::Constant { expected_value } => {
        format!(
          "{target} must equal {}",
          Self::format_literal(expected_value)
        )
      }
      ValidationErrorKind::Contains => {
        format!("{target} must contain at least one item matching the schema")
      }
      ValidationErrorKind::ContentEncoding { content_encoding } => {
        format!("{target} is not valid {content_encoding} content encoding")
      }
      ValidationErrorKind::ContentMediaType { content_media_type } => {
        format!(
          "{target} is not compliant with media type {content_media_type}"
        )
      }
      ValidationErrorKind::Custom { message } => {
        format!("{target}: {message}")
      }
      ValidationErrorKind::Required { property } => {
        format!(
          "missing required setting {}",
          Self::format_setting(&Self::join_path_segments(
            &path,
            &Self::value_to_property(property)
          ))
        )
      }
      ValidationErrorKind::Type { kind } => {
        format!(
          "expected {} for {target}, got {}",
          Self::expected_types(kind),
          Self::describe_value(error.instance.as_ref())
        )
      }
      ValidationErrorKind::Enum { options } => {
        format!(
          "{target} must be one of: {}",
          Self::format_enum_options(options)
        )
      }
      ValidationErrorKind::ExclusiveMaximum { limit } => {
        format!(
          "expected a value less than {limit} for {target}, got {}",
          Self::describe_value(error.instance.as_ref())
        )
      }
      ValidationErrorKind::ExclusiveMinimum { limit } => {
        format!(
          "expected a value greater than {limit} for {target}, got {}",
          Self::describe_value(error.instance.as_ref())
        )
      }
      ValidationErrorKind::FalseSchema => {
        format!("no values are allowed for {target}")
      }
      ValidationErrorKind::Format { format } => {
        format!("{target} is not a valid {format}")
      }
      ValidationErrorKind::FromUtf8 { error } => {
        format!("invalid utf-8 data for {target}: {error}")
      }
      ValidationErrorKind::MaxItems { limit } => {
        let count = Self::array_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} allows at most {limit} items, found {len}")
          }
          None => format!("{target} allows at most {limit} items"),
        }
      }
      ValidationErrorKind::Maximum { limit } => {
        format!(
          "expected a value no greater than {limit} for {target}, got {}",
          Self::describe_value(error.instance.as_ref())
        )
      }
      ValidationErrorKind::MaxLength { limit } => {
        let length = Self::string_length(error.instance.as_ref());

        match length {
          Some(len) => format!(
            "{target} must be at most {limit} characters long, found {len}"
          ),
          None => format!("{target} must be at most {limit} characters long"),
        }
      }
      ValidationErrorKind::MaxProperties { limit } => {
        let count = Self::object_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} allows at most {limit} properties, found {len}")
          }
          None => format!("{target} allows at most {limit} properties"),
        }
      }
      ValidationErrorKind::MinItems { limit } => {
        let count = Self::array_length(error.instance.as_ref());

        match count {
          Some(len) => {
            format!("{target} must contain at least {limit} items, found {len}")
          }
          None => format!("{target} must contain at least {limit} items"),
        }
      }
      ValidationErrorKind::Minimum { limit } => {
        let actual = Self::describe_value(error.instance.as_ref());

        format!(
          "expected a value no less than {limit} for {target}, got {actual}"
        )
      }
      ValidationErrorKind::MinLength { limit } => {
        let length = Self::string_length(error.instance.as_ref());

        match length {
          Some(len) => format!(
            "{target} must be at least {limit} characters long, found {len}"
          ),
          None => format!("{target} must be at least {limit} characters long"),
        }
      }
      ValidationErrorKind::MinProperties { limit } => {
        let count = Self::object_length(error.instance.as_ref());

        match count {
          Some(len) => format!(
            "{target} must contain at least {limit} properties, found {len}"
          ),
          None => format!("{target} must contain at least {limit} properties"),
        }
      }
      ValidationErrorKind::MultipleOf { multiple_of } => {
        format!(
          "expected a multiple of {multiple_of} for {target}, got {}",
          Self::describe_value(error.instance.as_ref())
        )
      }
      ValidationErrorKind::Not { .. } => {
        format!("{target} must not match the disallowed schema")
      }
      ValidationErrorKind::OneOfMultipleValid => {
        format!("{target} matches multiple schemas in oneOf")
      }
      ValidationErrorKind::OneOfNotValid => {
        format!("{target} does not match any schema in oneOf")
      }
      ValidationErrorKind::Pattern { pattern } => {
        format!("{target} does not match pattern `{pattern}`")
      }
      ValidationErrorKind::PropertyNames { error } => {
        format!(
          "invalid property name in {target}: {}",
          Self::format_validation_error(error)
        )
      }
      ValidationErrorKind::UnevaluatedItems { unexpected }
      | ValidationErrorKind::UnevaluatedProperties { unexpected } => {
        if unexpected.is_empty() {
          format!("unevaluated properties are not allowed in {target}")
        } else {
          let properties = unexpected.join(", ");
          format!(
            "unevaluated properties are not allowed in {target}: {properties}"
          )
        }
      }
      ValidationErrorKind::UniqueItems => {
        format!("items in {target} must be unique")
      }
      ValidationErrorKind::Referencing(error) => {
        format!("schema reference error: {error}")
      }
    };

    Self::lowercase_message(message)
  }

  fn join_path_segments(base: &str, segment: &str) -> String {
    if base.is_empty() {
      segment.to_string()
    } else {
      format!("{base}.{segment}")
    }
  }

  fn lowercase_message(message: String) -> String {
    let mut chars = message.chars();

    if let Some(first) = chars.next() {
      let mut lowered = String::with_capacity(message.len());
      lowered.extend(first.to_lowercase());
      lowered.push_str(chars.as_str());
      lowered
    } else {
      message
    }
  }

  fn object_length(value: &Value) -> Option<usize> {
    value.as_object().map(serde_json::Map::len)
  }

  fn string_length(value: &Value) -> Option<usize> {
    value.as_str().map(|string| string.chars().count())
  }

  fn value_to_property(property: &Value) -> String {
    property
      .as_str()
      .map_or_else(|| property.to_string(), str::to_string)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn message_for_first_error(schema: Value, instance: Value) -> String {
    let schema = jsonschema::options()
      .with_draft(jsonschema::Draft::Draft7)
      .build(&schema)
      .unwrap();

    let error = schema.iter_errors(&instance).next().unwrap();

    JsonSchemaValidationError(&error).to_string()
  }

  #[test]
  fn formats_additional_properties_error() {
    let message = message_for_first_error(
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
      json!({
        "tool": {
          "black": {
            "unknown": true
          }
        }
      }),
    );

    assert_eq!(message, "unknown setting `tool.black.unknown`");
  }

  #[test]
  fn formats_type_mismatch_error() {
    let message = message_for_first_error(
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
      json!({
        "tool": {
          "black": {
            "line-length": "eighty"
          }
        }
      }),
    );

    assert_eq!(
      message,
      "expected integer for `tool.black.line-length`, got string \"eighty\""
    );
  }

  #[test]
  fn formats_enum_error() {
    let message = message_for_first_error(
      json!({
        "type": "object",
        "properties": {
          "color": {
            "type": "string",
            "enum": ["red", "green", "blue"]
          }
        }
      }),
      json!({
        "color": "orange"
      }),
    );

    assert_eq!(
      message,
      "`color` must be one of: \"red\", \"green\", \"blue\""
    );
  }

  #[test]
  fn formats_additional_items_error() {
    let message = message_for_first_error(
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
    let message = message_for_first_error(
      json!({
        "type": "object",
        "properties": {
          "choice": {
            "type": ["string", "integer"]
          }
        }
      }),
      json!({
        "choice": true
      }),
    );

    assert_eq!(
      message,
      "expected integer or string for `choice`, got boolean true"
    );
  }

  #[test]
  fn decodes_pointer_segments_in_paths() {
    let message = message_for_first_error(
      json!({
        "type": "object",
        "properties": {
          "path~to/setting": {
            "type": "integer"
          }
        }
      }),
      json!({
        "path~to/setting": "wrong"
      }),
    );

    assert_eq!(
      message,
      "expected integer for `path~to/setting`, got string \"wrong\""
    );
  }

  #[test]
  fn formats_min_length_error_with_count() {
    let message = message_for_first_error(
      json!({
        "type": "object",
        "properties": {
          "code": {
            "type": "string",
            "minLength": 5
          }
        }
      }),
      json!({
        "code": "abc"
      }),
    );

    assert_eq!(
      message,
      "`code` must be at least 5 characters long, found 3"
    );
  }

  #[test]
  fn formats_unique_items_error() {
    let message = message_for_first_error(
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
      json!({
        "ids": [1, 1]
      }),
    );

    assert_eq!(message, "items in `ids` must be unique");
  }
}
